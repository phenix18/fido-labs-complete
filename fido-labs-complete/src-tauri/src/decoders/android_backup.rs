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

/// Déballe et inventorie une sauvegarde `adb backup` (.ab).
#[tauri::command]
pub fn parse_android_backup(case_id: String, ab_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let data = std::fs::read(&ab_path).map_err(|e| format!("Lecture {}: {}", ab_path, e))?;

    // ── En-tête : 4 lignes ASCII terminées par \n ──
    let mut nl = Vec::new();
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' { nl.push(i); if nl.len() == 4 { break; } }
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
    let payload = &data[nl[3] + 1..];

    if !encryption.is_empty() && encryption.to_lowercase() != "none" {
        return Ok(serde_json::json!({
            "success": false, "encrypted": true, "version": version, "encryption": encryption,
            "summary": format!("Sauvegarde Android CHIFFRÉE ({}) — mot de passe requis. Aucun contournement effectué.", encryption),
        }));
    }

    // ── Décompression zlib → TAR ──
    let tar_bytes: Vec<u8> = if compressed {
        let mut z = flate2::read::ZlibDecoder::new(payload);
        let mut out = Vec::new();
        z.read_to_end(&mut out).map_err(|e| format!("Décompression zlib échouée : {}", e))?;
        out
    } else {
        payload.to_vec()
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
        "success": true, "encrypted": false,
        "summary": format!(
            "Sauvegarde Android déballée — {} fichiers, {} app(s), {} base(s) SQLite ({:.1} Mo)",
            file_count, apps.len(), databases.len(), total_bytes as f64 / 1e6
        ),
        "version": version, "compressed": compressed,
        "output_dir": base, "file_count": file_count, "total_bytes": total_bytes,
        "apps": apps_json, "databases": databases,
        "artifacts": artifacts, "manifest_sha256": manifest_sha256,
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
            "CASE-AB".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string(),
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
    fn signale_sauvegarde_chiffree_sans_contournement() {
        let dir = std::env::temp_dir().join(format!("fido_abenc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let abp = dir.join("enc.ab");
        // En-tête chiffré AES-256 (payload factice) : doit être signalé, pas déchiffré.
        std::fs::write(&abp, b"ANDROID BACKUP\n5\n1\nAES-256\nXXXX").unwrap();
        let r = parse_android_backup("C".into(), abp.to_string_lossy().to_string(), dir.to_string_lossy().to_string())
            .expect("header parse");
        assert_eq!(r["encrypted"], true);
        assert_eq!(r["success"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
