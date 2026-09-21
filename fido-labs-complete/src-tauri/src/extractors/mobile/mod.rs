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

#[tauri::command]
pub fn extract_logical(case_id: String, serial: String, output_dir: String) -> Result<String, String> {
    use std::process::Command;
    use chrono::Utc;

    // Vérification ADB
    Command::new("adb")
        .args(["-s", &serial, "shell", "echo", "ok"])
        .output()
        .map_err(|_| "ADB non disponible - vérifiez Android Platform Tools".to_string())?;

    // Packages installés
    let pkgs = Command::new("adb")
        .args(["-s", &serial, "shell", "pm", "list", "packages", "-3"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let manifest = format!(
        "FIDO-LABS Extraction Logique\nCase: {}\nSerial: {}\nTimestamp: {}\nPackages:\n{}",
        case_id, serial, ts, pkgs
    );

    let dest = format!("{}/manifest_{}.txt", output_dir, ts);
    std::fs::write(&dest, &manifest).map_err(|e| e.to_string())?;

    Ok(format!("Extraction logique terminée -> {}", dest))
}
