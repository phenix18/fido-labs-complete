// MOBILE EXTRACTOR mod.rs v3
pub mod android;
pub mod ios;
pub mod advanced;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceProfile {
    pub brand: String,
    pub model: String,
    pub chipset: String,
    pub android_version: String,
    pub state: String,         // ON, OFF, LOCKED, BROKEN_SCREEN
    pub extraction_method: String,
    pub serial: String,
    pub imei: String,
    pub storage_gb: f32,
    pub battery: i32,
}

#[tauri::command]
pub fn detect_device() -> Result<DeviceProfile, String> {
    use std::process::Command;

    // Essai ADB en premier
    if let Ok(out) = Command::new("adb").args(["devices"]).output() {
        let txt = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = txt.lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .collect();

        if let Some(line) = lines.first() {
            let serial = line.split_whitespace().next().unwrap_or("unknown").to_string();
            let state = if line.contains("unauthorized") { "ADB_UNAUTHORIZED" }
                        else if line.contains("offline") { "OFF" }
                        else { "ON" };

            // Récupérer les infos du device
            let brand = adb_getprop(&serial, "ro.product.brand");
            let model = adb_getprop(&serial, "ro.product.model");
            let hw    = adb_getprop(&serial, "ro.hardware");
            let ver   = adb_getprop(&serial, "ro.build.version.release");
            let imei  = adb_getprop(&serial, "ro.ril.oem.imei");

            return Ok(DeviceProfile {
                brand,
                model,
                chipset: hw,
                android_version: ver,
                state: state.into(),
                extraction_method: "ADB Logical".into(),
                serial,
                imei,
                storage_gb: 0.0,
                battery: -1,
            });
        }
    }

    // Aucun device - retourner profil vide
    Err("Aucun appareil détecté. Branchez un appareil et activez le débogage USB.".into())
}

fn adb_getprop(serial: &str, prop: &str) -> String {
    use std::process::Command;
    Command::new("adb")
        .args(["-s", serial, "shell", "getprop", prop])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "N/A".into())
}

#[tauri::command]
pub fn list_usb_devices() -> Result<Vec<android::UsbDevice>, String> {
    android::list_usb_devices()
}

#[tauri::command]
pub fn get_device_info(serial: String) -> Result<serde_json::Value, String> {
    android::get_device_info(serial)
}

