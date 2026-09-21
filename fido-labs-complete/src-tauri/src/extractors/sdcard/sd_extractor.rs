// SD CARD EXTRACTOR - FAT32 / exFAT - carving secteur par secteur
use std::fs::{File, create_dir_all};
use std::io::{Read, Write, Seek, SeekFrom};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use chrono::Utc;

use crate::extractors::sdcard::Partition;

// Signatures de fichiers connues (magic bytes)
const FILE_SIGS: &[(&[u8], &str, &str)] = &[
    (&[0xFF, 0xD8, 0xFF], "jpg",  "Images"),
    (&[0x89, 0x50, 0x4E, 0x47], "png",  "Images"),
    (&[0x47, 0x49, 0x46, 0x38], "gif",  "Images"),
    (&[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70], "mp4", "Videos"),
    (&[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70], "mp4", "Videos"),
    (&[0x1A, 0x45, 0xDF, 0xA3], "mkv",  "Videos"),
    (&[0x25, 0x50, 0x44, 0x46], "pdf",  "Documents"),
    (&[0x50, 0x4B, 0x03, 0x04], "docx", "Documents"),
    (&[0xD0, 0xCF, 0x11, 0xE0], "doc",  "Documents"),
    (&[0x52, 0x49, 0x46, 0x46], "wav",  "Audio"),
    (&[0x49, 0x44, 0x33], "mp3",  "Audio"),
    (&[0x4F, 0x67, 0x67, 0x53], "ogg",  "Audio"),
    (&[0x53, 0x51, 0x4C, 0x69], "db",   "Databases"),
    (&[0x3C, 0x3F, 0x78, 0x6D, 0x6C], "xml", "Documents"),
];

pub fn image_sd_card(source: &str, dest: &str, case_id: &str) -> Result<String, String> {
    let mut src = File::open(source).map_err(|e| format!("Impossible d'ouvrir {}: {}", source, e))?;
    let mut dst = File::create(dest).map_err(|e| format!("Impossible de créer {}: {}", dest, e))?;

    let meta = src.metadata().map_err(|e| e.to_string())?;
    let total = meta.len();

    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 4 * 1024 * 1024]; // 4MB blocs
    let mut copied = 0u64;

    loop {
        let n = src.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        dst.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        copied += n as u64;
    }

    let hash = hex::encode(hasher.finalize());
    let ts = Utc::now().to_rfc3339();

    // Manifest
    let manifest = format!(
        "FIDO-LABS SD Image\nCase: {}\nSource: {}\nDest: {}\nSize: {} bytes\nSHA256: {}\nTimestamp: {}\n",
        case_id, source, dest, total, hash, ts
    );
    let manifest_path = format!("{}.manifest.txt", dest);
    std::fs::write(&manifest_path, &manifest).ok();

    Ok(format!("Image SD créée: {} octets | SHA256: {}", copied, &hash[..16]))
}

pub fn carve_sd(image_path: &str, output_dir: &str) -> Result<serde_json::Value, String> {
    let mut f = File::open(image_path).map_err(|e| e.to_string())?;
    let meta = f.metadata().map_err(|e| e.to_string())?;
    let total = meta.len() as usize;

    create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let buf_size = 8 * 1024 * 1024usize; // 8MB window
    let mut buffer = vec![0u8; buf_size];
    let mut offset = 0usize;
    let mut carved: Vec<serde_json::Value> = Vec::new();
    let mut counters: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    while offset < total {
        let read_len = (total - offset).min(buf_size);
        f.seek(SeekFrom::Start(offset as u64)).map_err(|e| e.to_string())?;
        let n = f.read(&mut buffer[..read_len]).map_err(|e| e.to_string())?;
        if n == 0 { break; }

        let data = &buffer[..n];
        for i in 0..n.saturating_sub(8) {
            for (sig, ext, category) in FILE_SIGS {
                if data[i..].starts_with(sig) {
                    let cnt = counters.entry(ext.to_string()).or_insert(0);
                    *cnt += 1;
                    let cat_dir = format!("{}/{}", output_dir, category);
                    create_dir_all(&cat_dir).ok();
                    let filename = format!("{}/carved_{:08x}.{}", cat_dir, offset + i, ext);

                    // Extraire jusqu'à 20MB max ou fin de buffer
                    let end = (i + 20 * 1024 * 1024).min(n);
                    if let Ok(mut out) = File::create(&filename) {
                        out.write_all(&data[i..end]).ok();
                    }

                    carved.push(serde_json::json!({
                        "offset": offset + i,
                        "ext": ext,
                        "category": category,
                        "filename": filename,
                        "size_approx": end - i,
                    }));
                }
            }
        }
        offset += n.saturating_sub(8); // overlap pour ne pas rater les sigs sur les bords
    }

    Ok(serde_json::json!({
        "total_carved": carved.len(),
        "files": carved,
        "output_dir": output_dir,
    }))
}

pub fn list_partitions(path: &str) -> Result<Vec<Partition>, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut mbr = [0u8; 512];
    f.read_exact(&mut mbr).map_err(|e| e.to_string())?;

    // Signature MBR
    if mbr[510] != 0x55 || mbr[511] != 0xAA {
        return Ok(vec![Partition {
            index: 0,
            offset: 0,
            size_mb: f.metadata().map(|m| m.len() / 1024 / 1024).unwrap_or(0),
            filesystem: "RAW/GPT".into(),
        }]);
    }

    let mut parts = Vec::new();
    for i in 0..4 {
        let off = 446 + i * 16;
        let ptype = mbr[off + 4];
        if ptype == 0 { continue; }
        let lba_start = u32::from_le_bytes(mbr[off+8..off+12].try_into().unwrap()) as u64;
        let lba_size  = u32::from_le_bytes(mbr[off+12..off+16].try_into().unwrap()) as u64;
        let fs = match ptype {
            0x0B | 0x0C => "FAT32",
            0x07 => "NTFS/exFAT",
            0x83 => "Linux ext",
            0x8E => "Linux LVM",
            _ => "Inconnu",
        };
        parts.push(Partition {
            index: i as u32,
            offset: lba_start * 512,
            size_mb: (lba_size * 512) / 1024 / 1024,
            filesystem: fs.into(),
        });
    }
    Ok(parts)
}
