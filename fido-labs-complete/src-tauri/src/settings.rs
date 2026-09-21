// SETTINGS MODULE - Réglages application, affichage, modules, exports, sessions
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

// ─── PARAMÈTRES GLOBAUX ─────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    // Affichage
    pub theme: String,           // "dark" | "light" | "system"
    pub language: String,        // "fr" | "en"
    pub font_size: u8,           // 11-18
    pub compact_mode: bool,      // UI compacte
    pub show_sidebar_icons: bool,
    pub animations_enabled: bool,

    // Chemins par défaut
    pub default_cases_dir: String,
    pub default_export_dir: String,
    pub default_backup_dir: String,

    // Comportement
    pub auto_backup_interval_minutes: u32,
    pub confirm_destructive: bool,
    pub log_to_file: bool,
    pub max_log_size_mb: u32,

    // Opérateur par défaut
    pub default_operator: String,
    pub default_organization: String,

    // Fenêtre
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,
    pub remember_window_pos: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: "dark".into(),
            language: "fr".into(),
            font_size: 13,
            compact_mode: false,
            show_sidebar_icons: true,
            animations_enabled: true,
            default_cases_dir: "C:\\FIDO-LABS\\Cases".into(),
            default_export_dir: "C:\\FIDO-LABS\\Exports".into(),
            default_backup_dir: "C:\\FIDO-LABS\\Backups".into(),
            auto_backup_interval_minutes: 30,
            confirm_destructive: true,
            log_to_file: true,
            max_log_size_mb: 100,
            default_operator: "Investigateur".into(),
            default_organization: "".into(),
            window_width: 1280,
            window_height: 900,
            window_maximized: false,
            remember_window_pos: true,
        }
    }
}

// ─── PARAMÈTRES PAR MODULE ──────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ModuleSettings {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub settings: serde_json::Value,  // JSON blob pour params spécifiques
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AllModulesSettings {
    pub mobile: ModuleSettings,
    pub sdcard: ModuleSettings,
    pub disk: ModuleSettings,
    pub whatsapp: ModuleSettings,
    pub money_flow: ModuleSettings,
    pub timeline: ModuleSettings,
    pub csm: ModuleSettings,
    pub cartography: ModuleSettings,
    pub filters: ModuleSettings,
    pub reports: ModuleSettings,
}

impl Default for AllModulesSettings {
    fn default() -> Self {
        AllModulesSettings {
            mobile: ModuleSettings {
                id: "mobile".into(),
                name: "Extraction Mobile".into(),
                enabled: true,
                settings: serde_json::json!({
                    "adb_path": "adb",
                    "fastboot_path": "fastboot",
                    "scrcpy_enabled": true,
                    "agent_apk_auto_install": true,
                }),
            },
            sdcard: ModuleSettings {
                id: "sdcard".into(),
                name: "Carte SD".into(),
                enabled: true,
                settings: serde_json::json!({
                    "auto_verify_hash": true,
                    "carving_extensions": ["jpg","png","mp4","pdf","docx","db"],
                    "carving_max_size_mb": 5000,
                }),
            },
            disk: ModuleSettings {
                id: "disk".into(),
                name: "Disque Externe".into(),
                enabled: true,
                settings: serde_json::json!({
                    "default_format": "E01",
                    "compression_level": 6,
                    "split_size_gb": 2000,
                    "verify_after_create": true,
                }),
            },
            whatsapp: ModuleSettings {
                id: "whatsapp".into(),
                name: "WhatsApp".into(),
                enabled: true,
                settings: serde_json::json!({
                    "parse_deleted": true,
                    "parse_wal": true,
                    "export_media": true,
                    "max_messages": 10000,
                }),
            },
            money_flow: ModuleSettings {
                id: "money_flow".into(),
                name: "Money Flow".into(),
                enabled: true,
                settings: serde_json::json!({
                    "providers": ["ORANGE","MOOV","WAVE","MTN"],
                    "auto_detect_currency": true,
                    "threshold_suspect": 500000,
                }),
            },
            timeline: ModuleSettings {
                id: "timeline".into(),
                name: "Timeline".into(),
                enabled: true,
                settings: serde_json::json!({
                    "max_events": 50000,
                    "group_by_source": true,
                    "show_deleted": true,
                }),
            },
            csm: ModuleSettings {
                id: "csm".into(),
                name: "Chain of Custody".into(),
                enabled: true,
                settings: serde_json::json!({
                    "auto_sign_steps": true,
                    "require_signature": true,
                    "export_pdf_layout": "detailed",
                }),
            },
            cartography: ModuleSettings {
                id: "cartography".into(),
                name: "Cartographie".into(),
                enabled: true,
                settings: serde_json::json!({
                    "default_provider": "google",
                    "heatmap_enabled": true,
                    "connections_visible": true,
                }),
            },
            filters: ModuleSettings {
                id: "filters".into(),
                name: "Filtres Avancés".into(),
                enabled: true,
                settings: serde_json::json!({
                    "max_results": 10000,
                    "cache_filters": true,
                }),
            },
            reports: ModuleSettings {
                id: "reports".into(),
                name: "Rapports".into(),
                enabled: true,
                settings: serde_json::json!({
                    "default_format": "PDF",
                    "include_toc": true,
                    "page_size": "A4",
                    "font": "Inter",
                }),
            },
        }
    }
}

