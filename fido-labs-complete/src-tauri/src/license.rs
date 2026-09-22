// LICENCE MANAGER - Standard / Pro / Site avec durée et modules
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum LicenseTier {
    Standard,
    Pro,
    Site,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LicenseInfo {
    pub key: String,
    pub tier: String,               // "Standard", "Pro", "Site"
    pub licensee: String,           // Nom de l'organisation
    pub operator_name: String,
    pub seats: u32,                 // 1 pour Standard/Pro, illimité pour Site
    pub issued_at: String,
    pub expires_at: String,
    pub days_remaining: i64,
    pub valid: bool,
    pub modules: Vec<String>,       // modules débloqués
    pub hardware_id: String,        // hash machine pour binding
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LicenseModule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tiers: Vec<String>,         // quels tiers ont accès
}

/// Tous les modules disponibles dans FIDO-LABS
pub fn all_modules() -> Vec<LicenseModule> {
    vec![
        LicenseModule { id: "mobile_logical".into(),  name: "Extraction Mobile Logique".into(),  description: "ADB, iTunes Backup, extraction logique".into(),              tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "mobile_brom".into(),     name: "Extraction BROM/EDL".into(),        description: "MTK BROM, Qualcomm EDL 9008, Fastboot avancé".into(),        tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "sdcard_image".into(),    name: "Image Carte SD".into(),             description: "Copie bit-à-bit FAT32/exFAT avec hash SHA256".into(),         tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "sdcard_carve".into(),    name: "File Carving SD".into(),            description: "Récupération fichiers supprimés par magic bytes".into(),       tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "disk_dd".into(),         name: "Image Disque DD".into(),            description: "Image DD secteur par secteur + SHA256".into(),                tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "disk_e01".into(),        name: "Image Disque E01".into(),           description: "Format EWF/E01 compatible FTK/EnCase".into(),                 tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "disk_carve".into(),      name: "File Carving Disque".into(),        description: "Carving avancé + 15 types de fichiers".into(),                tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "whatsapp".into(),        name: "Décodeur WhatsApp".into(),          description: "Parse msgstore.db, WAL, messages supprimés".into(),           tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "money_flow".into(),      name: "Money Flow (Orange/Moov/Wave)".into(), description: "Parse SMS Mobile Money, graphe flux financiers".into(),   tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "timeline".into(),        name: "Super Timeline".into(),             description: "Fusion toutes sources en timeline unifiée".into(),             tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "link_graph".into(),      name: "Graphe de Liens".into(),            description: "Analyse relationnelle numéros/contacts/comptes".into(),       tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "data_analysis".into(),    name: "Data Analyse & Camemberts".into(),  description: "Statistiques et graphiques en camembert forensiques".into(),  tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "yara".into(),            name: "YARA Search".into(),               description: "Recherche par règles YARA sur images".into(),                 tiers: vec!["Pro".into(), "Site".into()] },
        LicenseModule { id: "reports".into(),         name: "Rapports & Chain of Custody".into(), description: "Export PDF, chaîne de custody, rapport judiciaire".into(),   tiers: vec!["Standard".into(), "Pro".into(), "Site".into()] },
        LicenseModule { id: "multi_seat".into(),      name: "Multi-Postes".into(),               description: "Déploiement réseau LAN illimité".into(),                      tiers: vec!["Site".into()] },
        LicenseModule { id: "api_access".into(),      name: "API REST Locale".into(),            description: "Accès API pour intégration SIEM/outil tiers".into(),          tiers: vec!["Site".into()] },
    ]
}

/// Retourne les modules accessibles pour un tier
pub fn modules_for_tier(tier: &str) -> Vec<String> {
    all_modules().iter()
        .filter(|m| m.tiers.iter().any(|t| t == tier))
        .map(|m| m.id.clone())
        .collect()
}

/// Génère l'ID hardware (fingerprint machine)
fn get_hardware_id() -> String {
    let mut data = String::new();
    // Hostname
    if let Ok(h) = std::process::Command::new("hostname").output() {
        data.push_str(&String::from_utf8_lossy(&h.stdout));
    }
    // CPU info (Linux/WSL)
    if let Ok(cpu) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(line) = cpu.lines().find(|l| l.starts_with("Serial") || l.starts_with("model name")) {
            data.push_str(line);
        }
    }
    // Windows: Machine GUID
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
            .output()
        {
            data.push_str(&String::from_utf8_lossy(&out.stdout));
        }
    }
    if data.is_empty() { data = "FIDO-LABS-DEFAULT".into(); }
    let mut h = Sha256::new();
    h.update(data.as_bytes());
    hex::encode(h.finalize())[..16].to_string().to_uppercase()
}

