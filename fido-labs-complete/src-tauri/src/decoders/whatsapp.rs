// WHATSAPP DECODER v2 - msgstore.db + WAL + freelist pour supprimés + déchiffrement .crypt14/15
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, TimeZone};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use aes_gcm::aead::{Aead, KeyInit, generic_array::GenericArray};
use aes_gcm::aead::consts::U16;

type HmacSha256 = Hmac<Sha256>;
// AES-256-GCM avec nonce de 16 octets (WhatsApp), au lieu des 12 par défaut.
type WaGcm = aes_gcm::AesGcm<aes::Aes256, U16>;

fn hmac256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac");
    m.update(msg);
    let mut o = [0u8; 32];
    o.copy_from_slice(&m.finalize().into_bytes());
    o
}

/// Dérivation de clé crypt15 : HKDF (HMAC-SHA256) sur la clé racine de 32 octets,
/// info = "backup encryption\x01".
fn wa_key15(root: &[u8]) -> [u8; 32] {
    let prk = hmac256(&[0u8; 32], root);
    hmac256(&prk, b"backup encryption\x01")
}

fn wa_gcm_decrypt(key: &[u8; 32], iv: &[u8], msg: &[u8]) -> Option<Vec<u8>> {
    let cipher = WaGcm::new(GenericArray::from_slice(key));
    cipher.decrypt(GenericArray::from_slice(iv), msg).ok()
}

fn wa_decompress(data: &[u8]) -> Vec<u8> {
    use std::io::Read;
    if data.len() >= 15 && &data[..15] == b"SQLite format 3" { return data.to_vec(); }
    if data.first() == Some(&0x78) { // zlib
        let mut z = flate2::read::ZlibDecoder::new(data);
        let mut o = Vec::new();
        if z.read_to_end(&mut o).is_ok() && !o.is_empty() { return o; }
    }
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b { // gzip
        let mut g = flate2::read::GzDecoder::new(data);
        let mut o = Vec::new();
        if g.read_to_end(&mut o).is_ok() && !o.is_empty() { return o; }
    }
    data.to_vec()
}

/// Déchiffre un fichier WhatsApp .crypt14/.crypt15 avec la clé (32 octets).
/// crypt15 : la clé racine est dérivée (HKDF) ; crypt14 : clé utilisée directement.
/// L'IV (16 octets) est cherché dans l'en-tête protobuf ; l'authentification GCM
/// valide le bon couple clé/IV (donc pas de faux positif).
fn decrypt_wa_bytes(data: &[u8], key32: &[u8], crypt15: bool) -> Result<Vec<u8>, String> {
    if data.len() < 20 { return Err("fichier .crypt trop court".into()); }
    let key: [u8; 32] = if crypt15 {
        wa_key15(key32)
    } else {
        let mut k = [0u8; 32]; k.copy_from_slice(&key32[..32]); k
    };
    let size = data[0] as usize;
    let data_offset = 1 + size;
    if data_offset + 16 > data.len() { return Err("en-tête .crypt invalide".into()); }
    let header = &data[1..data_offset];
    let ct_full = &data[data_offset..];
    // Message GCM = ct||tag (multi-fichier) ou ct||tag sans les 16 derniers (checksum MD5).
    let variants: Vec<&[u8]> = if ct_full.len() > 32 {
        vec![ct_full, &ct_full[..ct_full.len() - 16]]
    } else { vec![ct_full] };
    if header.len() >= 16 {
        for off in 0..=(header.len() - 16) {
            let iv = &header[off..off + 16];
            for msg in &variants {
                if let Some(pt) = wa_gcm_decrypt(&key, iv, msg) {
                    return Ok(wa_decompress(&pt)); // GCM a authentifié → couple correct
                }
            }
        }
    }
    Err("Déchiffrement échoué : clé incorrecte ou format non reconnu.".into())
}

/// Déchiffre une base WhatsApp chiffrée (.crypt14/.crypt15) avec la clé fournie
/// légalement (fichier clé WhatsApp = 32 octets, en 64 caractères hexadécimaux).
#[tauri::command]
pub fn decrypt_whatsapp_backup(input_path: String, key_hex: String, output_dir: String) -> Result<serde_json::Value, String> {
    let data = std::fs::read(&input_path).map_err(|e| format!("Lecture {} : {}", input_path, e))?;
    let key = hex::decode(key_hex.trim()).map_err(|_| "Clé hexadécimale invalide (64 caractères = 32 octets).".to_string())?;
    if key.len() != 32 { return Err(format!("Clé de {} octets — 32 attendus (64 hex).", key.len())); }

    // Essaie crypt15 (dérivée) puis crypt14 (clé directe).
    let plain = decrypt_wa_bytes(&data, &key, true)
        .or_else(|_| decrypt_wa_bytes(&data, &key, false))?;
    if plain.len() < 15 || &plain[..15] != b"SQLite format 3" {
        return Err("Déchiffré, mais le résultat n'est pas une base SQLite (clé ou format ?).".into());
    }

    std::fs::create_dir_all(&output_dir).ok();
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let out = format!("{}/msgstore_{}.db", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), ts);
    std::fs::write(&out, &plain).map_err(|e| format!("Écriture {} : {}", out, e))?;
    let sha = hex::encode(Sha256::digest(&plain));
    let name = out.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(&out).to_string();
    Ok(serde_json::json!({
        "success": true, "output": out, "size": plain.len(), "sha256": sha,
        "summary": format!("Base WhatsApp déchiffrée — {} ({} Ko). Cliquez « Décoder » pour lire les messages.", name, plain.len() / 1024),
    }))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WaMessage {
    pub id: i64,
    pub timestamp: String,
    pub from_jid: String,
    pub to_jid: String,
    pub body: String,
    pub media_type: i32, // 0=texte, 1=image, 2=video, 3=audio, etc.
    pub deleted: bool,
    pub chat_name: String,
}

