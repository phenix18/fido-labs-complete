// PASSWORD EXTRACTOR v1 - Extraction de mots de passe
// Usage legal uniquement avec autorisation judiciaire.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct ExtractedCredential {
    pub id: String,
    pub service: String,
    pub account: String,
    pub credential_type: CredentialType,
    pub value: String,
    pub category: CredentialCategory,
    pub domain: String,
    pub source: String,
    pub timestamp: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CredentialType {
    Password, Token, ApiKey, SessionToken, BiometricTemplate, PinCode, EncryptionKey,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CredentialCategory {
    SocialMedia, Email, Banking, Messaging, Shopping, System, Browser, Custom,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CredentialExtractionResult {
    pub case_id: String,
    pub output_dir: String,
    pub credentials: Vec<ExtractedCredential>,
    pub by_category: HashMap<String, usize>,
    pub total_credentials: usize,
    pub sha256: String,
    pub success: bool,
}

fn parse_android_passwords_xml(path: &std::path::Path) -> Vec<ExtractedCredential> {
    let mut creds = Vec::new();
    let text = match fs::read_to_string(path) { Ok(t) => t, Err(_) => return creds };
    for cap in regex::Regex::new(r#"<account name="([^"]+)" type="([^"]+)" password="([^"]+)""#).unwrap().captures_iter(&text) {
        creds.push(ExtractedCredential {
            id: Uuid::new_v4().to_string(),
            service: cap[2].to_string(),
            account: cap[1].to_string(),
            credential_type: CredentialType::Password,
            value: cap[3].to_string(),
            category: categorize_service(&cap[2]),
            domain: "android".into(),
            source: path.to_string_lossy().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            sha256: String::new(),
        });
    }
    for c in &mut creds {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}", c.service, c.account, c.value).as_bytes());
        c.sha256 = hex::encode(hasher.finalize());
    }
    creds
}

fn categorize_service(service: &str) -> CredentialCategory {
    let s = service.to_lowercase();
    if s.contains("gmail") || s.contains("email") || s.contains("outlook") { CredentialCategory::Email }
    else if s.contains("whatsapp") || s.contains("telegram") || s.contains("signal") { CredentialCategory::Messaging }
    else if s.contains("instagram") || s.contains("facebook") || s.contains("twitter") { CredentialCategory::SocialMedia }
    else if s.contains("bank") || s.contains("paypal") || s.contains("pay") { CredentialCategory::Banking }
    else if s.contains("chrome") || s.contains("firefox") || s.contains("edge") { CredentialCategory::Browser }
    else { CredentialCategory::Custom }
}

#[tauri::command]
pub fn extract_passwords(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/passwords_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut credentials = Vec::new();
    let passwords_xml = path.join("passwords.xml");
    if passwords_xml.exists() { credentials.extend(parse_android_passwords_xml(&passwords_xml)); }
    let prefs = path.join("shared_prefs");
    if prefs.exists() {
        for entry in walkdir::WalkDir::new(&prefs).into_iter().flatten() {
            if entry.path().extension().map(|e| e == "xml").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                if text.contains("password") || text.contains("token") {
                    for cap in regex::Regex::new(r#"(?i)(password|token|secret)"[^>]*>([^<]+)"#).unwrap().captures_iter(&text) {
                        credentials.push(ExtractedCredential {
                            id: Uuid::new_v4().to_string(), service: "SharedPreferences".into(),
                            account: cap[1].to_lowercase(), credential_type: CredentialType::Token,
                            value: cap[2].to_string(), category: CredentialCategory::System,
                            domain: entry.path().to_string_lossy().to_string(),
                            source: entry.path().to_string_lossy().to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(), sha256: String::new(),
                        });
                    }
                }
            }
        }
    }
    for c in &mut credentials {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}", c.service, c.account, format!("{:?}", c.category), c.value).as_bytes());
        c.sha256 = hex::encode(hasher.finalize());
    }
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for c in &credentials { *by_category.entry(format!("{:?}", c.category)).or_insert(0) += 1; }
    let json = serde_json::to_string_pretty(&credentials).unwrap_or_default();
    fs::write(format!("{}/credentials.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_credentials": credentials.len(), "sha256": sha, "by_category": by_category, "credentials": credentials.iter().take(500).cloned().collect::<Vec<_>>(), "summary": format!("{} credentials extraits - {} categories", credentials.len(), by_category.len()), "note": "Manifeste : credentials.json"}))
}

#[tauri::command]
pub fn extract_ios_keychain(case_id: String, backup_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&backup_path);
    if !path.exists() { return Err(format!("Backup iOS introuvable : {}", backup_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/ios_keychain_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut credentials = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(3).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains("keychain") || name.contains("keybag") || entry.path().extension().map(|e| e == "db").unwrap_or(false) {
            let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
            if text.contains("akv") || text.contains("genp") || text.contains("inet") {
                for cap in regex::Regex::new(r#"(?i)(password|secret|token)[^:]*[:=]\s*([a-zA-Z0-9+/=]{10,})"#).unwrap().captures_iter(&text) {
                    credentials.push(ExtractedCredential {
                        id: Uuid::new_v4().to_string(), service: "iOS Keychain".into(),
                        account: format!("keychain_entry_{}", credentials.len()),
                        credential_type: CredentialType::EncryptionKey, value: cap[2].to_string(),
                        category: CredentialCategory::System,
                        domain: entry.path().to_string_lossy().to_string(),
                        source: entry.path().to_string_lossy().to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(), sha256: String::new(),
                    });
                }
            }
        }
    }
    for c in &mut credentials {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}", c.service, c.account, c.value).as_bytes());
        c.sha256 = hex::encode(hasher.finalize());
    }
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for c in &credentials { *by_category.entry(format!("{:?}", c.category)).or_insert(0) += 1; }
    let json = serde_json::to_string_pretty(&credentials).unwrap_or_default();
    fs::write(format!("{}/ios_keychain.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_credentials": credentials.len(), "sha256": sha, "by_category": by_category, "credentials": credentials.iter().take(500).cloned().collect::<Vec<_>>(), "summary": format!("{} credentials iOS Keychain extraits", credentials.len()), "note": "Chaine de garde : ios_keychain.json"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_categorize() {
        assert!(matches!(categorize_service("com.google.android.gmail"), CredentialCategory::Email));
        assert!(matches!(categorize_service("com.whatsapp"), CredentialCategory::Messaging));
        assert!(matches!(categorize_service("com.instagram.android"), CredentialCategory::SocialMedia));
    }
}
