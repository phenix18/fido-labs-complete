// DISK EXTRACTOR mod.rs v2
pub mod disk_imager;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DiskInfo {
    pub path: String,
    pub label: String,
    pub size_gb: f64,
    pub filesystem: String,
    pub removable: bool,
}

#[tauri::command]
pub fn list_disks() -> Result<Vec<DiskInfo>, String> {
    disk_imager::enumerate_disks()
}

#[tauri::command]
pub fn create_image(disk_path: String, dest_path: String, format: String, case_id: String) -> Result<String, String> {
    disk_imager::create_forensic_image(&disk_path, &dest_path, &format, &case_id)
}

#[tauri::command]
pub fn verify_image(image_path: String) -> Result<String, String> {
    disk_imager::verify_image_hash(&image_path)
}