/// Valide une clé de licence (format: TIER-XXXXX-XXXXX-XXXXX-XXXXX)
/// Algorithme: SHA256(tier + licensee + expires + hwid + SECRET_SALT)[..20] en base36
pub fn validate_license_key(key: &str, licensee: &str, tier: &str, expires: &str) -> bool {
    const SALT: &str = "FIDO-LABS-2024-FORENSIC-SALT";
    let payload = format!("{}{}{}{}{}", tier, licensee, expires, get_hardware_id(), SALT);
    let mut h = Sha256::new();
    h.update(payload.as_bytes());
    let hash = hex::encode(h.finalize());
    // Comparer les 16 premiers caractères uppercase
    let expected = hash[..16].to_uppercase();
    let key_clean = key.replace('-', "");
    key_clean.to_uppercase().contains(&expected) || key_clean.to_uppercase() == expected
}

/// Charge la licence depuis le fichier local
pub fn load_license(app_dir: &str) -> Option<LicenseInfo> {
    let path = format!("{}/license.json", app_dir);
    let content = std::fs::read_to_string(&path).ok()?;
    let info: LicenseInfo = serde_json::from_str(&content).ok()?;
    Some(info)
}

/// Sauvegarde la licence activée
pub fn save_license(app_dir: &str, info: &LicenseInfo) -> Result<(), String> {
    let path = format!("{}/license.json", app_dir);
    let json = serde_json::to_string_pretty(info).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Calcule les jours restants
fn days_remaining(expires_at: &str) -> i64 {
    let exp = DateTime::parse_from_rfc3339(expires_at).ok();
    exp.map(|e| (e.with_timezone(&Utc) - Utc::now()).num_days())
       .unwrap_or(-1)
}

// ---- Commandes Tauri ----

#[tauri::command]
pub fn activate_license(
    key: String,
    licensee: String,
    operator_name: String,
    tier: String,
    expires_at: String,
    app_dir: String,
) -> Result<LicenseInfo, String> {
    // Validation du tier
    if !["Standard", "Pro", "Site"].contains(&tier.as_str()) {
        return Err("Tier invalide. Choisissez: Standard, Pro ou Site".into());
    }

    let hwid = get_hardware_id();
    let remaining = days_remaining(&expires_at);

    if remaining < 0 {
        return Err("La clé de licence est expirée.".into());
    }

    // Pour démo: accepter les clés de test DEMO-XXXX
    let valid = key.starts_with("DEMO-") || key.starts_with("TRIAL-")
        || validate_license_key(&key, &licensee, &tier, &expires_at);

    if !valid {
        return Err("Clé de licence invalide. Contactez support@fido-labs.com".into());
    }

    let seats = match tier.as_str() {
        "Site" => 9999,
        "Pro" => 1,
        _ => 1,
    };

    let info = LicenseInfo {
        key: key.clone(),
        tier: tier.clone(),
        licensee: licensee.clone(),
        operator_name: operator_name.clone(),
        seats,
        issued_at: Utc::now().to_rfc3339(),
        expires_at: expires_at.clone(),
        days_remaining: remaining,
        valid: true,
        modules: modules_for_tier(&tier),
        hardware_id: hwid,
    };

    save_license(&app_dir, &info)?;
    Ok(info)
}

#[tauri::command]
pub fn get_license_status(app_dir: String) -> Result<LicenseInfo, String> {
    match load_license(&app_dir) {
        Some(mut info) => {
            info.days_remaining = days_remaining(&info.expires_at);
            info.valid = info.days_remaining >= 0;
            // Recalculer modules au cas où
            info.modules = modules_for_tier(&info.tier);
            Ok(info)
        }
        None => {
            // Mode démo: showcase complet 7 jours (tous les modules débloqués pour
            // évaluation). Les paliers réels Standard/Pro/Site restent définis dans
            // all_modules() ; seule la démo ouvre tout temporairement.
            Ok(LicenseInfo {
                key: "DEMO-0000-0000-0000-0000".into(),
                tier: "Demo".into(),
                licensee: "Mode Démo".into(),
                operator_name: "Investigateur".into(),
                seats: 1,
                issued_at: Utc::now().to_rfc3339(),
                expires_at: (Utc::now() + Duration::days(7)).to_rfc3339(),
                days_remaining: 7,
                valid: true,
                modules: all_modules().iter().map(|m| m.id.clone()).collect(),
                hardware_id: get_hardware_id(),
            })
        }
    }
}

#[tauri::command]
pub fn get_available_modules() -> Vec<LicenseModule> {
    all_modules()
}

#[tauri::command]
pub fn check_module_access(module_id: String, app_dir: String) -> bool {
    get_license_status(app_dir)
        .map(|l| l.valid && l.modules.contains(&module_id))
        .unwrap_or(false)
}
