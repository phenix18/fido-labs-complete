// PARSEUR DE SAUVEGARDE ANDROID (.ab) — format `adb backup`.
// Un .ab = en-tête ASCII de 4 lignes puis une charge utile :
//   ANDROID BACKUP\n <version>\n <compressé 0|1>\n <chiffrement none|AES-256>\n <payload>
// payload = archive TAR, compressée en zlib si le drapeau vaut 1.
// On déballe l'archive, on écrit chaque fichier, on le hache (SHA256) et on repère les
// bases SQLite. Sauvegarde chiffrée → signalée, JAMAIS déchiffrée sans mot de passe légal.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;

fn sha256_bytes(b: &[u8]) -> String { hex::encode(Sha256::digest(b)) }

/// Nettoie un chemin d'archive (empêche la traversée « ../ » à l'extraction).
fn sanitize_rel(p: &str) -> String {
    p.split(|c| c == '/' || c == '\\')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/")
}

// ─── Déchiffrement AES-256 d'un `adb backup` protégé par mot de passe ─────────
// Schéma documenté (AOSP) : PBKDF2-HMAC-SHA1(mot de passe) → clé utilisateur → déchiffre
// le blob de clé maître (AES-CBC) → clé maître + IV → déchiffre la charge (AES-CBC) →
// zlib → tar. Le mot de passe est fourni par l'enquêteur (voie légale) : ce n'est pas un
// contournement, c'est la lecture autorisée d'une sauvegarde protégée.
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

fn pbkdf2_sha1(pw: &[u8], salt: &[u8], rounds: u32, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(pw, salt, rounds, &mut out);
    out
}

fn aes256_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 || iv.len() != 16 { return Err("clé/IV de taille invalide".into()); }
    if data.is_empty() || data.len() % 16 != 0 { return Err("données AES non alignées".into()); }
    let mut buf = data.to_vec();
    let pt = Aes256CbcDec::new(key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| "déchiffrement AES échoué (mot de passe incorrect ?)".to_string())?;
    Ok(pt.to_vec())
}

/// Déchiffre la charge utile d'un .ab AES-256 avec le mot de passe fourni.
/// Renvoie les octets en clair (= zlib(tar) si compressé, sinon tar).
fn decrypt_adb_backup(password: &str, user_salt: &str, _ck_salt: &str, rounds: u32,
                      user_iv: &str, mk_blob: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
    let usalt = hex::decode(user_salt.trim()).map_err(|_| "sel utilisateur (hex) invalide".to_string())?;
    let uiv = hex::decode(user_iv.trim()).map_err(|_| "IV utilisateur (hex) invalide".to_string())?;
    let blob_enc = hex::decode(mk_blob.trim()).map_err(|_| "blob de clé maître (hex) invalide".to_string())?;

    let user_key = pbkdf2_sha1(password.as_bytes(), &usalt, rounds, 32);
    let blob = aes256_cbc_decrypt(&user_key, &uiv, &blob_enc)
        .map_err(|_| "Mot de passe incorrect (déchiffrement du blob de clé échoué).".to_string())?;

    // blob = [n]<masterIV>[n]<masterKey>[n]<checksum>
    let mut i = 0usize;
    let mut take = || -> Result<Vec<u8>, String> {
        if i >= blob.len() { return Err("blob de clé tronqué".into()); }
        let n = blob[i] as usize; i += 1;
        if i + n > blob.len() { return Err("blob de clé tronqué".into()); }
        let v = blob[i..i + n].to_vec(); i += n; Ok(v)
    };
    let master_iv = take()?;
    let master_key = take()?;
    let _checksum = take().unwrap_or_default(); // vérif. de somme best-effort (bug de signe Android connu)

    aes256_cbc_decrypt(&master_key, &master_iv, payload)
        .map_err(|_| "Charge utile indéchiffrable (mot de passe ou format).".to_string())
}

