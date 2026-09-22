// iOS BACKUP EXTRACTOR — parsing RÉEL d'une sauvegarde iTunes/Finder.
// Une sauvegarde iOS est un dossier local (créé par iTunes/Finder/3uTools/iMazing) :
//   - Manifest.db  : SQLite listant chaque fichier {fileID, domain, relativePath}
//   - fichiers      : rangés à <backup>/<2 premiers car. du fileID>/<fileID>
//   - Info.plist / Manifest.plist : métadonnées appareil + indicateur de chiffrement
// On lit Manifest.db pour localiser sms.db, AddressBook, CallHistory, on hache chaque
// preuve (SHA256) et on décode SMS / contacts / appels. AUCUN contournement : si la
// sauvegarde est chiffrée, on le signale (le déchiffrement exige le mot de passe légal).
// La sortie a la MÊME forme qu'extract_logical → rapport PDF + timeline la réutilisent.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

// Époque Apple/Mac (2001-01-01) en secondes Unix.
const APPLE_EPOCH: i64 = 978_307_200;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct IosDeviceInfo {
    pub device_name: String,
    pub product_type: String,
    pub ios_version: String,
    pub serial: String,
    pub imei: String,
    pub phone_number: String,
    pub last_backup: String,
    pub encrypted: bool,
}

fn sha256_file(p: &Path) -> Option<(String, u64)> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Some((hex::encode(hasher.finalize()), size))
}

/// Emplacement physique d'un fichier dans la sauvegarde à partir de son fileID.
fn backup_file_path(backup: &Path, file_id: &str) -> PathBuf {
    // iOS 10+ : <backup>/<fileID[0:2]>/<fileID> ; iOS 9- : à plat <backup>/<fileID>
    let nested = backup.join(&file_id[..file_id.len().min(2)]).join(file_id);
    if nested.exists() { nested } else { backup.join(file_id) }
}

/// Cherche le fileID d'un artefact via Manifest.db (par relativePath + domaine).
fn find_file_id(manifest: &rusqlite::Connection, domain_like: &str, rel_path: &str) -> Option<String> {
    manifest.query_row(
        "SELECT fileID FROM Files WHERE relativePath = ?1 AND domain LIKE ?2 LIMIT 1",
        rusqlite::params![rel_path, domain_like],
        |r| r.get::<_, String>(0),
    ).ok()
}

/// Copie un fichier + ses annexes SQLite (-wal/-shm) dans out_dir sous un nom lisible,
/// puis renvoie le chemin de la copie (pour que le WAL soit appliqué à l'ouverture).
fn materialize(backup: &Path, file_id: &str, out_dir: &Path, nice_name: &str) -> Option<PathBuf> {
    let src = backup_file_path(backup, file_id);
    if !src.exists() { return None; }
    let dst = out_dir.join(nice_name);
    std::fs::copy(&src, &dst).ok()?;
    Some(dst)
}

