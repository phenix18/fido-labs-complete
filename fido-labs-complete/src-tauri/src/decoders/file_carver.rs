// FILE CARVER v2 - JPG, MP4, PDF, DOCX, SQLite sur disque/SD brute
use std::fs::{File, create_dir_all};
use std::io::{Read, Write, Seek, SeekFrom};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Clone)]
pub struct CarvedFile {
    pub offset: u64,
    pub filename: String,
    pub extension: String,
    pub category: String,
    pub size_bytes: usize,
    pub sha256: String,
}

const SIGS: &[(&[u8], &str, &str, usize)] = &[
    // (magic, ext, category, max_size)
    (&[0xFF,0xD8,0xFF],             "jpg",  "Images",    15_000_000),
    (&[0x89,0x50,0x4E,0x47],        "png",  "Images",    20_000_000),
    (&[0x47,0x49,0x46,0x38],        "gif",  "Images",     5_000_000),
    (&[0x42,0x4D],                   "bmp",  "Images",    30_000_000),
    (&[0x00,0x00,0x00,0x18,0x66,0x74,0x79,0x70], "mp4", "Videos", 500_000_000),
    (&[0x00,0x00,0x00,0x20,0x66,0x74,0x79,0x70], "mp4", "Videos", 500_000_000),
    (&[0x1A,0x45,0xDF,0xA3],        "mkv",  "Videos",  500_000_000),
    (&[0x25,0x50,0x44,0x46],        "pdf",  "Documents",  50_000_000),
    (&[0x50,0x4B,0x03,0x04],        "docx", "Documents",  50_000_000),
    (&[0xD0,0xCF,0x11,0xE0],        "doc",  "Documents",  50_000_000),
    (&[0x53,0x51,0x4C,0x69],        "db",   "Databases",  200_000_000),
    (&[0x49,0x44,0x33],             "mp3",  "Audio",      50_000_000),
    (&[0x4F,0x67,0x67,0x53],        "ogg",  "Audio",      50_000_000),
    (&[0x52,0x49,0x46,0x46],        "wav",  "Audio",      200_000_000),
    (&[0x66,0x74,0x79,0x70,0x4D,0x34,0x41,0x20], "m4a", "Audio", 50_000_000),
    (&[0x7B,0x5C,0x72,0x74,0x66],   "rtf",  "Documents",  10_000_000),
    (&[0x3C,0x3F,0x78,0x6D,0x6C],   "xml",  "Documents",   5_000_000),
];

#[tauri::command]
pub fn carve_files(image_path: String, output_dir: String, max_files: Option<usize>) -> Result<serde_json::Value, String> {
    let max = max_files.unwrap_or(2000);
    let mut f = File::open(&image_path).map_err(|e| format!("Impossible d'ouvrir {}: {}", image_path, e))?;
    let meta = f.metadata().map_err(|e| e.to_string())?;
    let total = meta.len() as usize;

    create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    // Créer sous-dossiers par catégorie
    for (_, _, cat, _) in SIGS {
        create_dir_all(format!("{}/{}", output_dir, cat)).ok();
    }

    let buf_size = 16 * 1024 * 1024usize; // 16MB
    let overlap = 16usize; // overlap pour sigs sur bords
    let mut window = vec![0u8; buf_size + overlap];
    let mut carved: Vec<CarvedFile> = Vec::new();
    let mut offset = 0usize;
    let mut counters: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    // Pre-remplir overlap
    f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let n0 = f.read(&mut window[overlap..overlap + buf_size.min(total)])
        .map_err(|e| e.to_string())?;

    let mut valid_len = n0;

    while offset < total && carved.len() < max {
        let data = &window[overlap..overlap + valid_len];

        for i in 0..valid_len.saturating_sub(8) {
            for (sig, ext, cat, max_sz) in SIGS {
                if data[i..].starts_with(sig) {
                    let cnt = counters.entry(ext).or_insert(0);
                    *cnt += 1;
                    let filename = format!("{}/{}/carved_{:012x}_{}.{}", output_dir, cat, offset + i, cnt, ext);

                    // Taille approximative: on lit max_sz ou fin du buffer
                    let slice_end = (i + max_sz).min(valid_len);
                    let slice = &data[i..slice_end];

                    // Hash SHA256
                    let mut hasher = Sha256::new();
                    hasher.update(slice);
                    let hash = hex::encode(hasher.finalize());

                    if let Ok(mut out) = File::create(&filename) {
                        out.write_all(slice).ok();
                    }

                    carved.push(CarvedFile {
                        offset: (offset + i) as u64,
                        filename: filename.clone(),
                        extension: ext.to_string(),
                        category: cat.to_string(),
                        size_bytes: slice.len(),
                        sha256: hash,
                    });
                }
            }
        }

        // Avancer la fenêtre
        offset += valid_len.saturating_sub(overlap);
        // Déplacer l'overlap
        let ov_src_start = valid_len.saturating_sub(overlap);
        window.copy_within(overlap + ov_src_start..overlap + valid_len, 0);

        f.seek(SeekFrom::Start(offset as u64)).map_err(|e| e.to_string())?;
        valid_len = f.read(&mut window[overlap..overlap + buf_size])
            .map_err(|e| e.to_string())?;
        if valid_len == 0 { break; }
    }

    // Stats par catégorie
    let mut stats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for c in &carved {
        *stats.entry(c.category.clone()).or_insert(0) += 1;
    }

    Ok(serde_json::json!({
        "total": carved.len(),
        "stats": stats,
        "files": carved,
        "output_dir": output_dir,
    }))
}