/// Déballe et inventorie une sauvegarde `adb backup` (.ab). `password` déchiffre les
/// sauvegardes AES-256 (mot de passe fourni légalement — pas un contournement).
#[tauri::command]
pub fn parse_android_backup(case_id: String, ab_path: String, output_dir: String, password: Option<String>) -> Result<serde_json::Value, String> {
    let data = std::fs::read(&ab_path).map_err(|e| format!("Lecture {}: {}", ab_path, e))?;

    // ── En-tête ASCII (4 lignes en clair ; 9 lignes si chiffré AES-256) ──
    let mut nl = Vec::new();
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' { nl.push(i); if nl.len() >= 9 { break; } }
    }
    if nl.len() < 4 { return Err("Fichier .ab invalide (en-tête incomplet).".into()); }
    let line = |a: usize, b: usize| String::from_utf8_lossy(&data[a..b]).trim().to_string();
    let magic = line(0, nl[0]);
    if magic != "ANDROID BACKUP" {
        return Err(format!("Signature inattendue : « {} » (attendu « ANDROID BACKUP »).", magic));
    }
    let version = line(nl[0] + 1, nl[1]);
    let compressed = line(nl[1] + 1, nl[2]) == "1";
    let encryption = line(nl[2] + 1, nl[3]);
    let is_encrypted = !encryption.is_empty() && encryption.to_lowercase() != "none";
    let pw = password.unwrap_or_default();

    // Charge utile EN CLAIR (déchiffrée si besoin) = zlib(tar) ou tar.
    let plain_payload: Vec<u8> = if is_encrypted {
        if pw.trim().is_empty() {
            return Ok(serde_json::json!({
                "success": false, "encrypted": true, "needs_password": true,
                "version": version, "encryption": encryption,
                "summary": format!("Sauvegarde Android CHIFFRÉE ({}) — saisissez le mot de passe (légal) pour déchiffrer.", encryption),
            }));
        }
        if nl.len() < 9 { return Err("En-tête chiffré incomplet (9 lignes attendues).".into()); }
        let user_salt = line(nl[3] + 1, nl[4]);
        let ck_salt   = line(nl[4] + 1, nl[5]);
        let rounds: u32 = line(nl[5] + 1, nl[6]).parse().map_err(|_| "Nombre de tours PBKDF2 invalide.".to_string())?;
        let user_iv   = line(nl[6] + 1, nl[7]);
        let mk_blob   = line(nl[7] + 1, nl[8]);
        let payload = &data[nl[8] + 1..];
        decrypt_adb_backup(&pw, &user_salt, &ck_salt, rounds, &user_iv, &mk_blob, payload)?
    } else {
        data[nl[3] + 1..].to_vec()
    };

    // ── Décompression zlib → TAR ──
    let tar_bytes: Vec<u8> = if compressed {
        let mut z = flate2::read::ZlibDecoder::new(&plain_payload[..]);
        let mut out = Vec::new();
        z.read_to_end(&mut out).map_err(|e| format!("Décompression zlib échouée : {}", e))?;
        out
    } else {
        plain_payload
    };

    // ── Dossier de sortie ──
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/adbbackup_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), sanitize_rel(&case_id), ts);
    let files_dir = format!("{}/files", base);
    std::fs::create_dir_all(&files_dir).map_err(|e| format!("Création {}: {}", files_dir, e))?;

    // ── Extraction du TAR + hachage ──
    let mut archive = tar::Archive::new(std::io::Cursor::new(&tar_bytes));
    let mut apps: std::collections::BTreeMap<String, (usize, u64)> = std::collections::BTreeMap::new();
    let mut databases: Vec<serde_json::Value> = Vec::new();
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;

    let entries = archive.entries().map_err(|e| format!("Archive tar illisible : {}", e))?;
    for entry in entries {
        let mut entry = match entry { Ok(e) => e, Err(_) => continue };
        if entry.header().entry_type().is_dir() { continue; }
        let raw_path = entry.path().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let rel = sanitize_rel(&raw_path);
        if rel.is_empty() { continue; }

        let mut buf = Vec::new();
        if entry.read_to_end(&mut buf).is_err() { continue; }

        let dst = PathBuf::from(&files_dir).join(&rel);
        if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent).ok(); }
        std::fs::write(&dst, &buf).ok();

        let size = buf.len() as u64;
        total_bytes += size;
        file_count += 1;

        // Regroupement par application : "apps/<pkg>/..." ; "shared/..." = /sdcard
        let comps: Vec<&str> = rel.split('/').collect();
        let app = if comps.len() >= 2 && comps[0] == "apps" {
            comps[1].to_string()
        } else if comps.first() == Some(&"shared") {
            "shared (/sdcard)".to_string()
        } else {
            comps.first().copied().unwrap_or("").to_string()
        };
        let e = apps.entry(app).or_insert((0, 0));
        e.0 += 1;
        e.1 += size;

        // Détection des bases SQLite (magie « SQLite format 3 »)
        if buf.len() >= 16 && &buf[..15] == b"SQLite format 3" {
            databases.push(serde_json::json!({ "path": rel, "size": size, "sha256": sha256_bytes(&buf) }));
        }
    }

    let apps_json: Vec<serde_json::Value> = apps.iter()
        .map(|(k, (n, s))| serde_json::json!({ "package": k, "files": n, "size": s }))
        .collect();
    // Chaque base SQLite devient un artefact haché (repris par le rapport PDF).
    let artifacts: Vec<serde_json::Value> = databases.iter()
        .map(|d| serde_json::json!({
            "name": d["path"], "kind": "db", "count": 0, "size": d["size"], "sha256": d["sha256"], "status": "OK"
        }))
        .collect();

    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3", "extraction_type": "ANDROID_ADB_BACKUP", "case_id": case_id,
        "timestamp": chrono::Utc::now().to_rfc3339(), "output_dir": base,
        "version": version, "compressed": compressed,
        "file_count": file_count, "total_bytes": total_bytes,
        "apps": apps_json, "databases": databases,
    });
    let mbytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
    std::fs::write(format!("{}/manifest.json", base), &mbytes).ok();
    let manifest_sha256 = sha256_bytes(&mbytes);

    Ok(serde_json::json!({
        "success": true, "encrypted": is_encrypted, "decrypted": is_encrypted,
        "summary": format!(
            "{}{} fichiers, {} app(s), {} base(s) SQLite ({:.1} Mo)",
            if is_encrypted { "Sauvegarde Android déchiffrée & déballée — " } else { "Sauvegarde Android déballée — " },
            file_count, apps.len(), databases.len(), total_bytes as f64 / 1e6
        ),
        "version": version, "compressed": compressed,
        "output_dir": base, "file_count": file_count, "total_bytes": total_bytes,
        "apps": apps_json, "databases": databases,
        "artifacts": artifacts, "manifest_sha256": manifest_sha256,
    }))
}

