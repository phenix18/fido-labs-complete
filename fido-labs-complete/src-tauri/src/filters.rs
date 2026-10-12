// FILTERS MODULE - Filtres avancés multi-critères
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Mutex;
use sha2::{Sha256, Digest};
use hex;

#[derive(Serialize, Deserialize, Clone)]
pub struct AdvancedFilter {
    pub id: String,
    pub name: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub operators: Vec<String>,
    pub device_ids: Vec<String>,
    pub categories: Vec<String>,
    pub severities: Vec<String>,
    pub keywords: Vec<String>,
    pub exclude_deleted: bool,
    pub case_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FilterResult {
    pub filter_id: String,
    pub total_matched: usize,
    pub items: Vec<FilterableItem>,
    pub execution_time_ms: u128,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FilterableItem {
    pub id: String,
    pub timestamp: String,
    pub category: String,
    pub operator: String,
    pub device_id: String,
    pub content: String,
    pub severity: String,
    pub metadata: serde_json::Value,
}

static FILTERS: Mutex<Vec<AdvancedFilter>> = Mutex::new(Vec::new());
static FILTERED_ITEMS: Mutex<Vec<FilterableItem>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn create_filter(
    name: String,
    date_from: Option<String>,
    date_to: Option<String>,
    operators: Vec<String>,
    device_ids: Vec<String>,
    categories: Vec<String>,
    severities: Vec<String>,
    keywords: Vec<String>,
    exclude_deleted: bool,
    case_id: String,
) -> Result<AdvancedFilter, String> {
    let f = AdvancedFilter {
        id: Uuid::new_v4().to_string(),
        name, date_from, date_to,
        operators, device_ids, categories, severities, keywords,
        exclude_deleted, case_id,
    };
    FILTERS.lock().unwrap().push(f.clone());
    Ok(f)
}

#[tauri::command]
pub fn list_filters(case_id: String) -> Result<Vec<AdvancedFilter>, String> {
    let db = FILTERS.lock().unwrap();
    Ok(db.iter().filter(|f| case_id.is_empty() || f.case_id == case_id).cloned().collect())
}

#[tauri::command]
pub fn apply_filter(filter_id: String) -> Result<FilterResult, String> {
    use chrono::Utc;
    let start = Utc::now();
    let filters = FILTERS.lock().unwrap();
    let items_db = FILTERED_ITEMS.lock().unwrap();

    let filter = filters.iter().find(|f| f.id == filter_id)
        .ok_or_else(|| "Filtre non trouvé".to_string())?;

    let items: Vec<FilterableItem> = items_db.iter()
        .filter(|item| {
            if let Some(ref df) = filter.date_from {
                if item.timestamp < *df { return false; }
            }
            if let Some(ref dt) = filter.date_to {
                if item.timestamp > *dt { return false; }
            }
            if !filter.operators.is_empty() && !filter.operators.iter().any(|o| item.operator.contains(o.as_str())) {
                return false;
            }
            if !filter.categories.is_empty() && !filter.categories.iter().any(|c| item.category.contains(c.as_str())) {
                return false;
            }
            if !filter.keywords.is_empty() && !filter.keywords.iter().any(|k| item.content.to_lowercase().contains(&k.to_lowercase())) {
                return false;
            }
            true
        })
        .cloned().collect();

    let elapsed = Utc::now().signed_duration_since(start).num_milliseconds() as u128;
    Ok(FilterResult {
        filter_id,
        total_matched: items.len(),
        items,
        execution_time_ms: elapsed,
    })
}

#[tauri::command]
pub fn save_filter_as_template(filter_id: String, name: String) -> Result<String, String> {
    let mut db = FILTERS.lock().unwrap();
    let f = db.iter_mut().find(|x| x.id == filter_id)
        .ok_or_else(|| "Filtre non trouvé".to_string())?;
    f.name = name;
    Ok("Template sauvegardé".into())
}

/// Supprime toutes les données de démonstration.
#[tauri::command]
pub fn clear_demo_data() {
    let mut db = FILTERED_ITEMS.lock().unwrap();
    db.clear();
    let mut fb = FILTERS.lock().unwrap();
    fb.clear();
}

/// Sauvegarde les données d'extraction dans un fichier JSON local avec chaîne de garde.
#[tauri::command]
pub fn save_extracted_to_file(data: String, output_path: String) -> Result<String, String> {
    let json = data;
    std::fs::create_dir_all(std::path::Path::new(&output_path).parent().unwrap())
        .map_err(|e| format!("Création dossier : {}", e))?;
    std::fs::write(&output_path, &json).map_err(|e| format!("Écriture : {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&json);
    let sha = hex::encode(hasher.finalize());
    Ok(format!("Fichier sauvegardé : {} (SHA256: {})", output_path, sha))
}

/// Seed items de démonstration
pub fn seed_filtered_items() {
    let mut db = FILTERED_ITEMS.lock().unwrap();
    if !db.is_empty() { return; }
    *db = (0..50).map(|i| FilterableItem {
        id: Uuid::new_v4().to_string(),
        timestamp: format!("2024-03-{:02}T{:02}:00:00Z", (i % 28) + 1, i % 24),
        category: ["MESSAGE","CALL","TRANSACTION","FILE","APP","LOCATION"][i % 6].into(),
        operator: format!("Inspecteur {}", (i % 4) + 1),
        device_id: format!("DEV-{:03}", (i % 5) + 1),
        content: format!("Événement forensique #{} — données analysées", i),
        severity: ["LOW","NORMAL","HIGH","CRITICAL"][i % 4].into(),
        metadata: serde_json::json!({"case_id": format!("case-{}", i % 3)}),
    }).collect();
}