// ─── SESSIONS / SAUVEGARDE ─────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkSession {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
    pub case_ids: Vec<String>,
    pub active_case_id: String,
    pub settings: AppSettings,
    pub module_settings: AllModulesSettings,
    pub window_state: WindowState,
    pub tags: Vec<Tag>,
    pub notes: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: String,
}

// ─── EXPORT FORMATS ─────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub enum ExportFormat {
    PDF, DOCX, HTML, TXT, CSV, JSON, XLSX, XML, ZIP,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_attachments: bool,
    pub include_media: bool,
    pub compress: bool,
    pub encryption: Option<String>,  // "aes256" ou None
    pub password: Option<String>,
    pub template_id: Option<String>,
    pub watermark: Option<String>,
    pub page_size: String,  // A4, A3, Letter
    pub orientation: String, // portrait, landscape
    pub font: String,
    pub include_toc: bool,
    pub include_index: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            format: ExportFormat::PDF,
            include_attachments: true,
            include_media: true,
            compress: false,
            encryption: None,
            password: None,
            template_id: None,
            watermark: None,
            page_size: "A4".into(),
            orientation: "portrait".into(),
            font: "Inter".into(),
            include_toc: true,
            include_index: false,
        }
    }
}

// ─── TAURI COMMANDS ───────────────────────────────────────────

fn get_settings_path(app_dir: &str) -> String {
    format!("{}/settings.json", app_dir)
}

fn get_modules_settings_path(app_dir: &str) -> String {
    format!("{}/module_settings.json", app_dir)
}

#[tauri::command]
pub fn load_app_settings(app_dir: String) -> Result<AppSettings, String> {
    let path = get_settings_path(&app_dir);
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppSettings::default())
    }
}

#[tauri::command]
pub fn save_app_settings(app_dir: String, settings: AppSettings) -> Result<(), String> {
    let path = get_settings_path(&app_dir);
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_modules_settings(app_dir: String) -> Result<AllModulesSettings, String> {
    let path = get_modules_settings_path(&app_dir);
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AllModulesSettings::default())
    }
}