/// Un artefact extrait : fichier sur disque + empreinte SHA256 (chaîne de garde)
#[derive(Serialize, Deserialize, Clone)]
pub struct Artifact {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub count: i64,
    pub size: u64,
    pub sha256: String,
    pub status: String,
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn count_rows(kind: &str, text: &str) -> i64 {
    match kind {
        "sms" | "calls" | "contacts" =>
            text.lines().filter(|l| l.trim_start().starts_with("Row:")).count() as i64,
        "apps" =>
            text.lines().filter(|l| l.contains("package:")).count() as i64,
        _ =>
            text.lines().filter(|l| !l.trim().is_empty()).count() as i64,
    }
}

/// Exécute `adb -s SERIAL <args>` et renvoie la sortie brute, ou l'erreur adb.
fn adb_dump(serial: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("adb")
        .arg("-s").arg(serial)
        .args(args)
        .output()
        .map_err(|e| format!("adb introuvable: {}", e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() { "commande adb échouée".into() } else { err });
    }
    Ok(out.stdout)
}

/// Extraction LOGIQUE réelle via ADB (sans root) : SMS, appels, contacts,
/// comptes, Wi-Fi, applications, fichiers /sdcard + propriétés appareil.
/// Chaque artefact est écrit sur disque et scellé par un SHA256 ; un manifeste
/// JSON récapitule l'ensemble (chaîne de garde).
#[tauri::command]
pub fn extract_logical(case_id: String, serial: String, output_dir: String) -> Result<serde_json::Value, String> {
    use chrono::Utc;
    use sha2::{Sha256, Digest};

    // 1. Vérifier la connexion ADB
    let check = std::process::Command::new("adb")
        .args(["-s", &serial, "shell", "echo", "ok"])
        .output()
        .map_err(|_| "ADB non disponible — installez Android Platform Tools (adb) et activez le débogage USB".to_string())?;
    if !check.status.success() {
        return Err(format!("Appareil '{}' injoignable via ADB (débogage USB autorisé ?)", serial));
    }

    // 2. Dossier d'extraction horodaté
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!(
        "{}/extraction_{}_{}",
        output_dir.trim_end_matches(|c| c == '/' || c == '\\'),
        sanitize(&serial), ts
    );
    std::fs::create_dir_all(&base).map_err(|e| format!("Création dossier {}: {}", base, e))?;

    // 3. Sources logiques (content providers / dumpsys, sans root)
    let sources: Vec<(&str, &str, &str, Vec<&str>)> = vec![
        ("Propriétés appareil",   "device",   "device_props.txt", vec!["shell", "getprop"]),
        ("Applications installées","apps",     "packages.txt",     vec!["shell", "pm", "list", "packages", "-f", "-3"]),
        ("SMS",                   "sms",      "sms.txt",          vec!["shell", "content", "query", "--uri", "content://sms"]),
        ("Journal d'appels",      "calls",    "call_log.txt",     vec!["shell", "content", "query", "--uri", "content://call_log/calls"]),
        ("Contacts",              "contacts", "contacts.txt",     vec!["shell", "content", "query", "--uri", "content://com.android.contacts/data/phones"]),
        ("Comptes",               "accounts", "accounts.txt",     vec!["shell", "dumpsys", "account"]),
        ("Wi-Fi connus",          "wifi",     "wifi.txt",         vec!["shell", "cmd", "wifi", "list-networks"]),
        ("Inventaire /sdcard + SHA256 (à la source)", "media", "sdcard_inventory_sha256.txt",
                                              vec!["shell", "find", "/sdcard", "-type", "f", "-exec", "sha256sum", "{}", "+"]),
    ];

    let mut artifacts: Vec<Artifact> = Vec::new();
    for (name, kind, fname, args) in &sources {
        let fpath = format!("{}/{}", base, fname);
        match adb_dump(&serial, args) {
            Ok(bytes) => {
                std::fs::write(&fpath, &bytes).ok();
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let sha = hex::encode(hasher.finalize());
                let count = count_rows(kind, &String::from_utf8_lossy(&bytes));
                artifacts.push(Artifact {
                    name: (*name).into(), kind: (*kind).into(), file: fpath,
                    count, size: bytes.len() as u64, sha256: sha, status: "OK".into(),
                });
            }
            Err(e) => artifacts.push(Artifact {
                name: (*name).into(), kind: (*kind).into(), file: fpath,
                count: 0, size: 0, sha256: String::new(), status: format!("ÉCHEC: {}", e),
            }),
        }
    }

    // 4. Manifeste chaîne de garde (JSON)
    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3",
        "extraction_type": "LOGICAL_ADB",
        "case_id": case_id,
        "serial": serial,
        "timestamp": Utc::now().to_rfc3339(),
        "output_dir": base,
        "artifacts": artifacts,
    });
    let manifest_path = format!("{}/manifest.json", base);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default()).ok();

    // 5. Sceau : SHA256 global du manifeste
    let mut h = Sha256::new();
    h.update(serde_json::to_string(&manifest).unwrap_or_default().as_bytes());
    let manifest_sha = hex::encode(h.finalize());

    let ok = artifacts.iter().filter(|a| a.status == "OK").count();
    Ok(serde_json::json!({
        "summary": format!("Extraction logique terminée — {}/{} sources OK", ok, artifacts.len()),
        "case_id": case_id,
        "serial": serial,
        "timestamp": Utc::now().to_rfc3339(),
        "output_dir": base,
        "manifest": manifest_path,
        "manifest_sha256": manifest_sha,
        "artifacts": artifacts,
    }))
}

fn categorize(p: &std::path::Path) -> String {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" => "Images",
        "mp4" | "mkv" | "3gp" | "avi" | "mov" | "webm"           => "Vidéos",
        "mp3" | "ogg" | "opus" | "wav" | "m4a" | "aac"           => "Audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "txt" | "rtf"  => "Documents",
        "db" | "sqlite" | "crypt14" | "crypt15"                  => "Bases de données",
        _                                                        => "Autres",
    }.to_string()
}

