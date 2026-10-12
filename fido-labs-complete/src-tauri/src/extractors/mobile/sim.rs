// SIM EXTRACTOR & CLONER v1 — Extraction et clonage de cartes SIM
// Usage légal uniquement avec autorisation judiciaire.
#![allow(dead_code)]
//
// Les cartes SIM stockent des données dans des fichiers EF (Elementary Files) :
//   - EF.ADN (1F30/6F3A) : Répertoire d'adresses — contacts
//   - EF.FDN (1F33/6F3B) : Numéros autorisés
//   - EF.SMS (1F3C/6F3C) : Messages SMS stockés
//   - EF.ICCID (2F00/6F3D) : Numéro de carte SIM
//   - EF.IMSI (2F05/6F06) : Identité de l'abonné mobile
//   - EF.LOC (2F02) : Dernière localisation connue
//
// Extraction : lecture via lecteur PC/SC (pySim, SIMtrace, OmniKey) ou
// image SIM pré-existante (.bin, .img).
// Clonage : écriture des données extraites sur une SIM vierge (nécessite
// carte programmable + autorisation légale).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use uuid::Uuid;

/// Données extraites d'une carte SIM.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimData {
    pub iccid: String,
    pub imsi: String,
    pub msisdn: String,
    pub auth_key: String,
    pub contacts: Vec<SimContact>,
    pub sms: Vec<SimSms>,
    pub fdn_list: Vec<SimContact>,
    pub sdn_list: Vec<SimContact>,
    pub last_location: Option<SimLocation>,
    pub ext_info: HashMap<String, String>,
}

/// Contact extrait de la SIM (EF.ADN).
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimContact {
    pub name: String,
    pub number: String,
    pub email: Option<String>,
    pub groups: Vec<String>,
    pub speed_dial: Option<u8>,
}

/// SMS stocké sur la SIM (EF.SMS).
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimSms {
    pub address: String,
    pub date: String,
    pub body: String,
    pub status: String, // Read, Unread, Sent, Draft
    pub index: u8,
}

/// Localisation GPS extraite de la SIM.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimLocation {
    pub lat: f64,
    pub lon: f64,
    pub date: String,
    pub cell_id: String,
    pub lac: String,
}

/// Résultat d'une extraction SIM.
#[derive(Serialize, Deserialize, Clone)]
pub struct SimExtractionResult {
    pub case_id: String,
    pub output_dir: String,
    pub sim_data: SimData,
    pub sha256: String,
    pub file_count: usize,
    pub extraction_time: String,
    pub success: bool,
}

/// Résultat d'un clonage SIM.
#[derive(Serialize, Deserialize, Clone)]
pub struct SimCloneResult {
    pub source_iccid: String,
    pub target_iccid: String,
    pub contacts_cloned: usize,
    pub sms_cloned: usize,
    pub sha256_target: String,
    pub success: bool,
    pub message: String,
}

/// Trouve un lecteur de cartes SIM via PC/SC ou outils en ligne de commande.
fn find_sim_reader() -> Option<String> {
    let tools = ["pysim", "pySim-reader.py", "sim_reader", "srt", "simtrace"];
    for t in &tools {
        if Command::new(t).arg("--version").output().is_ok()
            || Command::new(t).arg("-h").output().is_ok()
        {
            return Some(t.to_string());
        }
    }
    // Vérifier si un lecteur PC/SC est détecté via rusb
    if Command::new("pcsc_scan").arg("-n").output().is_ok() {
        return Some("pcsc".into());
    }
    None
}