#[tauri::command]
pub fn save_modules_settings(app_dir: String, settings: AllModulesSettings) -> Result<(), String> {
    let path = get_modules_settings_path(&app_dir);
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_to_defaults(app_dir: String) -> Result<(), String> {
    let _ = save_app_settings(app_dir.clone(), AppSettings::default())?;
    let _ = save_modules_settings(app_dir, AllModulesSettings::default())?;
    Ok(())
}

// ─── SESSIONS ─────────────────────────────────────────────────

fn get_sessions_dir(app_dir: &str) -> String {
    format!("{}/sessions", app_dir)
}

#[tauri::command]
pub fn save_session(app_dir: String, session: WorkSession) -> Result<String, String> {
    let dir = get_sessions_dir(&app_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut session = session;
    session.modified_at = chrono::Utc::now().to_rfc3339();
    if session.id.is_empty() {
        session.id = Uuid::new_v4().to_string();
    }
    if session.created_at.is_empty() {
        session.created_at = chrono::Utc::now().to_rfc3339();
    }

    let path = format!("{}/{}.json", dir, session.id);
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(session.id)
}

#[tauri::command]
pub fn load_session(app_dir: String, session_id: String) -> Result<WorkSession, String> {
    let path = format!("{}/{}.json", get_sessions_dir(&app_dir), session_id);
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_sessions(app_dir: String) -> Result<Vec<WorkSession>, String> {
    let dir = get_sessions_dir(&app_dir);
    if !PathBuf::from(&dir).exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(session) = serde_json::from_str::<WorkSession>(&content) {
                    sessions.push(session);
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(sessions)
}

#[tauri::command]
pub fn delete_session(app_dir: String, session_id: String) -> Result<(), String> {
    let path = format!("{}/{}.json", get_sessions_dir(&app_dir), session_id);
    fs::remove_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn duplicate_session(app_dir: String, session_id: String, new_name: String) -> Result<String, String> {
    let mut session = load_session(app_dir.clone(), session_id)?;
    session.id = "".into();
    session.name = new_name;
    session.created_at = "".into();
    session.modified_at = "".into();
    save_session(app_dir, session)
}

// ─── TAGS ────────────────────────────────────────────────────

#[tauri::command]
pub fn create_tag(app_dir: String, name: String, color: String, description: String) -> Result<Tag, String> {
    let tag = Tag {
        id: Uuid::new_v4().to_string(),
        name,
        color,
        description,
    };

    let path = format!("{}/tags.json", app_dir);
    let mut tags: Vec<Tag> = if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    tags.push(tag.clone());
    let json = serde_json::to_string_pretty(&tags).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(tag)
}

#[tauri::command]
pub fn list_tags(app_dir: String) -> Result<Vec<Tag>, String> {
    let path = format!("{}/tags.json", app_dir);
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn delete_tag(app_dir: String, tag_id: String) -> Result<(), String> {
    let path = format!("{}/tags.json", app_dir);
    let mut tags: Vec<Tag> = if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    tags.retain(|t| t.id != tag_id);
    let json = serde_json::to_string_pretty(&tags).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

// ─── EXPORT ──────────────────────────────────────────────────

#[tauri::command]
pub fn export_to_format(
    data: serde_json::Value,
    format: String,
    options: ExportOptions,
    output_path: String,
) -> Result<String, String> {
    match format.as_str() {
        "PDF" => export_pdf(&data, &options, &output_path),
        "DOCX" => export_docx(&data, &options, &output_path),
        "CSV" => export_csv(&data, &output_path),
        "JSON" => export_json(&data, &output_path),
        "HTML" => export_html(&data, &output_path),
        "TXT" => export_txt(&data, &output_path),
        "XLSX" => export_xlsx(&data, &output_path),
        "XML" => export_xml(&data, &output_path),
        "ZIP" => export_zip(&data, &options, &output_path),
        _ => Err(format!("Format non supporté: {}", format)),
    }
}

fn export_pdf(data: &serde_json::Value, opts: &ExportOptions, path: &str) -> Result<String, String> {
    // Simulation - en prod, utiliser pdf-lib ou weasyprint
    let content = format!("PDF Export\nFormat: {:?}\nPages: {:?}\nOrientation: {}\nWatermark: {:?}\n---\n{}",
        opts.format, opts.page_size, opts.orientation, opts.watermark, data);
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_docx(data: &serde_json::Value, opts: &ExportOptions, path: &str) -> Result<String, String> {
    let content = format!("DOCX Export\nFormat: {:?}\nInclude TOC: {}\n---\n{}",
        opts.format, opts.include_toc, data);
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_csv(data: &serde_json::Value, path: &str) -> Result<String, String> {
    // Simple CSV - en prod, utiliser csv crate
    let content = "id,name,value\n1,Item 1,100\n2,Item 2,200\n";
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_json(data: &serde_json::Value, path: &str) -> Result<String, String> {
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_html(data: &serde_json::Value, path: &str) -> Result<String, String> {
    let html = format!("<!DOCTYPE html><html><head><meta charset='UTF-8'><title>FIDO-LABS Export</title></head><body><h1>Export FIDO-LABS</h1><pre>{}</pre></body></html>", data);
    fs::write(path, html).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_txt(data: &serde_json::Value, path: &str) -> Result<String, String> {
    let txt = format!("FIDO-LABS Export\n==================\n\n{}", data);
    fs::write(path, txt).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_xlsx(data: &serde_json::Value, path: &str) -> Result<String, String> {
    // Simulation - en prod, utiliser calamine ou xlsx writer
    fs::write(path, "XLSX format - placeholder").map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_xml(data: &serde_json::Value, path: &str) -> Result<String, String> {
    let xml = format!("<?xml version='1.0'?><root>{}</root>", data);
    fs::write(path, xml).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

fn export_zip(data: &serde_json::Value, opts: &ExportOptions, path: &str) -> Result<String, String> {
    // Simulation - en prod, utiliser zip crate
    let content = format!("ZIP Archive\nCompressed: {}\nPassword: {:?}\n---\n{}",
        opts.compress, opts.password, data);
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(format!("Exporté: {}", path))
}

#[tauri::command]
pub fn get_export_templates() -> Vec<ExportTemplate> {
    vec![
        ExportTemplate { id: "rpt_gen".into(), name: "Rapport Général".into(), format: "PDF".into(), description: "Rapport complet d'enquête".into() },
        ExportTemplate { id: "rpt_suspect".into(), name: "Rapport Suspect".into(), format: "PDF".into(), description: "Focus sur un individu".into() },
        ExportTemplate { id: "rpt_custody".into(), name: "Chaîne de Garde".into(), format: "PDF".into(), description: "Rapport CSM judiciaire".into() },
        ExportTemplate { id: "rpt_timeline".into(), name: "Timeline".into(), format: "HTML".into(), description: "Chronologie interactive".into() },
        ExportTemplate { id: "rpt_money".into(), name: "Flux Financiers".into(), format: "XLSX".into(), description: "Analyse financière détaillée".into() },
        ExportTemplate { id: "rpt_evidence".into(), name: "Catalogue Preuves".into(), format: "CSV".into(), description: "Inventaire des preuves".into() },
        ExportTemplate { id: "backup_full".into(), name: "Sauvegarde Complète".into(), format: "ZIP".into(), description: "Backup session + données".into() },
    ]
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExportTemplate {
    pub id: String,
    pub name: String,
    pub format: String,
    pub description: String,
}

// ─── RACCOURCIS CLAVIER ───────────────────────────────────────

#[tauri::command]
pub fn get_keyboard_shortcuts() -> Vec<KeyboardShortcut> {
    vec![
        KeyboardShortcut { action: "new_case".into(), keys: "Ctrl+N".into(), description: "Nouveau dossier".into() },
        KeyboardShortcut { action: "save_session".into(), keys: "Ctrl+S".into(), description: "Sauvegarder session".into() },
        KeyboardShortcut { action: "open_session".into(), keys: "Ctrl+O".into(), description: "Ouvrir session".into() },
        KeyboardShortcut { action: "export".into(), keys: "Ctrl+E".into(), description: "Exporter".into() },
        KeyboardShortcut { action: "settings".into(), keys: "Ctrl+,".into(), description: "Paramètres".into() },
        KeyboardShortcut { action: "search".into(), keys: "Ctrl+F".into(), description: "Rechercher".into() },
        KeyboardShortcut { action: "toggle_sidebar".into(), keys: "Ctrl+B".into(), description: "Afficher/masquer barre latérale".into() },
        KeyboardShortcut { action: "fullscreen".into(), keys: "F11".into(), description: "Plein écran".into() },
        KeyboardShortcut { action: "refresh".into(), keys: "F5".into(), description: "Actualiser".into() },
        KeyboardShortcut { action: "help".into(), keys: "F1".into(), description: "Aide".into() },
        KeyboardShortcut { action: "quit".into(), keys: "Alt+F4".into(), description: "Quitter".into() },
    ]
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KeyboardShortcut {
    pub action: String,
    pub keys: String,
    pub description: String,
}

// ─── INFO APPLICATION ─────────────────────────────────────────

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "FIDO-LABS".into(),
        version: "3.0.0".into(),
        build: "2024.09.19".into(),
        codename: "Forensic Phoenix".into(),
        author: "FIDO-LABS Team".into(),
        website: "https://fido-labs.com".into(),
        support_email: "support@fido-labs.com".into(),
        license: "Proprietary - Usage réglementé".into(),
        copyright: "© 2024 FIDO-LABS - Tous droits réservés".into(),
        features: vec![
            "Extraction Mobile (ADB/BROM/EDL)".into(),
            "Extraction SD Card & Disque".into(),
            "Décodeur WhatsApp & Mobile Money".into(),
            "Timeline & Graphes".into(),
            "Cartographie GPS".into(),
            "Chain of Custody Management".into(),
            "Filtres Avancés".into(),
            "Générateur Rapports".into(),
            "Gestion Licences".into(),
            "Sessions & Sauvegardes".into(),
        ],
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub build: String,
    pub codename: String,
    pub author: String,
    pub website: String,
    pub support_email: String,
    pub license: String,
    pub copyright: String,
    pub features: Vec<String>,
}