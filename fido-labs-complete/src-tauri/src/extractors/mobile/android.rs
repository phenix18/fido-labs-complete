// ANDROID EXTRACTOR v2 - FIDO-LABS
// Gère 4 états: ON / OFF (BROM/EDL) / LOCKED (avec autorisation) / BROKEN_SCREEN (OTG/Scrcpy)

use serde::{Deserialize, Serialize};
use std::process::Command;

pub enum PhoneState {
    On,
    Off,     // -> BROM pour MTK (VID 0e8d), EDL 9008 pour Qualcomm (VID 05c6)
    Locked,  // -> Nécessite autorisation + méthode légale (pas de bruteforce)
    BrokenScreen, // -> OTG + ADB + Screen Mirroring
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UsbDevice {
    pub vid: String,
    pub pid: String,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub detected_type: String, // ADB, BROM_MTK, EDL_QUALCOMM, FASTBOOT, MTP, iOS
}

/// Détecte les appareils USB branchés (ADB + USB raw)
#[tauri::command]
pub fn list_usb_devices() -> Result<Vec<UsbDevice>, String> {
    let mut devices = Vec::new();

    // Tentative via ADB
    if let Ok(output) = Command::new("adb").args(["devices", "-l"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            if line.trim().is_empty() { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let serial = parts[0];
                let state = parts[1];
                if state == "device" || state == "unauthorized" {
                    devices.push(UsbDevice {
                        vid: "Unknown".into(),
                        pid: "Unknown".into(),
                        manufacturer: "Android".into(),
                        product: serial.to_string(),
                        serial: serial.to_string(),
                        detected_type: if state == "unauthorized" { "ADB_UNAUTHORIZED".into() } else { "ADB".into() },
                    });
                }
            }
        }
    }

    // Détection USB raw via rusb (VID/PID connus forensique)
    if let Ok(usb_devices) = rusb::devices() {
        for device in usb_devices.iter() {
            if let Ok(desc) = device.device_descriptor() {
                let vid = format!("{:04x}", desc.vendor_id());
                let pid = format!("{:04x}", desc.product_id());
                let detected_type = detect_device_type(desc.vendor_id(), desc.product_id());
                if detected_type != "UNKNOWN" {
                    devices.push(UsbDevice {
                        vid: vid.clone(),
                        pid: pid.clone(),
                        manufacturer: format!("VID:{}", vid),
                        product: format!("PID:{}", pid),
                        serial: format!("{}-{}", desc.vendor_id(), desc.product_id()),
                        detected_type,
                    });
                }
            }
        }
    }

    Ok(devices)
}

fn detect_device_type(vid: u16, pid: u16) -> String {
    match (vid, pid) {
        (0x0e8d, 0x0003) | (0x0e8d, 0x2000) => "BROM_MTK".into(),
        (0x05c6, 0x9008) => "EDL_QUALCOMM".into(),
        (0x05c6, 0x9025) | (0x05c6, 0x900e) => "FASTBOOT_QUALCOMM".into(),
        (0x18d1, 0x4ee7) | (0x18d1, 0xd001) => "FASTBOOT_GOOGLE".into(),
        (0x05ac, _) => "iOS".into(),
        (0x04e8, _) => "SAMSUNG".into(),
        (0x2717, _) => "XIAOMI".into(),
        (0x12d1, _) => "HUAWEI".into(),
        _ => "UNKNOWN".into(),
    }
}

pub fn handle_state(state: PhoneState) -> Result<String, String> {
    match state {
        PhoneState::On => {
            // ADB logical + Agent APK
            adb_logical_extraction()
        }
        PhoneState::Off => {
            // Si VID 0e8d -> MTKClient handshake BROM
            // Si VID 05c6 -> QFIL EDL handshake - lecture UFS
            Err("Mode BROM/EDL: Nécessite câble EDL / test point - matériel forensic certifié".into())
        }
        PhoneState::Locked => {
            // INVESTIGATEUR: Ne pas bruteforcer sans cadre légal
            Err("VERROU ACTIF: Demande code au propriétaire ou mandat judiciaire requis. FIDO-LABS ne contourne pas les verroux sans autorisation.".into())
        }
        PhoneState::BrokenScreen => {
            // Si ADB déjà autorisé -> scrcpy mirror + extraction
            adb_logical_extraction()
        }
    }
}

fn adb_logical_extraction() -> Result<String, String> {
    // Vérifie qu'ADB est disponible
    Command::new("adb")
        .args(["shell", "echo", "ok"])
        .output()
        .map(|_| "ADB connecté - prêt pour extraction logique".into())
        .map_err(|_| "ADB non trouvé - installez Android Platform Tools".into())
}

#[tauri::command]
pub fn get_device_info(serial: String) -> Result<serde_json::Value, String> {
    let props = vec![
        ("ro.product.brand", "brand"),
        ("ro.product.model", "model"),
        ("ro.product.device", "device"),
        ("ro.build.version.release", "android_version"),
        ("ro.build.version.sdk", "sdk"),
        ("ro.hardware", "hardware"),
        ("ro.serialno", "serial"),
        ("ro.product.cpu.abi", "cpu_abi"),
    ];

    let mut info = serde_json::json!({});

    for (prop, key) in props {
        let output = Command::new("adb")
            .args(["-s", &serial, "shell", "getprop", prop])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "N/A".into());
        info[key] = serde_json::Value::String(output);
    }

    // Stockage
    if let Ok(output) = Command::new("adb").args(["-s", &serial, "shell", "df", "/data"]).output() {
        info["storage"] = serde_json::Value::String(
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        );
    }

    Ok(info)
}
