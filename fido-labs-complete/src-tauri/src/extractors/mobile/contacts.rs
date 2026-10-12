// CONTACTS MODULE v1 - Répertoire téléphonique
// Usage legal uniquement avec autorisation judiciaire.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct ContactEntry {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub phone_numbers: Vec<PhoneNumber>,
    pub email_addresses: Vec<String>,
    pub photo_uri: String,
    pub contact_type: ContactType,
    pub organization: String,
    pub job_title: String,
    pub notes: String,
    pub is_favorite: bool,
    pub is_sim_contact: bool,
    pub source: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PhoneNumber {
    pub number: String,
    pub type_: PhoneType,
    pub is_primary: bool,
    pub is_international: bool,
    pub country_code: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PhoneType {
    Mobile, Home, Work, Fax, Other, Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ContactType {
    Individual, Organization, Family, Unknown,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContactSummary {
    pub total_contacts: usize,
    pub unique_numbers: usize,
    pub with_emails: usize,
    pub with_photos: usize,
    pub by_type: HashMap<String, usize>,
    pub by_organization: HashMap<String, usize>,
    pub top_numbers: Vec<(String, String)>,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ContactResult {
    pub case_id: String,
    pub output_dir: String,
    pub contacts: Vec<ContactEntry>,
    pub summary: ContactSummary,
    pub by_organization: Vec<(String, usize)>,
    pub total_contacts: usize,
    pub total_numbers: usize,
    pub sha256: String,
    pub success: bool,
}

fn classify_phone(number: &str) -> PhoneType {
    let num = number.replace("+", "").replace(" ", "");
    if num.starts_with("06") || num.starts_with("07") || num.starts_with("05") { PhoneType::Mobile }
    else if num.starts_with("01") || num.starts_with("02") || num.starts_with("03") { PhoneType::Home }
    else if num.starts_with("08") || num.starts_with("09") { PhoneType::Work }
    else { PhoneType::Mobile }
}

fn detect_country_code(number: &str) -> String {
    if number.starts_with("+") {
        let cc: String = number.chars().skip(1).take_while(|c| c.is_ascii_digit()).take(3).collect();
        format!("+{}", cc)
    } else { String::new() }
}

fn classify_contact_type(name: &str, org: &str) -> ContactType {
    let n = name.to_lowercase();
    if org.is_empty() { ContactType::Individual }
    else if n.contains("mme") || n.contains("m.") { ContactType::Individual }
    else { ContactType::Organization }
}

#[tauri::command]
pub fn extract_contacts(case_id: String, source_path: String, output_dir: String) -> Result<ContactResult, String> {
    let path = std::path::PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/contacts_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut contacts = Vec::new();
    let text = std::fs::read_to_string(&path).map_err(|_| format!("Impossible de lire {}", source_path))?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("Row:") || line.is_empty() { continue; }
        let mut name = String::new();
        let mut number = String::new();
        let mut email = String::new();
        let mut org = String::new();
        let mut is_favorite = false;
        for field in line.split(',').filter(|f| f.trim().contains('=')) {
            let parts: Vec<&str> = field.splitn(2, '=').collect();
            if parts.len() != 2 { continue; }
            let key = parts[0].trim();
            let val = parts[1].trim();
            match key {
                "display_name" | "name" => name = val.to_string(),
                "data1" | "number" | "address" => number = val.to_string(),
                "data2" => email = val.to_string(),
                "company" | "organization" => org = val.to_string(),
                "starred" => is_favorite = val == "1",
                _ => {}
            }
        }
        if number.is_empty() { continue; }
        let phone_type = classify_phone(&number);
        let cc = detect_country_code(&number);
        let contact_type = classify_contact_type(&name, &org);
        let phone = PhoneNumber {
            number: number.clone(),
            type_: phone_type,
            is_primary: true,
            is_international: number.starts_with('+'),
            country_code: cc,
        };
        let contact = ContactEntry {
            id: Uuid::new_v4().to_string(),
            name: if name.is_empty() { "Inconnu".into() } else { name.clone() },
            display_name: if name.is_empty() { number.clone() } else { name.clone() },
            phone_numbers: vec![phone],
            email_addresses: if email.is_empty() { vec![] } else { vec![email] },
            photo_uri: String::new(),
            contact_type,
            organization: org,
            job_title: String::new(),
            notes: String::new(),
            is_favorite,
            is_sim_contact: false,
            source: source_path.clone(),
            sha256: String::new(),
        };
        contacts.push(contact);
    }
    for c in &mut contacts {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}", c.name, c.phone_numbers.iter().map(|p| p.number.clone()).collect::<Vec<_>>().join(";"), format!("{:?}", c.contact_type), c.organization).as_bytes());
        c.sha256 = hex::encode(hasher.finalize());
    }
    let total_contacts = contacts.len();
    let unique_numbers: std::collections::HashSet<String> = contacts.iter().flat_map(|c| c.phone_numbers.iter().map(|p| p.number.clone())).collect();
    let with_emails = contacts.iter().filter(|c| !c.email_addresses.is_empty()).count();
    let with_photos = contacts.iter().filter(|c| !c.photo_uri.is_empty()).count();
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_org: HashMap<String, usize> = HashMap::new();
    for c in &contacts { *by_type.entry(format!("{:?}", c.contact_type)).or_insert(0) += 1; if !c.organization.is_empty() { *by_org.entry(c.organization.clone()).or_insert(0) += 1; } }
    let mut top_numbers: Vec<(String, String)> = Vec::new();
    for c in &contacts { top_numbers.push((c.name.clone(), c.phone_numbers.iter().map(|p| p.number.clone()).next().unwrap_or_default())); }
    top_numbers.truncate(50);
    let mut by_org_sorted: Vec<(String, usize)> = by_org.iter().map(|(k, v)| (k.clone(), *v)).collect();
    by_org_sorted.sort_by(|a, b| b.1.cmp(&a.1)); by_org_sorted.truncate(20);
    let json = serde_json::to_string_pretty(&contacts).unwrap_or_default();
    std::fs::write(format!("{}/contacts.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    let summary = ContactSummary {
        total_contacts, unique_numbers: unique_numbers.len(), with_emails, with_photos, by_type, by_organization: by_org.clone(),
        top_numbers, sha256: String::new(),
    };
    Ok(ContactResult {
        case_id, output_dir: base, contacts: contacts.iter().take(5000).cloned().collect(), summary,
        by_organization: by_org_sorted, total_contacts, total_numbers: unique_numbers.len(), sha256: sha, success: true,
    })
}

#[tauri::command]
pub fn search_contacts(case_id: String, query: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let result = extract_contacts(case_id, source_path, output_dir)?;
    let q = query.to_lowercase();
    let matches: Vec<ContactEntry> = result.contacts.into_iter().filter(|c| {
        c.name.to_lowercase().contains(&q) || c.display_name.to_lowercase().contains(&q) ||
        c.phone_numbers.iter().any(|p| p.number.contains(&q)) ||
        c.email_addresses.iter().any(|e| e.to_lowercase().contains(&q)) ||
        c.organization.to_lowercase().contains(&q)
    }).collect();
    Ok(serde_json::json!({"success": true, "case_id": result.case_id, "query": query, "matches": matches.len(), "results": matches}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_classify_phone() {
        assert!(matches!(classify_phone("+22670123456"), PhoneType::Mobile));
    }
    #[test] fn test_detect_country_code() {
        assert_eq!(detect_country_code("+22670123456"), "+226");
    }
}
