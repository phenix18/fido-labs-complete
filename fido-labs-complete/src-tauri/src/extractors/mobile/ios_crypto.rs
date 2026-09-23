// Déchiffrement des sauvegardes iOS chiffrées — avec le mot de passe fourni LÉGALEMENT.
// Ce n'est PAS un contournement : on lit une sauvegarde protégée dont on a le mot de passe.
//
// Schéma Apple (iOS 10.2+) :
//   keybag (Manifest.plist/BackupKeyBag) → PBKDF2-SHA256(mdp, DPSL, DPIC) puis
//   PBKDF2-SHA1(., SALT, ITER) = clé de code → déballage AES (RFC 3394) des clés de classe
//   → ManifestKey déballe la clé du Manifest.db (AES-CBC IV nul) → chaque fichier a sa clé
//   (dans Files.file, un plist NSKeyedArchiver) déballée puis AES-CBC IV nul.

use std::collections::HashMap;
use std::io::Cursor;

pub fn pbkdf2_sha1(pw: &[u8], salt: &[u8], rounds: u32, len: usize) -> Vec<u8> {
    let mut o = vec![0u8; len];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(pw, salt, rounds, &mut o);
    o
}
pub fn pbkdf2_sha256(pw: &[u8], salt: &[u8], rounds: u32, len: usize) -> Vec<u8> {
    let mut o = vec![0u8; len];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(pw, salt, rounds, &mut o);
    o
}

/// Déballage de clé AES (RFC 3394) — KEK 256 bits. Valide aussi le mot de passe
/// (l'IV par défaut 0xA6A6… doit ressortir), sinon erreur.
pub fn aes_key_unwrap(kek: &[u8], wrapped: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray};
    if wrapped.len() % 8 != 0 || wrapped.len() < 16 { return Err("clé emballée invalide".into()); }
    let n = wrapped.len() / 8 - 1;
    let mut a = [0u8; 8];
    a.copy_from_slice(&wrapped[..8]);
    let mut r: Vec<[u8; 8]> = (0..n).map(|i| {
        let mut b = [0u8; 8]; b.copy_from_slice(&wrapped[8 * (i + 1)..8 * (i + 2)]); b
    }).collect();
    let cipher = aes::Aes256::new_from_slice(kek).map_err(|_| "KEK invalide".to_string())?;
    for j in (0..6i64).rev() {
        for i in (0..n).rev() {
            let t = (n as u64) * (j as u64) + (i as u64) + 1;
            let mut blk = [0u8; 16];
            for k in 0..8 { blk[k] = a[k] ^ (((t >> (8 * (7 - k))) & 0xff) as u8); }
            blk[8..].copy_from_slice(&r[i]);
            let mut g = GenericArray::clone_from_slice(&blk);
            cipher.decrypt_block(&mut g);
            a.copy_from_slice(&g[..8]);
            r[i].copy_from_slice(&g[8..]);
        }
    }
    if a != [0xa6u8; 8] { return Err("Mot de passe incorrect (déballage AES invalide).".into()); }
    let mut out = Vec::with_capacity(n * 8);
    for b in r { out.extend_from_slice(&b); }
    Ok(out)
}

/// AES-256-CBC à IV nul, sans retrait de bourrage (chiffrement fichier iOS).
pub fn aes_cbc_zero_iv_decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit, generic_array::GenericArray};
    type Dec = cbc::Decryptor<aes::Aes256>;
    if data.is_empty() || data.len() % 16 != 0 { return Err("données non alignées (AES-CBC)".into()); }
    let iv = [0u8; 16];
    let mut dec = Dec::new(key.into(), (&iv).into());
    let mut buf = data.to_vec();
    for chunk in buf.chunks_exact_mut(16) {
        let mut g = GenericArray::clone_from_slice(chunk);
        dec.decrypt_block_mut(&mut g);
        chunk.copy_from_slice(&g);
    }
    Ok(buf)
}

pub struct BackupCrypto {
    pub class_keys: HashMap<u32, Vec<u8>>,
}