/// Collecte RÉELLE des médias/fichiers du stockage partagé (sans root) :
/// DCIM, Pictures, Downloads + dossiers médias WhatsApp/Facebook/Telegram/Signal…
/// Chaque fichier récupéré est haché (SHA256) et catalogué dans un manifeste.
#[tauri::command]
pub fn pull_media(case_id: String, serial: String, output_dir: String) -> Result<serde_json::Value, String> {
    use chrono::Utc;
    use sha2::{Sha256, Digest};
    use std::io::Read;

    let check = std::process::Command::new("adb")
        .args(["-s", &serial, "shell", "echo", "ok"]).output()
        .map_err(|_| "ADB non disponible — installez Android Platform Tools".to_string())?;
    if !check.status.success() {
        return Err(format!("Appareil '{}' injoignable via ADB", serial));
    }

    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!(
        "{}/media_{}_{}",
        output_dir.trim_end_matches(|c| c == '/' || c == '\\'),
        sanitize(&serial), ts
    );
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;

    // Répertoires d'intérêt sur le stockage partagé (accessibles sans root)
    let dirs = [
        "/sdcard/DCIM", "/sdcard/Pictures", "/sdcard/Movies", "/sdcard/Music",
        "/sdcard/Download", "/sdcard/Documents",
        "/sdcard/WhatsApp", "/sdcard/Android/media/com.whatsapp",
        "/sdcard/Android/media/com.facebook.orca", "/sdcard/Android/media/com.facebook.katana",
        "/sdcard/Android/media/org.telegram.messenger", "/sdcard/Telegram",
        "/sdcard/Android/media/org.thoughtcrime.securesms",
        "/sdcard/Android/media/com.instagram.android", "/sdcard/Snapchat",
    ];

    let mut pulled_dirs = Vec::new();
    for d in dirs {
        let exists = std::process::Command::new("adb")
            .args(["-s", &serial, "shell", "test", "-d", d, "&&", "echo", "Y"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains('Y'))
            .unwrap_or(false);
        if !exists { continue; }
        let _ = std::process::Command::new("adb")
            .args(["-s", &serial, "pull", "-a", d, &base]).output();
        pulled_dirs.push(d.to_string());
    }

    // Hacher tous les fichiers récupérés
    let mut files = Vec::new();
    let mut total_bytes = 0u64;
    let mut by_cat: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(&base).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }
        let p = entry.path();
        if let Ok(mut f) = std::fs::File::open(p) {
            let mut hasher = Sha256::new();
            let mut buf = vec![0u8; 1 << 20];
            let mut size = 0u64;
            loop {
                let n = f.read(&mut buf).unwrap_or(0);
                if n == 0 { break; }
                hasher.update(&buf[..n]);
                size += n as u64;
            }
            total_bytes += size;
            let cat = categorize(p);
            *by_cat.entry(cat.clone()).or_insert(0) += 1;
            files.push(serde_json::json!({
                "path": p.to_string_lossy(),
                "size": size,
                "sha256": hex::encode(hasher.finalize()),
                "category": cat,
            }));
        }
    }

    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3",
        "extraction_type": "MEDIA_PULL",
        "case_id": case_id,
        "serial": serial,
        "timestamp": Utc::now().to_rfc3339(),
        "output_dir": base,
        "pulled_dirs": pulled_dirs,
        "file_count": files.len(),
        "total_bytes": total_bytes,
        "by_category": by_cat,
        "files": files,
    });
    std::fs::write(
        format!("{}/media_manifest.json", base),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    ).ok();

    Ok(serde_json::json!({
        "summary": format!(
            "{} fichiers collectés ({:.1} Mo) depuis {} répertoire(s)",
            manifest["file_count"], total_bytes as f64 / 1e6, pulled_dirs.len()
        ),
        "output_dir": base,
        "file_count": manifest["file_count"],
        "total_bytes": total_bytes,
        "by_category": by_cat,
        "pulled_dirs": pulled_dirs,
    }))
}