// ─── DÉCODAGE PROFOND D'UNE BASE TROUVÉE (.ab ou dump) ───────────────────────
// Identifie le schéma d'une base SQLite (WhatsApp ancien/récent, SMS Android, sinon
// générique) et renvoie les messages en clair. Usage : bouton « Décoder » sur une base
// listée après le déballage d'un .ab, ou sur toute base issue d'un dump physique.

fn ms_to_str(ms: i64) -> String {
    use chrono::TimeZone;
    chrono::Utc.timestamp_millis_opt(ms).single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}

fn list_tables(conn: &rusqlite::Connection) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(mut s) = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'") {
        if let Ok(rows) = s.query_map([], |r| r.get::<_, String>(0)) {
            for r in rows.flatten() { v.push(r); }
        }
    }
    v
}

fn column_exists(conn: &rusqlite::Connection, table: &str, col: &str) -> bool {
    if let Ok(mut s) = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table)) {
        if let Ok(rows) = s.query_map([], |r| r.get::<_, String>(1)) {
            return rows.flatten().any(|c| c == col);
        }
    }
    false
}

fn decode_whatsapp_old(conn: &rusqlite::Connection) -> Vec<serde_json::Value> {
    let q = "SELECT m.timestamp, m.key_from_me, m.key_remote_jid, COALESCE(m.data,''), \
             COALESCE(c.subject, m.key_remote_jid) \
             FROM messages m LEFT JOIN chat_list c ON c.key_remote_jid = m.key_remote_jid \
             WHERE m.data IS NOT NULL AND m.data <> '' ORDER BY m.timestamp DESC LIMIT 3000";
    let mut out = Vec::new();
    if let Ok(mut s) = conn.prepare(q) {
        if let Ok(rows) = s.query_map([], |r| {
            let ts: i64 = r.get(0).unwrap_or(0);
            let from_me: i64 = r.get(1).unwrap_or(0);
            let jid: String = r.get(2).unwrap_or_default();
            Ok(serde_json::json!({
                "date": ms_to_str(ts),
                "from": if from_me == 1 { "moi".into() } else { jid.clone() },
                "to": if from_me == 1 { jid } else { "moi".into() },
                "direction": if from_me == 1 { "Envoyé" } else { "Reçu" },
                "body": r.get::<_, String>(3).unwrap_or_default(),
                "chat": r.get::<_, String>(4).unwrap_or_default(),
            }))
        }) { for r in rows.flatten() { out.push(r); } }
    }
    out
}