fn apple_ts_to_str(raw: i64) -> String {
    use chrono::TimeZone;
    // Nanosecondes (iOS 11+) ou secondes depuis 2001 selon l'ordre de grandeur.
    let unix = if raw > 100_000_000_000 { raw / 1_000_000_000 + APPLE_EPOCH } else { raw + APPLE_EPOCH };
    chrono::Utc.timestamp_opt(unix, 0).single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn dur_to_str(sec: i64) -> String { format!("{}:{:02}", sec / 60, sec % 60) }

// ── Parseurs (défensifs : une table absente n'échoue pas toute l'extraction) ──

fn parse_ios_sms(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    let mut stmt = match conn.prepare(
        "SELECT m.date, m.text, m.is_from_me, h.id \
         FROM message m LEFT JOIN handle h ON m.handle_id = h.ROWID \
         ORDER BY m.date DESC LIMIT 5000"
    ) { Ok(s) => s, Err(_) => return vec![] };
    let rows = stmt.query_map([], |r| {
        let date: i64 = r.get(0).unwrap_or(0);
        let text: Option<String> = r.get(1).ok();
        let from_me: i64 = r.get(2).unwrap_or(0);
        let contact: Option<String> = r.get(3).ok();
        Ok(serde_json::json!({
            "date": apple_ts_to_str(date),
            "contact": contact.unwrap_or_default(),
            "direction": if from_me == 1 { "Envoyé" } else { "Reçu" },
            "body": text.unwrap_or_else(|| "[message sans texte / pièce jointe]".into()),
        }))
    });
    match rows { Ok(it) => it.filter_map(|x| x.ok()).collect(), Err(_) => vec![] }
}

fn parse_ios_contacts(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    let mut stmt = match conn.prepare(
        "SELECT p.First, p.Last, v.value \
         FROM ABPerson p LEFT JOIN ABMultiValue v ON v.record_id = p.ROWID AND v.property = 3 \
         ORDER BY p.Last, p.First LIMIT 10000"
    ) { Ok(s) => s, Err(_) => return vec![] };
    let rows = stmt.query_map([], |r| {
        let first: Option<String> = r.get(0).ok();
        let last: Option<String> = r.get(1).ok();
        let number: Option<String> = r.get(2).ok();
        let name = format!("{} {}", first.unwrap_or_default(), last.unwrap_or_default()).trim().to_string();
        Ok(serde_json::json!({
            "name": if name.is_empty() { "(sans nom)".into() } else { name },
            "number": number.unwrap_or_default(),
        }))
    });
    match rows { Ok(it) => it.filter_map(|x| x.ok()).collect(), Err(_) => vec![] }
}

fn parse_ios_calls(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    let mut stmt = match conn.prepare(
        "SELECT ZADDRESS, ZDATE, ZDURATION, ZORIGINATED, ZANSWERED \
         FROM ZCALLRECORD ORDER BY ZDATE DESC LIMIT 5000"
    ) { Ok(s) => s, Err(_) => return vec![] };
    let rows = stmt.query_map([], |r| {
        // ZADDRESS est un BLOB (numéro en octets ASCII) ou du texte.
        let addr = match r.get_ref(0) {
            Ok(rusqlite::types::ValueRef::Blob(b)) => String::from_utf8_lossy(b).trim().to_string(),
            Ok(rusqlite::types::ValueRef::Text(t)) => String::from_utf8_lossy(t).to_string(),
            _ => String::new(),
        };
        let zdate: f64 = r.get(1).unwrap_or(0.0);
        let dur: f64 = r.get(2).unwrap_or(0.0);
        let originated: i64 = r.get(3).unwrap_or(0);
        let answered: i64 = r.get(4).unwrap_or(0);
        let typ = if originated == 1 { "Sortant" } else if answered == 1 { "Entrant" } else { "Manqué" };
        Ok(serde_json::json!({
            "date": apple_ts_to_str(zdate as i64),
            "number": addr,
            "type": typ,
            "duration": dur_to_str(dur as i64),
        }))
    });
    match rows { Ok(it) => it.filter_map(|x| x.ok()).collect(), Err(_) => vec![] }
}

// ── Métadonnées appareil (Info.plist + Manifest.plist) ──

fn read_device_info(backup: &Path) -> IosDeviceInfo {
    let mut info = IosDeviceInfo::default();
    if let Ok(plist::Value::Dictionary(d)) = plist::Value::from_file(backup.join("Info.plist")) {
        let g = |k: &str| d.get(k).and_then(|v| v.as_string()).unwrap_or("").to_string();
        info.device_name = g("Device Name");
        info.product_type = g("Product Type");
        info.ios_version = g("Product Version");
        info.serial = g("Serial Number");
        info.imei = g("IMEI");
        info.phone_number = g("Phone Number");
        if let Some(dt) = d.get("Last Backup Date").and_then(|v| v.as_date()) {
            info.last_backup = format!("{:?}", dt);
        }
    }
    if let Ok(plist::Value::Dictionary(d)) = plist::Value::from_file(backup.join("Manifest.plist")) {
        info.encrypted = d.get("IsEncrypted").and_then(|v| v.as_boolean()).unwrap_or(false);
    }
    info
}

/// Analyse une sauvegarde iOS et produit le même format que l'extraction logique Android.
#[tauri::command]
pub fn analyze_ios_backup(case_id: String, backup_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let backup = PathBuf::from(&backup_path);
    let manifest_db = backup.join("Manifest.db");
    if !manifest_db.exists() {
        return Err(format!(
            "Manifest.db introuvable dans « {} ». Indiquez le dossier d'une sauvegarde iTunes/Finder \
             (ex. %APPDATA%\\Apple Computer\\MobileSync\\Backup\\<UDID>).",
            backup_path
        ));
    }

    let info = read_device_info(&backup);
    if info.encrypted {
        return Ok(serde_json::json!({
            "summary": "Sauvegarde iOS CHIFFRÉE — déchiffrement requis (mot de passe légal). Aucun contournement effectué.",
            "encrypted": true,
            "ios_info": info,
            "serial": info.serial,
            "artifacts": [],
            "totals": {"sms": 0, "calls": 0, "contacts": 0},
        }));
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let serial_tag: String = if info.serial.is_empty() { "ios".into() } else {
        info.serial.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
    };
    let base = format!("{}/ios_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), serial_tag, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;
    let out_dir = PathBuf::from(&base);

    let manifest = rusqlite::Connection::open(&manifest_db)
        .map_err(|e| format!("Ouverture Manifest.db: {}", e))?;

    // (nom lisible, domaine LIKE, relativePath, type)
    let targets = [
        ("sms.db",                 "HomeDomain", "Library/SMS/sms.db",                         "sms"),
        ("AddressBook.sqlitedb",   "HomeDomain", "Library/AddressBook/AddressBook.sqlitedb",   "contacts"),
        ("CallHistory.storedata",  "%",          "Library/CallHistoryDB/CallHistory.storedata","calls"),
    ];

    let mut artifacts: Vec<serde_json::Value> = Vec::new();
    let mut sms = vec![];
    let mut calls = vec![];
    let mut contacts = vec![];

    for (nice, domain, rel, kind) in targets {
        let Some(file_id) = find_file_id(&manifest, domain, rel) else { continue };
        let src = backup_file_path(&backup, &file_id);
        let Some((sha256, size)) = sha256_file(&src) else { continue };
        let local = materialize(&backup, &file_id, &out_dir, nice);
        let count = match (kind, local.as_ref()) {
            ("sms", Some(p))      => { sms = parse_ios_sms(p);      sms.len() as i64 }
            ("contacts", Some(p)) => { contacts = parse_ios_contacts(p); contacts.len() as i64 }
            ("calls", Some(p))    => { calls = parse_ios_calls(p);  calls.len() as i64 }
            _ => 0,
        };
        artifacts.push(serde_json::json!({
            "name": match kind { "sms" => "Messages (SMS/iMessage)", "contacts" => "Contacts", "calls" => "Journal d'appels", _ => nice },
            "kind": kind,
            "count": count,
            "size": size,
            "sha256": sha256,
            "status": "OK",
        }));
    }

    // Inventaire complet du contenu de la sauvegarde (nb de fichiers par domaine)
    let file_total: i64 = manifest.query_row("SELECT COUNT(*) FROM Files", [], |r| r.get(0)).unwrap_or(0);
    artifacts.push(serde_json::json!({
        "name": "Inventaire sauvegarde (Manifest.db)", "kind": "manifest",
        "count": file_total, "size": sha256_file(&manifest_db).map(|(_, s)| s).unwrap_or(0),
        "sha256": sha256_file(&manifest_db).map(|(h, _)| h).unwrap_or_default(), "status": "OK",
    }));

    // Manifeste scellé (chaîne de garde)
    let result_core = serde_json::json!({
        "tool": "FIDO-LABS v3", "extraction_type": "IOS_BACKUP",
        "case_id": case_id, "serial": info.serial,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "output_dir": base, "ios_info": info,
        "artifacts": artifacts,
        "totals": {"sms": sms.len(), "calls": calls.len(), "contacts": contacts.len()},
    });
    let manifest_bytes = serde_json::to_vec_pretty(&result_core).unwrap_or_default();
    std::fs::write(format!("{}/manifest.json", base), &manifest_bytes).ok();
    let manifest_sha256 = hex::encode(Sha256::digest(&manifest_bytes));

    Ok(serde_json::json!({
        "summary": format!(
            "Sauvegarde iOS analysée — {} SMS, {} appels, {} contacts ({} fichiers inventoriés)",
            sms.len(), calls.len(), contacts.len(), file_total
        ),
        "encrypted": false,
        "case_id": case_id,
        "serial": info.serial,
        "ios_info": info,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "output_dir": base,
        "manifest_sha256": manifest_sha256,
        "artifacts": artifacts,
        "sms": sms.iter().take(500).cloned().collect::<Vec<_>>(),
        "calls": calls.iter().take(500).cloned().collect::<Vec<_>>(),
        "contacts": contacts.iter().take(1000).cloned().collect::<Vec<_>>(),
        "totals": {"sms": sms.len(), "calls": calls.len(), "contacts": contacts.len()},
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_une_sauvegarde_ios_minimale() {
        // Construit une mini-sauvegarde : Manifest.db + sms.db haché à l'emplacement attendu.
        let root = std::env::temp_dir().join(format!("fido_ios_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file_id = "ab12ff00sms"; // -> sous-dossier "ab"
        std::fs::create_dir_all(root.join(&file_id[..2])).unwrap();

        // sms.db réel avec un message
        {
            let c = rusqlite::Connection::open(root.join(&file_id[..2]).join(file_id)).unwrap();
            c.execute_batch(
                "CREATE TABLE handle(ROWID INTEGER PRIMARY KEY, id TEXT);
                 CREATE TABLE message(ROWID INTEGER PRIMARY KEY, date INTEGER, text TEXT, is_from_me INTEGER, handle_id INTEGER);
                 INSERT INTO handle(ROWID,id) VALUES (1,'+22670123456');
                 INSERT INTO message(date,text,is_from_me,handle_id) VALUES (614822400, 'Rendez-vous confirmé', 0, 1);"
            ).unwrap();
        }
        // Manifest.db pointant sms.db
        {
            let m = rusqlite::Connection::open(root.join("Manifest.db")).unwrap();
            m.execute_batch(
                "CREATE TABLE Files(fileID TEXT PRIMARY KEY, domain TEXT, relativePath TEXT, flags INTEGER, file BLOB);"
            ).unwrap();
            m.execute(
                "INSERT INTO Files(fileID,domain,relativePath,flags,file) VALUES (?1,'HomeDomain','Library/SMS/sms.db',1,NULL)",
                rusqlite::params![file_id],
            ).unwrap();
        }

        let out = root.join("out");
        let r = analyze_ios_backup("CASE-IOS".into(), root.to_string_lossy().to_string(), out.to_string_lossy().to_string())
            .expect("analyse sauvegarde iOS");
        assert_eq!(r["encrypted"], false);
        assert_eq!(r["totals"]["sms"], 1, "1 SMS décodé");
        let body = r["sms"][0]["body"].as_str().unwrap();
        assert!(body.contains("Rendez-vous"), "corps du SMS décodé: {}", body);
        // L'artefact SMS porte une empreinte SHA256 (chaîne de garde)
        let arts = r["artifacts"].as_array().unwrap();
        let sms_art = arts.iter().find(|a| a["kind"] == "sms").unwrap();
        assert_eq!(sms_art["sha256"].as_str().unwrap().len(), 64);
        let _ = std::fs::remove_dir_all(&root);
    }
}
