// SD CARD EXTRACTOR mod.rs v2
pub mod sd_extractor;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Partition {
    pub index: u32,
    pub offset: u64,
    pub size_mb: u64,
    pub filesystem: String,
}

#[tauri::command]
pub fn image_sd(path: String, dest: String, case_id: String) -> Result<String, String> {
    sd_extractor::image_sd_card(&path, &dest, &case_id)
}

#[tauri::command]
pub fn carve_sd_card(image_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    sd_extractor::carve_sd(&image_path, &output_dir)
}

#[tauri::command]
pub fn list_partitions(path: String) -> Result<Vec<Partition>, String> {
    sd_extractor::list_partitions(&path)
}
