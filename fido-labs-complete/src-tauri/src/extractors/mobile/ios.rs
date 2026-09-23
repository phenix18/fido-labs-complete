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
use std::process::Command;

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

fn parse_ios_safari(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    let mut stmt = match conn.prepare(
        "SELECT hv.visit_time, hi.url, hv.title \
         FROM history_visits hv JOIN history_items hi ON hi.id = hv.history_item \
         ORDER BY hv.visit_time DESC LIMIT 5000"
    ) { Ok(s) => s, Err(_) => return vec![] };
    let rows = stmt.query_map([], |r| {
        let vt: f64 = r.get(0).unwrap_or(0.0);
        let url: Option<String> = r.get(1).ok();
        let title: Option<String> = r.get(2).ok();
        Ok(serde_json::json!({
            "date": apple_ts_to_str(vt as i64),
            "url": url.unwrap_or_default(),
            "title": title.unwrap_or_default(),
        }))
    });
    match rows { Ok(it) => it.filter_map(|x| x.ok()).collect(), Err(_) => vec![] }
}

fn parse_ios_accounts(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    // Comptes e-mail / cloud / réseaux (Gmail, iCloud, Exchange, Facebook…).
    let mut stmt = match conn.prepare(
        "SELECT a.ZUSERNAME, a.ZACCOUNTDESCRIPTION, t.ZACCOUNTTYPEDESCRIPTION \
         FROM ZACCOUNT a LEFT JOIN ZACCOUNTTYPE t ON a.ZACCOUNTTYPE = t.Z_PK \
         WHERE a.ZUSERNAME IS NOT NULL AND a.ZUSERNAME <> '' ORDER BY t.ZACCOUNTTYPEDESCRIPTION"
    ) { Ok(s) => s, Err(_) => return vec![] };
    let rows = stmt.query_map([], |r| {
        let user: Option<String> = r.get(0).ok();
        let desc: Option<String> = r.get(1).ok();
        let typ: Option<String> = r.get(2).ok();
        Ok(serde_json::json!({
            "username": user.unwrap_or_default(),
            "description": desc.unwrap_or_default(),
            "type": typ.unwrap_or_default(),
        }))
    });
    match rows { Ok(it) => it.filter_map(|x| x.ok()).collect(), Err(_) => vec![] }
}

fn parse_ios_locations(db: &Path) -> Vec<serde_json::Value> {
    let conn = match rusqlite::Connection::open(db) { Ok(c) => c, Err(_) => return vec![] };
    // GPS des photos (ZASSET iOS 13+, sinon ZGENERICASSET).
    for table in ["ZASSET", "ZGENERICASSET"] {
        let sql = format!(
            "SELECT ZDATECREATED, ZLATITUDE, ZLONGITUDE, ZFILENAME FROM {} \
             WHERE ZLATITUDE IS NOT NULL AND ZLATITUDE <> -180.0 ORDER BY ZDATECREATED DESC LIMIT 5000",
            table
        );
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => continue };
        let rows = stmt.query_map([], |r| {
            let d: f64 = r.get(0).unwrap_or(0.0);
            let lat: f64 = r.get(1).unwrap_or(0.0);
            let lon: f64 = r.get(2).unwrap_or(0.0);
            let file: Option<String> = r.get(3).ok();
            Ok(serde_json::json!({
                "date": apple_ts_to_str(d as i64),
                "lat": lat, "lon": lon,
                "source": "Photo",
                "label": file.unwrap_or_default(),
            }))
        });
        if let Ok(it) = rows {
            let v: Vec<_> = it.filter_map(|x| x.ok()).collect();
            if !v.is_empty() { return v; }
        }
    }
    vec![]
}

/// Copie une base + ses annexes -wal/-shm dans out_dir (pour que le WAL soit appliqué),
/// et renvoie (chemin copie, sha256 de l'ORIGINAL, taille de l'original).
fn materialize_db(
    manifest: &rusqlite::Connection, backup: &Path, domain: &str, rel: &str,
    out_dir: &Path, nice: &str,
) -> Option<(PathBuf, String, u64)> {
    let file_id = find_file_id(manifest, domain, rel)?;
    let src = backup_file_path(backup, &file_id);
    let (sha, size) = sha256_file(&src)?;
    let dst = out_dir.join(nice);
    std::fs::copy(&src, &dst).ok()?;
    for suf in ["-wal", "-shm"] {
        if let Some(fid) = find_file_id(manifest, domain, &format!("{}{}", rel, suf)) {
            let s = backup_file_path(backup, &fid);
            if s.exists() { std::fs::copy(&s, out_dir.join(format!("{}{}", nice, suf))).ok(); }
        }
    }
    Some((dst, sha, size))
}