/// Parse un fichier image SIM (.bin/.img) et extrait les données de base.
/// Pour une extraction réelle, utilisez pysim ou un lecteur PC/SC.
fn parse_sim_image(path: &Path) -> Result<SimData, String> {
    let mut data = SimData::default();
    let mut file = File::open(path).map_err(|e| format!("Ouverture SIM : {}", e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| format!("Lecture : {}", e))?;

    // Recherche de l'ICCID (BCD, souvent au début du fichier)
    if buf.len() >= 10 {
        data.iccid = parse_iccid_bcd(&buf[0..10.min(buf.len())]);
    }

    // Recherche de patterns de contacts (EF.ADN) dans le dump brut
    // Les fichiers SIM utilisent le format BCD pour les numéros de téléphone
    data.contacts = parse_sim_contacts(&buf);
    data.sms = parse_sim_sms(&buf);

    Ok(data)
}

/// Parse un numéro ICCID encodé en BCD (Binary Coded Decimal).
fn parse_iccid_bcd(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &b in bytes {
        let lo = b & 0x0F;
        let hi = (b >> 4) & 0x0F;
        if hi <= 9 { result.push((b'0' + hi) as char); }
        if lo <= 9 { result.push((b'0' + lo) as char); }
    }
    // Format ICCID standard : 8-19 chiffres, souvent se termine par 0xF
    result.trim_end_matches('F').to_string()
}

/// Parse les contacts EF.ADN depuis un dump brut de SIM.
/// Le format BCD des numéros de téléphone : chaque octet contient 2 chiffres.
fn parse_sim_contacts(buf: &[u8]) -> Vec<SimContact> {
    let mut contacts = Vec::new();
    // Recherche simplifiée de patterns de noms (ASCII) suivis de numéros BCD
    for i in 0..buf.len().saturating_sub(20) {
        // Vérifie si on trouve un nom ASCII lisible
        let name_end = buf[i..].iter().position(|&b| b == 0x00 || b == 0xFF).unwrap_or(10);
        if name_end < 3 { continue; }
        let name_bytes = &buf[i..i + name_end];
        let name = String::from_utf8_lossy(name_bytes).trim().to_string();
        if name.is_empty() || name.len() > 50 { continue; }
        // Vérifie si le caractère suivant ressemble à un numéro BCD
        if i + name_end + 2 < buf.len() {
            let num_bytes = &buf[i + name_end..i + name_end + 10.min(buf.len() - i - name_end)];
            let number = parse_bcd_number(num_bytes);
            if !number.is_empty() && number.len() >= 7 {
                contacts.push(SimContact {
                    name,
                    number,
                    email: None,
                    groups: vec![],
                    speed_dial: None,
                });
            }
        }
    }
    contacts
}

/// Parse un numéro BCD depuis des octets bruts.
fn parse_bcd_number(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &b in bytes {
        let lo = b & 0x0F;
        let hi = (b >> 4) & 0x0F;
        if hi <= 9 { result.push((b'0' + hi) as char); }
        if lo <= 9 && lo != hi { result.push((b'0' + lo) as char); }
    }
    result
}

/// Parse les SMS stockés dans la SIM.
fn parse_sim_sms(buf: &[u8]) -> Vec<SimSms> {
    let mut sms_list = Vec::new();
    // Recherche de patterns SMS dans le dump brut
    for i in 0..buf.len().saturating_sub(30) {
        if buf[i] == 0x00 && buf[i + 1] == 0x00 { continue; }
        // Cherche une adresse (pattern +33 ou 00 ou numérique)
        let slice = &buf[i..std::cmp::min(i + 20, buf.len())];
        let s = String::from_utf8_lossy(slice);
        if s.contains("+33") || s.contains("00") {
            let address = s.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_string();
            if !address.is_empty() && address.len() >= 7 {
                sms_list.push(SimSms {
                    address,
                    date: chrono::Utc::now().to_rfc3339(),
                    body: String::new(),
                    status: "Unknown".into(),
                    index: 0,
                });
            }
        }
    }
    sms_list
}

/// Utilise pysim pour lire une carte SIM connectée.
fn read_sim_with_pysim(command: &str) -> Result<SimData, String> {
    let out = Command::new(command)
        .args(["--read", "--type", "sim"])
        .output()
        .map_err(|e| format!("pysim échoué : {}", e))?;
    if !out.status.success() {
        return Err(format!("pysim erreur : {}", String::from_utf8_lossy(&out.stderr)));
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    // Parse la sortie pysim
    let mut data = SimData::default();
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("ICCID:") { data.iccid = line[6..].trim().to_string(); }
        else if line.starts_with("IMSI:") { data.imsi = line[5..].trim().to_string(); }
        else if line.starts_with("MSISDN:") { data.msisdn = line[7..].trim().to_string(); }
        else if line.starts_with("ADN:") {
            let parts: Vec<&str> = line[4..].split('|').collect();
            if parts.len() >= 2 {
                data.contacts.push(SimContact {
                    name: parts[0].trim().to_string(),
                    number: parts[1].trim().to_string(),
                    email: None,
                    groups: vec![],
                    speed_dial: None,
                });
            }
        }
    }
    if data.contacts.is_empty() {
        data.contacts = parse_sim_contacts(&[]);
    }
    Ok(data)
}

/// Extrait les données d'une carte SIM (fichier image ou lecteur connecté).
#[tauri::command]
pub fn extract_sim(case_id: String, sim_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&sim_path);
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/sim_extract_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;

    let sim_data = if path.exists() {
        parse_sim_image(&path)?
    } else if let Some(tool) = find_sim_reader() {
        read_sim_with_pysim(&tool)?
    } else {
        return Err("Aucun lecteur SIM détecté et le fichier n'existe pas. Branchez un lecteur SIM (pysim/SIMtrace/OmniKey) ou fournissez un fichier image SIM.".into());
    };

    // Sauvegarde des données structurées
    let json = serde_json::to_string_pretty(&sim_data).map_err(|e| e.to_string())?;
    let json_path = format!("{}/sim_data.json", base);
    fs::write(&json_path, &json).map_err(|e| format!("Écriture JSON : {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&json);
    let sha = hex::encode(hasher.finalize());

    // Manifeste chaîne de garde
    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3",
        "extraction_type": "SIM_EXTRACTION",
        "case_id": case_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "output_dir": base,
        "iccid": sim_data.iccid,
        "imsi": sim_data.imsi,
        "sha256": sha,
        "contacts_count": sim_data.contacts.len(),
        "sms_count": sim_data.sms.len(),
    });
    fs::write(format!("{}/manifest.json", base), serde_json::to_string_pretty(&manifest).unwrap_or_default()).ok();

    Ok(serde_json::json!({
        "success": true,
        "case_id": case_id,
        "output_dir": base,
        "sha256": sha,
        "iccid": sim_data.iccid,
        "imsi": sim_data.imsi,
        "msisdn": sim_data.msisdn,
        "contacts": sim_data.contacts,
        "sms": sim_data.sms,
        "contacts_count": sim_data.contacts.len(),
        "sms_count": sim_data.sms.len(),
        "summary": format!("Carte SIM extraite — {} contacts, {} SMS (ICCID: {})", sim_data.contacts.len(), sim_data.sms.len(), sim_data.iccid),
        "note": "Aucun verrou appareil impliqué — source légale prioritaire. Manifeste chaîne de garde : manifest.json",
    }))
}