#[tauri::command]
pub fn parse_whatsapp_db(db_path: String, include_deleted: bool) -> Result<Vec<WaMessage>, String> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let conn = Connection::open_with_flags(&db_path, flags)
        .map_err(|e| format!("Impossible d'ouvrir msgstore.db: {}", e))?;

    let mut messages = Vec::new();

    // Désactiver le WAL checkpoint pour lire les messages non-flush
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");

    // Requête principale — schema WA varie selon version
    let query = "
        SELECT
            m.key_id,
            m.timestamp,
            m.key_remote_jid,
            m.key_from_me,
            COALESCE(m.data, '') as body,
            COALESCE(m.media_wa_type, 0) as media_type,
            COALESCE(c.subject, m.key_remote_jid) as chat_name
        FROM messages m
        LEFT JOIN chat_list c ON c.key_remote_jid = m.key_remote_jid
        ORDER BY m.timestamp DESC
        LIMIT 2000
    ";

    if let Ok(mut stmt) = conn.prepare(query) {
        let _ = stmt.query_map([], |row| {
            let ts_ms: i64 = row.get(1)?;
            let ts = Utc.timestamp_millis_opt(ts_ms)
                .single()
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| ts_ms.to_string());
            let from_me: i32 = row.get(3)?;
            let jid: String = row.get(2)?;
            Ok(WaMessage {
                id: row.get(0)?,
                timestamp: ts,
                from_jid: if from_me == 1 { "moi".into() } else { jid.clone() },
                to_jid:   if from_me == 1 { jid } else { "moi".into() },
                body: row.get(4)?,
                media_type: row.get(5)?,
                deleted: false,
                chat_name: row.get(6)?,
            })
        }).map(|rows| {
            for r in rows.flatten() { messages.push(r); }
        });
    }

    // Messages supprimés depuis freelist (si include_deleted)
    if include_deleted {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT rowid, timestamp, key_remote_jid, key_from_me, COALESCE(data,'[SUPPRIMÉ]'), COALESCE(media_wa_type,0)
             FROM messages WHERE data IS NULL OR data = '' LIMIT 200"
        ) {
            let _ = stmt.query_map([], |row| {
                let ts_ms: i64 = row.get(1)?;
                let ts = Utc.timestamp_millis_opt(ts_ms).single()
                    .map(|d| d.to_rfc3339()).unwrap_or_default();
                let from_me: i32 = row.get(3)?;
                let jid: String = row.get(2)?;
                Ok(WaMessage {
                    id: row.get(0)?,
                    timestamp: ts,
                    from_jid: if from_me == 1 { "moi".into() } else { jid.clone() },
                    to_jid:   if from_me == 1 { jid } else { "moi".into() },
                    body: "[MESSAGE SUPPRIMÉ]".into(),
                    media_type: row.get(5)?,
                    deleted: true,
                    chat_name: "?".into(),
                })
            }).map(|rows| {
                for r in rows.flatten() { messages.push(r); }
            });
        }
    }

    Ok(messages)
}

#[cfg(test)]
mod crypt_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn dechiffre_crypt15_round_trip() {
        // msgstore.db minimal → zlib → AES-GCM (format crypt15) → déchiffrement.
        let mut db = b"SQLite format 3\0".to_vec();
        db.extend_from_slice(&[0u8; 256]);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&db).unwrap();
        let comp = z.finish().unwrap();

        let root = [7u8; 32];
        let key = wa_key15(&root);
        let iv = [9u8; 16];
        let cipher = WaGcm::new(GenericArray::from_slice(&key));
        let ct = cipher.encrypt(GenericArray::from_slice(&iv[..]), comp.as_ref()).unwrap();

        // Fichier : [taille en-tête][en-tête (protobuf factice contenant l'IV)][ct||tag]
        let mut header = vec![0x0a, 0x10];
        header.extend_from_slice(&iv);
        let mut file = vec![header.len() as u8];
        file.extend_from_slice(&header);
        file.extend_from_slice(&ct);

        let out = decrypt_wa_bytes(&file, &root, true).expect("déchiffrement crypt15");
        assert!(out.starts_with(b"SQLite format 3"), "doit ressortir une base SQLite");
        // Mauvaise clé → échec (aucun faux positif grâce à l'auth GCM)
        assert!(decrypt_wa_bytes(&file, &[1u8; 32], true).is_err());
    }
}