/// Analyse le keybag et dérive les clés de classe à partir du mot de passe.
/// Échoue si le mot de passe est incorrect (aucune clé ne se déballe).
pub fn build_crypto(keybag: &[u8], password: &str) -> Result<BackupCrypto, String> {
    let mut i = 0usize;
    let (mut salt, mut dpsl) = (Vec::new(), Vec::new());
    let (mut iter, mut dpic) = (0u32, 0u32);
    let mut classes: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut cur_class: Option<u32> = None;

    while i + 8 <= keybag.len() {
        let tag = [keybag[i], keybag[i + 1], keybag[i + 2], keybag[i + 3]];
        let len = u32::from_be_bytes([keybag[i + 4], keybag[i + 5], keybag[i + 6], keybag[i + 7]]) as usize;
        i += 8;
        if i + len > keybag.len() { break; }
        let val = &keybag[i..i + len];
        i += len;
        let as_u32 = || {
            let mut b = [0u8; 4];
            let s = val.len().min(4);
            b[4 - s..].copy_from_slice(&val[val.len() - s..]);
            u32::from_be_bytes(b)
        };
        match &tag {
            b"SALT" if cur_class.is_none() => salt = val.to_vec(),
            b"ITER" if cur_class.is_none() => iter = as_u32(),
            b"DPSL" => dpsl = val.to_vec(),
            b"DPIC" => dpic = as_u32(),
            b"CLAS" => cur_class = Some(as_u32()),
            b"WPKY" => { if let Some(c) = cur_class { classes.push((c, val.to_vec())); } }
            _ => {}
        }
    }

    // Dérivation du mot de passe (double protection iOS 10.2+ si DPSL/DPIC présents).
    let passcode1 = if !dpsl.is_empty() && dpic > 0 {
        pbkdf2_sha256(password.as_bytes(), &dpsl, dpic, 32)
    } else {
        password.as_bytes().to_vec()
    };
    let passcode_key = pbkdf2_sha1(&passcode1, &salt, iter, 32);

    let mut class_keys = HashMap::new();
    for (c, wpky) in classes {
        if wpky.len() == 40 {
            if let Ok(k) = aes_key_unwrap(&passcode_key, &wpky) { class_keys.insert(c, k); }
        }
    }
    if class_keys.is_empty() {
        return Err("Mot de passe incorrect (aucune clé de classe déballée).".into());
    }
    Ok(BackupCrypto { class_keys })
}

impl BackupCrypto {
    /// Déballe un keyblob (4 octets classe LE + 40 octets clé emballée).
    pub fn unwrap_keyblob(&self, blob: &[u8]) -> Result<Vec<u8>, String> {
        if blob.len() < 44 { return Err("keyblob trop court".into()); }
        let class = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
        let ck = self.class_keys.get(&class).ok_or_else(|| format!("classe {} absente du keybag", class))?;
        aes_key_unwrap(ck, &blob[4..44])
    }

    pub fn decrypt_manifest_db(&self, manifest_key: &[u8], enc: &[u8]) -> Result<Vec<u8>, String> {
        let key = self.unwrap_keyblob(manifest_key)?;
        aes_cbc_zero_iv_decrypt(&key, enc)
    }

    /// Déchiffre un fichier de sauvegarde à partir de son blob `Files.file` (NSKeyedArchiver).
    pub fn decrypt_file(&self, file_blob: &[u8], enc: &[u8]) -> Result<Vec<u8>, String> {
        let (enc_key, size) = parse_mbfile(file_blob)?;
        let key = self.unwrap_keyblob(&enc_key)?;
        let mut plain = aes_cbc_zero_iv_decrypt(&key, enc)?;
        if let Some(sz) = size { if (sz as usize) <= plain.len() { plain.truncate(sz as usize); } }
        Ok(plain)
    }
}

/// Extrait (keyblob de chiffrement, taille) d'un blob `Files.file` NSKeyedArchiver.
fn parse_mbfile(blob: &[u8]) -> Result<(Vec<u8>, Option<u64>), String> {
    let v = plist::Value::from_reader(Cursor::new(blob)).map_err(|e| format!("plist Files.file : {}", e))?;
    let d = v.as_dictionary().ok_or("blob non-dictionnaire")?;
    let objs = d.get("$objects").and_then(|x| x.as_array()).ok_or("pas de $objects")?;
    let mb = objs.get(1).and_then(|x| x.as_dictionary()).ok_or("MBFile absent")?;
    let size = mb.get("Size").and_then(|x| x.as_unsigned_integer());
    let uid = mb.get("EncryptionKey").and_then(|x| x.as_uid()).map(|u| u.get() as usize)
        .ok_or("EncryptionKey absente")?;
    let ek = objs.get(uid).and_then(|x| x.as_dictionary()).ok_or("objet clé absent")?;
    let data = ek.get("NS.data").and_then(|x| x.as_data()).ok_or("NS.data absente")?;
    Ok((data.to_vec(), size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_key_unwrap_vecteur_nist() {
        // RFC 3394 — 256-bit KEK, 256-bit key data (vecteur officiel NIST).
        let kek = hex::decode("000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F").unwrap();
        let wrapped = hex::decode("28C9F404C4B810F4CBCCB35CFB87F8263F5786E2D80ED326CBC7F0E71A99F43BFB988B9B7A02DD21").unwrap();
        let expect = hex::decode("00112233445566778899AABBCCDDEEFF000102030405060708090A0B0C0D0E0F").unwrap();
        let out = aes_key_unwrap(&kek, &wrapped).expect("unwrap");
        assert_eq!(out, expect, "AES key unwrap doit correspondre au vecteur NIST");
        // Mauvaise KEK → IV invalide → erreur (pas de contournement)
        let mut bad = kek.clone(); bad[0] ^= 0xFF;
        assert!(aes_key_unwrap(&bad, &wrapped).is_err());
    }
}