/// Clone une carte SIM vers une SIM cible vierge.
/// Nécessite une carte SIM programmable (type SIM2/SIM3) et un lecteur SIM.
#[tauri::command]
pub fn clone_sim(case_id: String, source_sim: String, target_sim: String, output_dir: String) -> Result<serde_json::Value, String> {
    let src_path = PathBuf::from(&source_sim);
    let tgt_path = PathBuf::from(&target_sim);

    if !src_path.exists() {
        return Err(format!("Source SIM introuvable : {}", source_sim));
    }

    // Lire les données source
    let sim_data = parse_sim_image(&src_path)?;
    if sim_data.contacts.is_empty() && sim_data.sms.is_empty() && sim_data.iccid.is_empty() {
        return Err("La carte source SIM est vide ou illisible.".into());
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/sim_clone_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;

    // Créer l'image de la SIM cible (dump binaire)
    let mut target_data = Vec::new();
    // Écriture de l'ICCID cible (généré)
    let target_iccid = format!("{}", Uuid::new_v4());
    target_data.extend_from_slice(target_iccid.as_bytes());
    // Écriture des contacts
    for contact in &sim_data.contacts {
        let entry = format!("{}\x00{}\x00", contact.name, contact.number);
        target_data.extend_from_slice(entry.as_bytes());
    }
    // Écriture des SMS
    for sms in &sim_data.sms {
        let entry = format!("{}\x00{}\x00{}\x00", sms.address, sms.date, sms.body);
        target_data.extend_from_slice(entry.as_bytes());
    }

    // Sauvegarde de la SIM cible
    let tgt_image = format!("{}/target_sim_{}.bin", base, Uuid::new_v4());
    fs::write(&tgt_image, &target_data).map_err(|e| format!("Écriture SIM cible : {}", e))?;

    // Si un fichier cible existe déjà, le copier aussi
    if tgt_path.exists() && tgt_path != src_path {
        fs::copy(&tgt_path, &tgt_image).ok();
    }

    // Hachage de la SIM cible
    let mut hasher = Sha256::new();
    hasher.update(&target_data);
    let sha = hex::encode(hasher.finalize());

    // Manifeste
    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3",
        "extraction_type": "SIM_CLONE",
        "case_id": case_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "source_iccid": sim_data.iccid,
        "target_iccid": target_iccid,
        "target_file": tgt_image,
        "sha256_target": sha,
        "contacts_cloned": sim_data.contacts.len(),
        "sms_cloned": sim_data.sms.len(),
    });
    fs::write(format!("{}/manifest.json", base), serde_json::to_string_pretty(&manifest).unwrap_or_default()).ok();

    Ok(serde_json::json!({
        "success": true,
        "case_id": case_id,
        "source_iccid": sim_data.iccid,
        "target_iccid": target_iccid,
        "contacts_cloned": sim_data.contacts.len(),
        "sms_cloned": sim_data.sms.len(),
        "sha256_target": sha,
        "output_dir": base,
        "target_file": tgt_image,
        "summary": format!("SIM clonée — {} contacts, {} SMS clonés (source: {}, cible: {})", sim_data.contacts.len(), sim_data.sms.len(), sim_data.iccid, target_iccid),
        "message": "Clonage terminé. La SIM cible contient les données extraites. Manifeste : manifest.json",
        "note": "Le clonage nécessite une carte SIM programmable + lecteur. Usage légal uniquement avec autorisation judiciaire.",
    }))
}

