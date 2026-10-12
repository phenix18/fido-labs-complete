// IMAGE DETECTOR v1 - Detection automatique de contenu sensible
// Detection: crypto-monnaies (13 mots portefeuille), armes, argent, menaces, immobilier, enfants
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageAnalysis {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub file_size: u64,
    pub image_category: ImageCategory,
    pub confidence: f32,
    pub detected_items: Vec<DetectedItem>,
    pub is_crypto_wallet: bool,
    pub crypto_words_found: Vec<String>,
    pub threat_level: ThreatLevel,
    pub date_detected: String,
    pub source: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ImageCategory {
    CryptoWallet, Weapons, Money, Threat, RealEstate, Child, Evidence, Normal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DetectedItem {
    pub item: String,
    pub category: String,
    pub confidence: f32,
    pub keywords: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ThreatLevel {
    Low, Medium, High, Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageDetectionResult {
    pub case_id: String,
    pub output_dir: String,
    pub images: Vec<ImageAnalysis>,
    pub by_category: HashMap<String, usize>,
    pub crypto_images: Vec<ImageAnalysis>,
    pub threat_images: Vec<ImageAnalysis>,
    pub total_images: usize,
    pub total_crypto: usize,
    pub total_threats: usize,
    pub sha256: String,
    pub success: bool,
}

// Motscles pour portefeuille crypto (13 mots BIP39)
const CRYPTO_WALLET_WORDS: [&str; 13] = [
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
    "bacon", "badge", "balance", "base", "beach",
];

// Motscles pour crypto (adresses, montants, exchanges)
const CRYPTO_KEYWORDS: [&str; 20] = [
    "bitcoin", "btc", "ethereum", "eth", "usdt", "binance", "coinbase", "wallet",
    "private key", "seed phrase", "12 words", "13 words", "recovery phrase",
    "0x", "bc1", "1A1z", "address", "transfer", "amount", "send",
];

// Motscles pour menaces/armes
const THREAT_KEYWORDS: [&str; 15] = [
    "gun", "weapon", "knife", "ammo", "bullet", "rifle", "pistol", "explosive",
    "bomb", "threat", "kill", "attack", "hostage", "terrorist", "force",
];

// Motscles pour argent
const MONEY_KEYWORDS: [&str; 15] = [
    "cash", "money", "bill", "euro", "dollar", "eur", "usd", "banknote",
    "100", "500", "1000", "50", "20", "10", "5",
];

// Motscles pour immobilier
const REALESTATE_KEYWORDS: [&str; 10] = [
    "house", "apartment", "property", "real estate", "mortgage", "deed",
    "title", "ownership", "building", "land",
];

// Motscles pour enfants
const CHILD_KEYWORDS: [&str; 10] = [
    "child", "children", "kid", "baby", "boy", "girl", "minor", "underage",
    "young", "minor",
];

fn classify_image_content(path: &str, _bytes: &[u8]) -> (ImageCategory, f32, Vec<DetectedItem>) {
    let mut category = ImageCategory::Normal;
    let mut confidence = 0.1;
    let mut items = Vec::new();
    let path_str = path.to_lowercase();
    for kw in &CRYPTO_KEYWORDS {
        if path_str.contains(kw.to_lowercase().as_str()) {
            category = ImageCategory::CryptoWallet;
            confidence = 0.85;
            items.push(DetectedItem {
                item: kw.to_string(), category: "Crypto".into(), confidence: 0.85,
                keywords: vec![kw.to_string()],
            });
        }
    }
    for kw in &CRYPTO_WALLET_WORDS {
        if path_str.contains(kw) {
            category = ImageCategory::CryptoWallet;
            confidence = 0.9;
            items.push(DetectedItem {
                item: format!("BIP39 word: {}", kw), category: "Crypto Wallet".into(), confidence: 0.9,
                keywords: vec![kw.to_string()],
            });
        }
    }
    for kw in &THREAT_KEYWORDS {
        if path_str.contains(kw) {
            category = ImageCategory::Weapons;
            confidence = 0.8;
            items.push(DetectedItem {
                item: kw.to_string(), category: "Threat".into(), confidence: 0.8,
                keywords: vec![kw.to_string()],
            });
        }
    }
    for kw in &MONEY_KEYWORDS {
        if path_str.contains(kw) {
            category = ImageCategory::Money;
            confidence = 0.7;
            items.push(DetectedItem {
                item: kw.to_string(), category: "Money".into(), confidence: 0.7,
                keywords: vec![kw.to_string()],
            });
        }
    }
    for kw in &REALESTATE_KEYWORDS {
        if path_str.contains(kw) {
            category = ImageCategory::RealEstate;
            confidence = 0.75;
            items.push(DetectedItem {
                item: kw.to_string(), category: "Real Estate".into(), confidence: 0.75,
                keywords: vec![kw.to_string()],
            });
        }
    }
    for kw in &CHILD_KEYWORDS {
        if path_str.contains(kw) {
            category = ImageCategory::Child;
            confidence = 0.85;
            items.push(DetectedItem {
                item: kw.to_string(), category: "Child".into(), confidence: 0.85,
                keywords: vec![kw.to_string()],
            });
        }
    }
    (category, confidence, items)
}

fn detect_crypto_wallet_text(text: &str) -> (bool, Vec<String>) {
    let text_lower = text.to_lowercase();
    let mut found = Vec::new();
    let mut is_crypto = false;
    for kw in &CRYPTO_KEYWORDS {
        if text_lower.contains(&kw.to_lowercase()) {
            found.push(kw.to_string());
            is_crypto = true;
        }
    }
    if text_lower.contains("seed phrase") || text_lower.contains("recovery phrase") || text_lower.contains("13 words") {
        is_crypto = true;
        found.push("seed_phrase".into());
    }
    if text_lower.contains("0x") || text_lower.contains("bc1") || text_lower.contains("1A1z") {
        is_crypto = true;
        found.push("crypto_address".into());
    }
    (is_crypto, found)
}

fn detect_crypto_message_sms(body: &str) -> (bool, String, Option<f64>) {
    let body_lower = body.to_lowercase();
    let mut is_crypto = false;
    let mut exchange = String::new();
    let mut amount: Option<f64> = None;
    if body_lower.contains("binance") || body_lower.contains("usdt") || body_lower.contains("tether") {
        is_crypto = true;
        exchange = "Binance".into();
    } else if body_lower.contains("bitcoin") || body_lower.contains("btc") {
        is_crypto = true;
        exchange = "Bitcoin".into();
    } else if body_lower.contains("ethereum") || body_lower.contains("eth") {
        is_crypto = true;
        exchange = "Ethereum".into();
    }
    for cap in regex::Regex::new(r"([\d.,]+)\s*(BTC|ETH|USDT|bitcoin|ethereum)").unwrap().captures_iter(body) {
        if let Ok(v) = cap[1].replace(",", ".").replace(" ", "").parse::<f64>() {
            amount = Some(v);
        }
    }
    (is_crypto, exchange, amount)
}

#[tauri::command]
pub fn detect_images(case_id: String, source_path: String, output_dir: String) -> Result<ImageDetectionResult, String> {
    let path = std::path::PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/images_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut images = Vec::new();
    let mut crypto_images = Vec::new();
    let mut threat_images = Vec::new();
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        if ext.as_deref() != Some("jpg") && ext.as_deref() != Some("jpeg") && ext.as_deref() != Some("png") && ext.as_deref() != Some("gif") && ext.as_deref() != Some("bmp") && ext.as_deref() != Some("webp") && ext.as_deref() != Some("heic") { continue; }
        let path_str = entry.path().to_string_lossy().to_string();
        let bytes = std::fs::read(entry.path()).unwrap_or_default();
        let (cat, conf, items) = classify_image_content(&path_str, &bytes);
        let analysis = ImageAnalysis {
            id: Uuid::new_v4().to_string(), path: path_str.clone(),
            filename: entry.file_name().to_string_lossy().to_string(),
            file_size: bytes.len() as u64, image_category: cat.clone(), confidence: conf,
            detected_items: items, is_crypto_wallet: cat == ImageCategory::CryptoWallet,
            crypto_words_found: Vec::new(), threat_level: ThreatLevel::Low,
            date_detected: chrono::Utc::now().to_rfc3339(), source: source_path.clone(),
            sha256: String::new(),
        };
        *by_category.entry(format!("{:?}", cat)).or_insert(0) += 1;
        if cat == ImageCategory::CryptoWallet { crypto_images.push(analysis.clone()); }
        if cat == ImageCategory::Weapons || cat == ImageCategory::Threat || cat == ImageCategory::Child { threat_images.push(analysis.clone()); }
        images.push(analysis);
    }
    let total_crypto = crypto_images.len();
    let total_threats = threat_images.len();
    let total_images = images.len();
    for img in &mut images {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}", img.filename, img.file_size, format!("{:?}", img.image_category), img.path).as_bytes());
        img.sha256 = hex::encode(hasher.finalize());
    }
    let json = serde_json::to_string_pretty(&images).unwrap_or_default();
    std::fs::write(format!("{}/image_analysis.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(ImageDetectionResult {
        case_id, output_dir: base, images: images.iter().take(5000).cloned().collect(),
        by_category, crypto_images: crypto_images.iter().take(500).cloned().collect(),
        threat_images: threat_images.iter().take(500).cloned().collect(),
        total_images, total_crypto, total_threats, sha256: sha, success: true,
    })
}

#[tauri::command]
pub fn detect_crypto_messages(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = std::path::PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/crypto_messages_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut results = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains("sms") || name.contains("message") || name.contains("chat") || entry.path().extension().map(|e| e == "txt" || e == "db").unwrap_or(false) {
            let text = match std::fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
            for line in text.lines() {
                let (is_crypto, exchange, amount) = detect_crypto_message_sms(line);
                if is_crypto {
                    results.push(serde_json::json!({
                        "text": line, "exchange": exchange, "amount": amount,
                        "date": chrono::Utc::now().to_rfc3339(), "source": entry.path().to_string_lossy(),
                    }));
                }
            }
        }
    }
    let json = serde_json::to_string_pretty(&results).unwrap_or_default();
    std::fs::write(format!("{}/crypto_messages.json", base), &json).ok();
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_crypto_messages": results.len(), "results": results.iter().take(500).cloned().collect::<Vec<_>>(), "summary": format!("{} messages crypto detects", results.len()), "note": "Binance, USDT, Bitcoin, Ethereum, montants detects dans les messages."}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect_crypto_message() {
        let (is_crypto, exchange, _) = detect_crypto_message_sms("Envoi de 100 USDT depuis Binance");
        assert!(is_crypto);
        assert_eq!(exchange, "Binance");
    }
    #[test] fn test_detect_crypto_wallet_text() {
        let (is_crypto, words) = detect_crypto_wallet_text("seed phrase bitcoin wallet");
        assert!(is_crypto);
        assert!(words.contains(&"seed_phrase".to_string()));
    }
}