/// Lit le keybag et la clé du Manifest depuis Manifest.plist (sauvegarde chiffrée).
fn read_encryption_params(backup: &Path) -> Result<(Vec<u8>, Vec<u8>), String> {
    let v = plist::Value::from_file(backup.join("Manifest.plist"))
        .map_err(|e| format!("Manifest.plist illisible : {}", e))?;
    let d = v.as_dictionary().ok_or("Manifest.plist non-dictionnaire")?;
    let kb = d.get("BackupKeyBag").and_then(|x| x.as_data()).ok_or("BackupKeyBag absent")?.to_vec();
    let mk = d.get("ManifestKey").and_then(|x| x.as_data()).ok_or("ManifestKey absente")?.to_vec();
    Ok((kb, mk))
}

/// Matérialise un fichier CHIFFRÉ : lit le blob de clé dans Manifest.db, déchiffre le
/// fichier, l'écrit dans out_dir. SHA256 renvoyé = celui du fichier chiffré ACQUIS.
fn materialize_encrypted(
    manifest: &rusqlite::Connection, backup: &Path, crypto: &super::ios_crypto::BackupCrypto,
    domain: &str, rel: &str, out_dir: &Path, nice: &str,
) -> Option<(PathBuf, String, u64)> {
    let (file_id, blob): (String, Vec<u8>) = manifest.query_row(
        "SELECT fileID, file FROM Files WHERE relativePath = ?1 AND domain LIKE ?2 LIMIT 1",
        rusqlite::params![rel, domain],
        |r| Ok((r.get(0)?, r.get::<_, Vec<u8>>(1)?)),
    ).ok()?;
    let src = backup_file_path(backup, &file_id);
    let enc = std::fs::read(&src).ok()?;
    let (sha, size) = (hex::encode(Sha256::digest(&enc)), enc.len() as u64);
    let dec = crypto.decrypt_file(&blob, &enc).ok()?;
    let dst = out_dir.join(nice);
    std::fs::write(&dst, &dec).ok()?;
    Some((dst, sha, size))
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
/// `password` déchiffre les sauvegardes chiffrées (mot de passe fourni légalement).
#[tauri::command]
pub fn analyze_ios_backup(case_id: String, backup_path: String, output_dir: String, password: Option<String>) -> Result<serde_json::Value, String> {
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
    let pw = password.unwrap_or_default();
    if info.encrypted && pw.trim().is_empty() {
        return Ok(serde_json::json!({
            "summary": "Sauvegarde iOS CHIFFRÉE — saisissez le mot de passe (légal) pour déchiffrer. Aucun contournement.",
            "encrypted": true, "needs_password": true, "success": false,
            "ios_info": info, "serial": info.serial,
            "artifacts": [], "totals": {"sms": 0, "calls": 0, "contacts": 0},
        }));
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let serial_tag: String = if info.serial.is_empty() { "ios".into() } else {
        info.serial.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
    };
    let base = format!("{}/ios_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), serial_tag, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;
    let out_dir = PathBuf::from(&base);

    // Manifest.db : déchiffré à la volée si la sauvegarde est protégée par mot de passe.
    let crypto: Option<super::ios_crypto::BackupCrypto>;
    let manifest = if info.encrypted {
        let (keybag, manifest_key) = read_encryption_params(&backup)?;
        let c = super::ios_crypto::build_crypto(&keybag, &pw)?; // erreur si mauvais mot de passe
        let enc = std::fs::read(&manifest_db).map_err(|e| format!("Lecture Manifest.db : {}", e))?;
        let dec = c.decrypt_manifest_db(&manifest_key, &enc)?;
        let dec_path = format!("{}/Manifest.db", base);
        std::fs::write(&dec_path, &dec).map_err(|e| format!("Écriture Manifest déchiffré : {}", e))?;
        let conn = rusqlite::Connection::open(&dec_path).map_err(|e| format!("Ouverture Manifest déchiffré : {}", e))?;
        crypto = Some(c);
        conn
    } else {
        crypto = None;
        rusqlite::Connection::open(&manifest_db).map_err(|e| format!("Ouverture Manifest.db : {}", e))?
    };

    // (nom lisible, domaine LIKE, relativePath, type)
    let targets = [
        ("sms.db",                 "HomeDomain",       "Library/SMS/sms.db",                          "sms"),
        ("AddressBook.sqlitedb",   "HomeDomain",       "Library/AddressBook/AddressBook.sqlitedb",    "contacts"),
        ("CallHistory.storedata",  "%",                "Library/CallHistoryDB/CallHistory.storedata", "calls"),
        ("History.db",             "HomeDomain",       "Library/Safari/History.db",                   "browser"),
        ("Accounts3.sqlite",       "HomeDomain",       "Library/Accounts/Accounts3.sqlite",           "accounts"),
        ("Photos.sqlite",          "CameraRollDomain", "Media/PhotoData/Photos.sqlite",               "locations"),
    ];

    let mut artifacts: Vec<serde_json::Value> = Vec::new();
    let mut sms = vec![];
    let mut calls = vec![];
    let mut contacts = vec![];
    let mut browser = vec![];
    let mut accounts = vec![];
    let mut locations = vec![];

    for (nice, domain, rel, kind) in targets {
        let mat = match crypto.as_ref() {
            Some(c) => materialize_encrypted(&manifest, &backup, c, domain, rel, &out_dir, nice),
            None => materialize_db(&manifest, &backup, domain, rel, &out_dir, nice),
        };
        let Some((local, sha256, size)) = mat else { continue };
        let count = match kind {
            "sms"       => { sms = parse_ios_sms(&local);        sms.len() as i64 }
            "contacts"  => { contacts = parse_ios_contacts(&local); contacts.len() as i64 }
            "calls"     => { calls = parse_ios_calls(&local);    calls.len() as i64 }
            "browser"   => { browser = parse_ios_safari(&local); browser.len() as i64 }
            "accounts"  => { accounts = parse_ios_accounts(&local); accounts.len() as i64 }
            "locations" => { locations = parse_ios_locations(&local); locations.len() as i64 }
            _ => 0,
        };
        artifacts.push(serde_json::json!({
            "name": match kind {
                "sms" => "Messages (SMS/iMessage)", "contacts" => "Contacts", "calls" => "Journal d'appels",
                "browser" => "Historique Safari", "accounts" => "Comptes (e-mail/cloud)", "locations" => "Géolocalisation (photos)",
                _ => nice,
            },
            "kind": kind, "count": count, "size": size, "sha256": sha256, "status": "OK",
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
        "totals": {"sms": sms.len(), "calls": calls.len(), "contacts": contacts.len(),
                   "browser": browser.len(), "accounts": accounts.len(), "locations": locations.len()},
    });
    let manifest_bytes = serde_json::to_vec_pretty(&result_core).unwrap_or_default();
    std::fs::write(format!("{}/manifest.json", base), &manifest_bytes).ok();
    let manifest_sha256 = hex::encode(Sha256::digest(&manifest_bytes));

    Ok(serde_json::json!({
        "summary": format!(
            "Sauvegarde iOS analysée — {} SMS, {} appels, {} contacts, {} URL, {} comptes, {} géoloc ({} fichiers inventoriés)",
            sms.len(), calls.len(), contacts.len(), browser.len(), accounts.len(), locations.len(), file_total
        ),
        "encrypted": info.encrypted,
        "decrypted": info.encrypted,
        "success": true,
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
        "browser": browser.iter().take(1000).cloned().collect::<Vec<_>>(),
        "accounts": accounts.iter().take(500).cloned().collect::<Vec<_>>(),
        "locations": locations.iter().take(1000).cloned().collect::<Vec<_>>(),
        "totals": {"sms": sms.len(), "calls": calls.len(), "contacts": contacts.len(),
                   "browser": browser.len(), "accounts": accounts.len(), "locations": locations.len()},
    }))
}

// ─── ACQUISITION iOS EN DIRECT via libimobiledevice ──────────────────────────
// Pilote idevice_id / ideviceinfo / idevicebackup2 (suite libimobiledevice) pour
// détecter un iPhone branché et créer une sauvegarde, puis l'analyser avec
// analyze_ios_backup. Prérequis Windows : service Apple Mobile Device (usbmuxd),
// fourni par l'app « Appareils Apple » (Microsoft Store) ou iTunes, + appairage
// « Se fier » sur l'appareil. Aucun contournement : c'est une sauvegarde autorisée.

fn sanitize_udid(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

/// Localise le dossier des binaires libimobiledevice (env FIDO_IDEVICE_DIR, chemins
/// candidats, sinon PATH). Un dossier vide = s'appuyer sur le PATH.
fn resolve_idevice_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("FIDO_IDEVICE_DIR") {
        let p = PathBuf::from(&d);
        if p.join("idevice_id.exe").exists() || p.join("idevice_id").exists() { return Some(p); }
    }
    for c in [
        "E:/fido-labs-complete/fido-labs-complete/tools/libimobiledevice-w64",
        "tools/libimobiledevice-w64",
    ] {
        let p = PathBuf::from(c);
        if p.join("idevice_id.exe").exists() || p.join("idevice_id").exists() { return Some(p); }
    }
    if Command::new("idevice_id").arg("-h").output().is_ok() { return Some(PathBuf::new()); }
    None
}

fn idevice_tool(dir: &Path, name: &str) -> PathBuf {
    if dir.as_os_str().is_empty() { return PathBuf::from(name); }
    let exe = dir.join(format!("{}.exe", name));
    if exe.exists() { exe } else { dir.join(name) }
}

const USBMUXD_HINT: &str = "Service Apple Mobile Device (usbmuxd) indisponible. Installez l'app « Appareils Apple » (Microsoft Store) ou iTunes, branchez l'iPhone via USB et touchez « Se fier à cet ordinateur » sur l'écran de l'appareil.";

/// Détecte les iPhone/iPad branchés (idevice_id -l) + infos (ideviceinfo).
#[tauri::command]
pub fn list_ios_devices() -> Result<serde_json::Value, String> {
    let dir = resolve_idevice_dir()
        .ok_or_else(|| "libimobiledevice introuvable (tools/libimobiledevice-w64) — définissez FIDO_IDEVICE_DIR.".to_string())?;
    let out = Command::new(idevice_tool(&dir, "idevice_id")).arg("-l").output()
        .map_err(|e| format!("Lancement idevice_id échoué: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if !out.status.success() || stderr.contains("Unable to retrieve device list") {
        return Ok(serde_json::json!({ "available": false, "devices": [], "message": USBMUXD_HINT }));
    }

    let udids: Vec<String> = stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    let mut devices = Vec::new();
    for u in &udids {
        let info = |key: &str| Command::new(idevice_tool(&dir, "ideviceinfo"))
            .args(["-u", u, "-k", key]).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        devices.push(serde_json::json!({
            "udid": u,
            "name": info("DeviceName"),
            "product": info("ProductType"),
            "ios_version": info("ProductVersion"),
            "imei": info("InternationalMobileEquipmentIdentity"),
        }));
    }
    Ok(serde_json::json!({
        "available": true, "devices": devices,
        "message": if udids.is_empty() {
            "Aucun iPhone/iPad détecté. Branchez l'appareil et touchez « Se fier » à l'écran.".to_string()
        } else { format!("{} appareil(s) iOS détecté(s)", udids.len()) },
    }))
}

fn find_backup_dir(base: &Path) -> Option<PathBuf> {
    for entry in walkdir::WalkDir::new(base).max_depth(2).into_iter().flatten() {
        if entry.file_name() == "Manifest.db" {
            return entry.path().parent().map(|p| p.to_path_buf());
        }
    }
    None
}

/// Crée une sauvegarde en direct depuis l'iPhone (idevicebackup2), puis l'analyse.
/// Peut être long (plusieurs Go). Nécessite usbmuxd + appareil appairé (« Se fier »).
#[tauri::command]
pub fn create_ios_backup(case_id: String, udid: String, output_dir: String) -> Result<serde_json::Value, String> {
    let dir = resolve_idevice_dir()
        .ok_or_else(|| "libimobiledevice introuvable (tools/libimobiledevice-w64) — définissez FIDO_IDEVICE_DIR.".to_string())?;

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/ios_live_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), sanitize_udid(&case_id), ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;

    let mut args: Vec<String> = Vec::new();
    if !udid.trim().is_empty() { args.push("-u".into()); args.push(udid.trim().to_string()); }
    args.push("backup".into());
    args.push(base.clone());

    let out = Command::new(idevice_tool(&dir, "idevicebackup2")).args(&args).output()
        .map_err(|e| format!("Lancement idevicebackup2 échoué: {}", e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    std::fs::write(format!("{}/idevicebackup2_log.txt", base),
        format!("$ idevicebackup2 {}\n\n=== STDOUT ===\n{}\n=== STDERR ===\n{}", args.join(" "), stdout, stderr)).ok();

    // Localiser la sauvegarde produite (dossier contenant Manifest.db)
    let backup_dir = match find_backup_dir(Path::new(&base)) {
        Some(d) => d,
        None => {
            let hint = if stderr.contains("Unable to retrieve device list") || stderr.contains("No device found") {
                USBMUXD_HINT
            } else if stderr.contains("pair") || stderr.contains("Trust") || stderr.contains("Please accept") {
                "Appairage requis : touchez « Se fier à cet ordinateur » sur l'iPhone, puis relancez."
            } else {
                "La sauvegarde n'a pas abouti (voir idevicebackup2_log.txt)."
            };
            let n = stderr.chars().count();
            let tail: String = stderr.chars().skip(n.saturating_sub(600)).collect();
            return Ok(serde_json::json!({
                "success": false, "output_dir": base,
                "summary": hint, "log": tail,
            }));
        }
    };

    // Analyser la sauvegarde produite (même pipeline que l'import iTunes/Finder)
    let mut parsed = analyze_ios_backup(case_id, backup_dir.to_string_lossy().to_string(), output_dir, None)?;
    if let Some(obj) = parsed.as_object_mut() {
        obj.insert("acquisition".into(), serde_json::json!("LIVE_IDEVICEBACKUP2"));
        obj.insert("live_backup_dir".into(), serde_json::json!(backup_dir.to_string_lossy()));
    }
    Ok(parsed)
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
        // Accounts3.sqlite : un compte Gmail
        let acc_id = "cd34ee11acc";
        std::fs::create_dir_all(root.join(&acc_id[..2])).unwrap();
        {
            let c = rusqlite::Connection::open(root.join(&acc_id[..2]).join(acc_id)).unwrap();
            c.execute_batch(
                "CREATE TABLE ZACCOUNTTYPE(Z_PK INTEGER PRIMARY KEY, ZACCOUNTTYPEDESCRIPTION TEXT);
                 CREATE TABLE ZACCOUNT(Z_PK INTEGER PRIMARY KEY, ZUSERNAME TEXT, ZACCOUNTDESCRIPTION TEXT, ZACCOUNTTYPE INTEGER);
                 INSERT INTO ZACCOUNTTYPE(Z_PK,ZACCOUNTTYPEDESCRIPTION) VALUES (1,'Google');
                 INSERT INTO ZACCOUNT(ZUSERNAME,ZACCOUNTDESCRIPTION,ZACCOUNTTYPE) VALUES ('suspect@gmail.com','Gmail',1);"
            ).unwrap();
        }
        // Manifest.db pointant sms.db + Accounts3.sqlite
        {
            let m = rusqlite::Connection::open(root.join("Manifest.db")).unwrap();
            m.execute_batch(
                "CREATE TABLE Files(fileID TEXT PRIMARY KEY, domain TEXT, relativePath TEXT, flags INTEGER, file BLOB);"
            ).unwrap();
            m.execute(
                "INSERT INTO Files(fileID,domain,relativePath,flags,file) VALUES (?1,'HomeDomain','Library/SMS/sms.db',1,NULL)",
                rusqlite::params![file_id],
            ).unwrap();
            m.execute(
                "INSERT INTO Files(fileID,domain,relativePath,flags,file) VALUES (?1,'HomeDomain','Library/Accounts/Accounts3.sqlite',1,NULL)",
                rusqlite::params![acc_id],
            ).unwrap();
        }

        let out = root.join("out");
        let r = analyze_ios_backup("CASE-IOS".into(), root.to_string_lossy().to_string(), out.to_string_lossy().to_string(), None)
            .expect("analyse sauvegarde iOS");
        assert_eq!(r["encrypted"], false);
        assert_eq!(r["totals"]["sms"], 1, "1 SMS décodé");
        let body = r["sms"][0]["body"].as_str().unwrap();
        assert!(body.contains("Rendez-vous"), "corps du SMS décodé: {}", body);
        // Compte e-mail (Gmail) décodé
        assert_eq!(r["totals"]["accounts"], 1, "1 compte e-mail");
        assert!(r["accounts"][0]["username"].as_str().unwrap().contains("gmail"), "compte gmail détecté");
        // L'artefact SMS porte une empreinte SHA256 (chaîne de garde)
        let arts = r["artifacts"].as_array().unwrap();
        let sms_art = arts.iter().find(|a| a["kind"] == "sms").unwrap();
        assert_eq!(sms_art["sha256"].as_str().unwrap().len(), 64);
        let _ = std::fs::remove_dir_all(&root);
    }
}