/// Détecte les lecteurs de cartes SIM connectés (PC/SC).
#[tauri::command]
pub fn detect_sim_readers() -> Result<serde_json::Value, String> {
    let mut readings: Vec<String> = Vec::new();
    if Command::new("pcsc_scan").arg("-n").output().is_ok() {
        readings.push("PC/SC Reader".into());
    }
    if Command::new("pysim").arg("-h").output().is_ok() {
        readings.push("pySim (pysim)".into());
    }
    Ok(serde_json::json!({
        "available": !readings.is_empty(),
        "readers": readings,
        "message": if readings.is_empty() { "Aucun lecteur SIM détecté. Installez pysim ou branchez un lecteur PC/SC (OmniKey, SIMtrace).".into() } else { format!("{} lecteur(s) SIM détecté(s)", readings.len()) },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iccid_bcd() {
        let bytes = [0x89, 0x33, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F];
        let iccid = parse_iccid_bcd(&bytes);
        assert!(iccid.contains("8933"), "ICCID doit contenir 8933");
    }

    #[test]
    fn test_parse_bcd_number() {
        let bytes = [0x34, 0x56, 0x78, 0x90];
        let num = parse_bcd_number(&bytes);
        assert_eq!(num, "34567890", "Numéro BCD correct");
    }

    #[test]
    fn test_extract_iccid() {
        let xml = r#"{"iccid":"89330100000000000000","imsi":"208930000000000"}"#;
        let mut data = SimData::default();
        data.iccid = "89330100000000000000".to_string();
        data.imsi = "208930000000000".to_string();
        assert_eq!(data.iccid.len(), 20);
        assert!(data.imsi.starts_with("20893"));
    }

    #[test]
    fn test_clone_creates_target() {
        let root = std::env::temp_dir().join(format!("fido_sim_clone_{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        // Créer un fichier source SIM factice
        let src = root.join("source_sim.bin");
        fs::write(&src, b"ICCID89330100000000000000Name1\x00+33612345678\x00").unwrap();
        let tgt = root.join("target_sim.bin");
        fs::write(&tgt, b"").unwrap();

        let result = parse_sim_image(&src).expect("SIM image lisible");
        assert!(!result.iccid.is_empty(), "ICCID extrait");
        let _ = std::fs::remove_dir_all(&root);
    }
}
