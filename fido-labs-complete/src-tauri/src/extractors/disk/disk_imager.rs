// DISK IMAGER v2 - Image E01/DD avec SHA256 chain of custody
use std::fs::{File, create_dir_all};
use std::io::{Read, Write, Seek, SeekFrom};
use sha2::{Sha256, Digest};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::extractors::disk::DiskInfo;

pub fn enumerate_disks() -> Result<Vec<DiskInfo>, String> {
    let mut disks = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // wmic est absent des Windows 11 récents → on interroge CIM via PowerShell.
        let ps = "Get-CimInstance Win32_DiskDrive | ForEach-Object { \"$($_.DeviceID)|$($_.Model)|$($_.Size)|$($_.MediaType)|$($_.InterfaceType)\" }";
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps])
            .output()
        {
            let txt = String::from_utf8_lossy(&out.stdout);
            for line in txt.lines() {
                let cols: Vec<&str> = line.trim().split('|').collect();
                if cols.len() >= 5 {
                    let size_bytes: u64 = cols[2].trim().parse().unwrap_or(0);
                    let iface = cols[4].to_lowercase();
                    let media = cols[3].to_lowercase();
                    disks.push(DiskInfo {
                        path: cols[0].trim().to_string(),
                        label: cols[1].trim().to_string(),
                        size_gb: size_bytes as f64 / 1e9,
                        filesystem: cols[3].trim().to_string(),
                        removable: iface.contains("usb")
                            || media.contains("removable")
                            || media.contains("external"),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(out) = Command::new("lsblk")
            .args(["-J", "-b", "-o", "NAME,SIZE,TYPE,FSTYPE,LABEL,RM"])
            .output()
        {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(devs) = json["blockdevices"].as_array() {
                    for d in devs {
                        let name = d["name"].as_str().unwrap_or("?");
                        let size: u64 = d["size"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let fstype = d["fstype"].as_str().unwrap_or("?").to_string();
                        let rm = d["rm"].as_bool().unwrap_or(false);
                        disks.push(DiskInfo {
                            path: format!("/dev/{}", name),
                            label: d["label"].as_str().unwrap_or(name).to_string(),
                            size_gb: size as f64 / 1e9,
                            filesystem: fstype,
                            removable: rm,
                        });
                    }
                }
            }
        }
    }

    // Pas de disque de démonstration : en forensique on ne doit JAMAIS présenter
    // un support fictif comme réel. Une liste vide = aucun support détecté.
    Ok(disks)
}

pub fn create_forensic_image(source: &str, dest: &str, format: &str, case_id: &str) -> Result<String, String> {
    let mut src = File::open(source)
        .map_err(|e| format!("Impossible d'ouvrir {}: {}", source, e))?;

    if let Some(parent) = std::path::Path::new(dest).parent() {
        create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let dest_path = if format == "E01" {
        format!("{}.E01", dest.trim_end_matches(".E01"))
    } else {
        format!("{}.dd", dest.trim_end_matches(".dd"))
    };

    let mut dst = File::create(&dest_path)
        .map_err(|e| format!("Impossible de créer {}: {}", dest_path, e))?;

    let meta = src.metadata().map_err(|e| e.to_string())?;
    let total = meta.len();

    // Pour E01: entête simplifié (en-tête EWF basique)
    if format == "E01" {
        write_ewf_header(&mut dst, source, case_id, total)?;
    }

    let mut hasher_src = Sha256::new();
    let block_size = 512 * 1024usize; // 512 KB
    let mut buf = vec![0u8; block_size];
    let mut written = 0u64;
    let mut segment_hasher = Sha256::new();

    loop {
        let n = src.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        dst.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher_src.update(&buf[..n]);
        segment_hasher.update(&buf[..n]);
        written += n as u64;
    }

    let hash_src = hex::encode(hasher_src.finalize());
    let ts = Utc::now().to_rfc3339();

    // Footer E01 avec hash
    if format == "E01" {
        let footer = format!("\n[MD5]\nSource: {}\nHash: {}\n", source, &hash_src);
        dst.write_all(footer.as_bytes()).ok();
    }

    // Manifest chain of custody
    let manifest = format!(
        "=== FIDO-LABS FORENSIC IMAGE MANIFEST ===\n\
        Case ID: {}\n\
        Source: {}\n\
        Destination: {}\n\
        Format: {}\n\
        Size: {} bytes ({:.2} GB)\n\
        SHA256: {}\n\
        Timestamp: {}\n\
        Operator: FIDO-LABS\n\
        ==========================================\n",
        case_id, source, dest_path, format,
        written, written as f64 / 1e9,
        hash_src, ts
    );

    let mpath = format!("{}.manifest.txt", dest_path);
    std::fs::write(&mpath, &manifest).ok();

    Ok(format!("Image créée: {} | {:.2}GB | SHA256: {}...", dest_path, written as f64/1e9, &hash_src[..16]))
}

fn write_ewf_header(dst: &mut File, source: &str, case_id: &str, _size: u64) -> Result<(), String> {
    // En-tête EWF simplifié (signature + métadonnées case)
    let sig = b"EVF\x09\x0D\x0A\xFF\x00"; // EVF signature
    dst.write_all(sig).map_err(|e| e.to_string())?;
    let header = format!(
        "1\nmain\nc\t{}\nn\tFIDO-LABS\ne\tForensic Image\n",
        case_id
    );
    dst.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn verify_image_hash(image_path: &str) -> Result<String, String> {
    let mpath = format!("{}.manifest.txt", image_path);
    if let Ok(manifest) = std::fs::read_to_string(&mpath) {
        // Extraire le hash du manifest
        for line in manifest.lines() {
            if line.starts_with("SHA256:") {
                let stored_hash = line.trim_start_matches("SHA256:").trim();
                // Recalculer
                let mut f = File::open(image_path).map_err(|e| e.to_string())?;
                let mut hasher = Sha256::new();
                let mut buf = vec![0u8; 4 * 1024 * 1024];
                loop {
                    let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                    if n == 0 { break; }
                    hasher.update(&buf[..n]);
                }
                let computed = hex::encode(hasher.finalize());
                if computed == stored_hash {
                    return Ok(format!("✓ INTÉGRITÉ OK | SHA256: {}", &computed[..32]));
                } else {
                    return Err(format!("✗ HASH MISMATCH! Attendu: {}... Calculé: {}...",
                        &stored_hash[..16], &computed[..16]));
                }
            }
        }
    }
    Err("Manifest non trouvé - impossible de vérifier l'intégrité".into())
}