fn decode_whatsapp_new(conn: &rusqlite::Connection) -> Vec<serde_json::Value> {
    let q = "SELECT m.timestamp, m.from_me, COALESCE(m.text_data,''), COALESCE(j.raw_string,'') \
             FROM message m LEFT JOIN chat c ON m.chat_row_id = c._id \
             LEFT JOIN jid j ON c.jid_row_id = j._id \
             WHERE m.text_data IS NOT NULL AND m.text_data <> '' ORDER BY m.timestamp DESC LIMIT 3000";
    let mut out = Vec::new();
    if let Ok(mut s) = conn.prepare(q) {
        if let Ok(rows) = s.query_map([], |r| {
            let ts: i64 = r.get(0).unwrap_or(0);
            let from_me: i64 = r.get(1).unwrap_or(0);
            let jid: String = r.get(3).unwrap_or_default();
            Ok(serde_json::json!({
                "date": ms_to_str(ts),
                "from": if from_me == 1 { "moi".into() } else { jid.clone() },
                "to": if from_me == 1 { jid } else { "moi".into() },
                "direction": if from_me == 1 { "Envoyé" } else { "Reçu" },
                "body": r.get::<_, String>(2).unwrap_or_default(),
                "chat": jid_short(&r.get::<_, String>(3).unwrap_or_default()),
            }))
        }) { for r in rows.flatten() { out.push(r); } }
    }
    out
}

fn jid_short(raw: &str) -> String { raw.split('@').next().unwrap_or(raw).to_string() }

fn decode_android_sms(conn: &rusqlite::Connection) -> Vec<serde_json::Value> {
    let q = "SELECT date, address, COALESCE(body,''), type FROM sms ORDER BY date DESC LIMIT 5000";
    let mut out = Vec::new();
    if let Ok(mut s) = conn.prepare(q) {
        if let Ok(rows) = s.query_map([], |r| {
            let ts: i64 = r.get(0).unwrap_or(0);
            let addr: String = r.get(1).unwrap_or_default();
            let typ: i64 = r.get(3).unwrap_or(0);
            let received = typ == 1;
            Ok(serde_json::json!({
                "date": ms_to_str(ts),
                "from": if received { addr.clone() } else { "moi".into() },
                "to": if received { "moi".into() } else { addr.clone() },
                "direction": if received { "Reçu" } else { "Envoyé" },
                "body": r.get::<_, String>(2).unwrap_or_default(),
                "chat": addr,
            }))
        }) { for r in rows.flatten() { out.push(r); } }
    }
    out
}

/// Décode une base SQLite trouvée (WhatsApp / SMS) en messages lisibles ; sinon
/// renvoie un résumé des tables. Lecture SEULE (la preuve n'est pas modifiée).
#[tauri::command]
pub fn decode_database(db_path: String) -> Result<serde_json::Value, String> {
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ).map_err(|e| format!("Ouverture {} : {}", db_path, e))?;

    let tables = list_tables(&conn);
    let has = |t: &str| tables.iter().any(|x| x == t);

    if has("messages") && column_exists(&conn, "messages", "key_remote_jid") {
        let m = decode_whatsapp_old(&conn);
        return Ok(serde_json::json!({ "type": "WhatsApp (msgstore)", "kind": "whatsapp", "count": m.len(), "messages": m }));
    }
    if has("message") && has("jid") {
        let m = decode_whatsapp_new(&conn);
        return Ok(serde_json::json!({ "type": "WhatsApp (msgstore récent)", "kind": "whatsapp", "count": m.len(), "messages": m }));
    }
    if has("sms") {
        let m = decode_android_sms(&conn);
        return Ok(serde_json::json!({ "type": "SMS Android (mmssms)", "kind": "sms", "count": m.len(), "messages": m }));
    }

    // Base inconnue : résumé des tables + nb de lignes.
    let mut summary = Vec::new();
    for t in &tables {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM \"{}\"", t), [], |r| r.get(0)).unwrap_or(-1);
        summary.push(serde_json::json!({ "table": t, "rows": n }));
    }
    Ok(serde_json::json!({
        "type": "Base SQLite (schéma non reconnu)", "kind": "generic",
        "count": 0, "messages": [], "tables": summary,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn deballe_une_sauvegarde_ab() {
        // Construit un TAR contenant une "base SQLite", le compresse en zlib,
        // ajoute l'en-tête .ab, puis vérifie le déballage + hachage + détection SQLite.
        let mut content = b"SQLite format 3\0".to_vec();
        content.extend_from_slice(&[0u8; 200]);

        let mut tarbuf = Vec::new();
        {
            let mut b = tar::Builder::new(&mut tarbuf);
            let mut h = tar::Header::new_gnu();
            h.set_size(content.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, "apps/com.test.app/db/data.db", &content[..]).unwrap();
            b.finish().unwrap();
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&tarbuf).unwrap();
        let comp = z.finish().unwrap();

        let mut ab = b"ANDROID BACKUP\n5\n1\nnone\n".to_vec();
        ab.extend_from_slice(&comp);

        let dir = std::env::temp_dir().join(format!("fido_ab_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let abp = dir.join("backup.ab");
        std::fs::write(&abp, &ab).unwrap();

        let r = parse_android_backup(
            "CASE-AB".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(), None,
        ).expect("parse .ab");
        assert_eq!(r["success"], true);
        assert_eq!(r["encrypted"], false);
        assert_eq!(r["file_count"], 1);
        let dbs = r["databases"].as_array().unwrap();
        assert_eq!(dbs.len(), 1, "1 base SQLite détectée");
        assert_eq!(dbs[0]["sha256"].as_str().unwrap().len(), 64);
        assert!(r["apps"].as_array().unwrap().iter().any(|a| a["package"] == "com.test.app"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decode_whatsapp_et_sms() {
        let dir = std::env::temp_dir().join(format!("fido_decode_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // WhatsApp ancien schéma (messages + chat_list)
        let wa = dir.join("msgstore.db");
        {
            let c = rusqlite::Connection::open(&wa).unwrap();
            c.execute_batch(
                "CREATE TABLE chat_list(key_remote_jid TEXT, subject TEXT);
                 CREATE TABLE messages(key_id TEXT, timestamp INTEGER, key_remote_jid TEXT, key_from_me INTEGER, data TEXT, media_wa_type INTEGER);
                 INSERT INTO chat_list VALUES('22670123456@s.whatsapp.net','Moussa');
                 INSERT INTO messages VALUES('k1',1710490991000,'22670123456@s.whatsapp.net',0,'Ok je transfere',0);
                 INSERT INTO messages VALUES('k2',1710491001000,'22670123456@s.whatsapp.net',1,'Recu merci',0);"
            ).unwrap();
        }
        let r = decode_database(wa.to_string_lossy().to_string()).expect("decode wa");
        assert_eq!(r["kind"], "whatsapp");
        assert_eq!(r["count"], 2);
        assert!(r["messages"][0]["body"].as_str().unwrap().len() > 0);

        // SMS Android (mmssms → table sms)
        let sms = dir.join("mmssms.db");
        {
            let c = rusqlite::Connection::open(&sms).unwrap();
            c.execute_batch(
                "CREATE TABLE sms(_id INTEGER PRIMARY KEY, address TEXT, body TEXT, date INTEGER, type INTEGER);
                 INSERT INTO sms(address,body,date,type) VALUES('+22670123456','Rendez-vous 20h',1710490800000,1);"
            ).unwrap();
        }
        let r2 = decode_database(sms.to_string_lossy().to_string()).expect("decode sms");
        assert_eq!(r2["kind"], "sms");
        assert_eq!(r2["count"], 1);
        assert_eq!(r2["messages"][0]["direction"], "Reçu");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn signale_sauvegarde_chiffree_sans_contournement() {
        let dir = std::env::temp_dir().join(format!("fido_abenc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let abp = dir.join("enc.ab");
        // En-tête chiffré AES-256 (payload factice) : doit être signalé, pas déchiffré.
        std::fs::write(&abp, b"ANDROID BACKUP\n5\n1\nAES-256\nXXXX").unwrap();
        let r = parse_android_backup("C".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(), None)
            .expect("header parse");
        assert_eq!(r["encrypted"], true);
        assert_eq!(r["success"], false);
        assert_eq!(r["needs_password"], true);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dechiffre_un_ab_protege_par_mot_de_passe() {
        use cbc::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
        type Enc = cbc::Encryptor<aes::Aes256>;
        fn aes_enc(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
            let mut buf = vec![0u8; data.len() + 16];
            buf[..data.len()].copy_from_slice(data);
            Enc::new(key.into(), iv.into()).encrypt_padded_mut::<Pkcs7>(&mut buf, data.len()).unwrap().to_vec()
        }

        // 1) TAR (avec une base SQLite) → zlib
        let mut content = b"SQLite format 3\0".to_vec(); content.extend_from_slice(&[7u8; 64]);
        let mut tarbuf = Vec::new();
        { let mut b = tar::Builder::new(&mut tarbuf); let mut h = tar::Header::new_gnu();
          h.set_size(content.len() as u64); h.set_mode(0o644); h.set_cksum();
          b.append_data(&mut h, "apps/com.secret.app/db/secret.db", &content[..]).unwrap(); b.finish().unwrap(); }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&tarbuf).unwrap(); let payload_plain = z.finish().unwrap();

        // 2) Paramètres crypto
        let pw = "MotDePasse#2026";
        let user_salt = [0x11u8; 64]; let ck_salt = [0x22u8; 64]; let rounds = 10_000u32;
        let user_iv = [0x33u8; 16]; let master_key = [0x44u8; 32]; let master_iv = [0x55u8; 16];
        let user_key = pbkdf2_sha1(pw.as_bytes(), &user_salt, rounds, 32);

        // 3) blob = [16]IV [32]MK [4]checksum(factice) puis chiffré avec la clé utilisateur
        let mut blob = Vec::new();
        blob.push(16); blob.extend_from_slice(&master_iv);
        blob.push(32); blob.extend_from_slice(&master_key);
        blob.push(4);  blob.extend_from_slice(&[0u8; 4]);
        let mk_blob_enc = aes_enc(&user_key, &user_iv, &blob);
        let payload_enc = aes_enc(&master_key, &master_iv, &payload_plain);

        // 4) Fichier .ab chiffré (en-tête 9 lignes)
        let mut ab = Vec::new();
        ab.extend_from_slice(format!(
            "ANDROID BACKUP\n5\n1\nAES-256\n{}\n{}\n{}\n{}\n{}\n",
            hex::encode_upper(user_salt), hex::encode_upper(ck_salt), rounds,
            hex::encode_upper(user_iv), hex::encode_upper(&mk_blob_enc)
        ).as_bytes());
        ab.extend_from_slice(&payload_enc);

        let dir = std::env::temp_dir().join(format!("fido_abenc2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let abp = dir.join("enc.ab"); std::fs::write(&abp, &ab).unwrap();

        // Sans mot de passe → demande le mot de passe
        let r0 = parse_android_backup("C".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(), None).unwrap();
        assert_eq!(r0["needs_password"], true);

        // Avec le bon mot de passe → déchiffré + déballé
        let r = parse_android_backup("C".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(), Some(pw.into())).unwrap();
        assert_eq!(r["success"], true, "déchiffrement OK");
        assert_eq!(r["decrypted"], true);
        assert_eq!(r["file_count"], 1);
        assert_eq!(r["databases"].as_array().unwrap().len(), 1);

        // Mauvais mot de passe → erreur (pas de contournement)
        let bad = parse_android_backup("C".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(), Some("faux".into()));
        assert!(bad.is_err(), "mauvais mot de passe doit échouer");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
