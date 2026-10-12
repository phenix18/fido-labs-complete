// LOCKED EXTRACTION MODULE v1 - Acquisition d'image pour appareils VERROUILLÉS et/ou
// NON DÉBOGABLES (adb unauthorized / pas de débogage / écran cassé / iOS verrouillé).
//
// Principe forensique strict : on N'ENFREINT PAS le verrou ni le chiffrement.
// On ACQUIERT l'image (image forensique) là où les données sont physiquement accessibles
// SANS déverrouiller, ou on acquiert le CHIFFRÉ tel quel pour analyse légale ultérieure.
//
// Méthodes d'acquisition d'image couvertes (par plateforme) :
//   - SIM : image complète de la carte SIM (contacts, SMS, ICCID/IMSI, derniers numéros)
//   - SD amovible : image forensique (raw .dd / .e01) + carving
//   - BROM/EDL : dump de la partition brute (chiffrée si FBE/BFU) via mtkclient / edl
//   - ISP / Chip-off : lecture directe de la puce eMMC/UFS (USB mort, dernier recours)
//   - iOS checkm8 : acquisition DFU de la mémoire sur A5–A11 (Secure Enclave intacte)
//   - iTunes/iCloud : sauvegarde autorisée ou réquisition fournisseur

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageAcquisitionStep {
    pub order: u32,
    pub method: String,
    pub target: String,             // SIM / SD / eMMC / UFS / Mémoire iOS / Cloud
    pub description: String,
    pub image_output: String,       // ce que produit la méthode (format d'image)
    pub command_hint: String,
    pub requires_hardware: bool,
    pub hardware_needed: Vec<String>,
    pub legal_note: String,
    pub is_destructive: bool,
    pub success_probability: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LockedAcquisitionPlan {
    pub device_state: String,       // LOCKED / NO_DEBUG / BROKEN_SCREEN / BFU / etc.
    pub platform: String,           // Android-MTK / Android-Qualcomm / Android-Other / iOS / Generic
    pub device_label: String,       // libellé lisible
    pub image_acquisition: Vec<ImageAcquisitionStep>,
    pub best_effort: String,        // recommandation principale
    pub legal_requirements: Vec<String>,
}

/// Catégorise la plateforme selon l'état et les indices fournis.
fn classify_platform(vid: &str, pid: &str, adb_state: &str, is_ios: bool, checkm8: bool) -> String {
    if is_ios {
        if checkm8 { "iOS-checkm8".into() } else { "iOS-modern".into() }
    } else {
        match (vid.to_lowercase().as_str(), pid.to_lowercase().as_str()) {
            ("0e8d", _) => "Android-MTK".into(),
            ("05c6", _) => "Android-Qualcomm".into(),
            _ if adb_state.contains("unauthorized") || adb_state.contains("device") => "Android-Other".into(),
            _ => "Android-Other".into(),
        }
    }
}

/// Construit le plan d'acquisition d'image pour un appareil verrouillé / non débogable.
/// C'est la recommandation, PAS une exécution de contournement : chaque méthode
/// documente son format d'image et sa note légale.
pub fn build_locked_plan(
    state: &str, platform: &str, vid: &str, pid: &str, serial: &str,
) -> LockedAcquisitionPlan {
    let mut steps: Vec<ImageAcquisitionStep> = Vec::new();
    let legal: Vec<String> = vec![
        "Autorisation du propriétaire ou mandat judiciaire OBLIGATOIRE.".into(),
        "Toute acquisition est scellée (SHA256 + horodatage + opérateur) dans le rapport chaîne de garde.".into(),
    ];
    let mut order = 1u32;

    // Source universelle n°1 : la carte SIM (image complète).
    steps.push(ImageAcquisitionStep {
        order: order, target: "SIM".into(),
        method: "Image complète de la carte SIM".into(),
        description: "La carte SIM est amovible et NON protégée par le verrou de l'appareil. Lecture directe au lecteur SIM : contacts, SMS stockés, ICCID/IMSI, derniers numéros composés.".into(),
        image_output: "Image SIM (raw) + export contacts/SMS (ADN/EF)".into(),
        command_hint: "SIM reader + pysim / SIMtrace".into(),
        requires_hardware: true,
        hardware_needed: vec!["Lecteur de carte SIM (OMNIKEY / Gemalto)".into()],
        legal_note: "Aucun verrou appareil impliqué — source prioritaire.".into(),
        is_destructive: false, success_probability: 0.90,
    });
    order += 1;

    // Source universelle n°2 : carte SD amovible (image forensique).
    steps.push(ImageAcquisitionStep {
        order: order, target: "SD".into(),
        method: "Image forensique de la carte SD amovible".into(),
        description: "Souvent non chiffrée. Image raw (.dd/.e01) de la carte + carving des fichiers supprimés (photos, médias WhatsApp, documents).".into(),
        image_output: "Image .dd / .e01 + carve (.carve)".into(),
        command_hint: "Bouton « Imager carte SD » + « Carver » (sans root)".into(),
        requires_hardware: true,
        hardware_needed: vec!["Câble USB / adaptateur SD".into()],
        legal_note: "La SD n'est pas protégée par le verrou — source prioritaire.".into(),
        is_destructive: false, success_probability: 0.95,
    });
    order += 1;

    match platform {
        "Android-MTK" => {
            steps.push(ImageAcquisitionStep {
                order: order, target: "eMMC (BROM)".into(),
                method: "Dump physique MediaTek (BROM) — partition brute".into(),
                description: "Lecture directe de la flash eMMC en mode BootROM (test point), SANS déverrouiller. On acquiert la partition telle quelle (chiffrée si FBE/BFU) pour analyse légale.".into(),
                image_output: "Dump .bin / .img des partitions (GPT, userdata…)".into(),
                command_hint: "mtkclient mtk r userdata userdata.bin".into(),
                requires_hardware: true,
                hardware_needed: vec!["Test point / câble EDL".into(), "mtkclient".into()],
                legal_note: "Dump du CHIFFRÉ — lisible seulement si l'appareil était en AFU ; sinon conservé pour analyse ultérieure. Mandat requis.".into(),
                is_destructive: false, success_probability: 0.75,
            });
            order += 1;
        }
        "Android-Qualcomm" => {
            steps.push(ImageAcquisitionStep {
                order: order, target: "UFS/eMMC (EDL)".into(),
                method: "Dump physique Qualcomm (EDL 9008) — partition brute".into(),
                description: "Mode Emergency Download via câble EDL/test point. Lecture UFS/eMMC via Sahara/Firehose, acquisition du chiffré tel quel.".into(),
                image_output: "Dump .bin / .img des partitions (userdata…)".into(),
                command_hint: "edl r userdata userdata.bin --loader prog_firehose_ddr.elf".into(),
                requires_hardware: true,
                hardware_needed: vec!["Câble EDL".into(), "Loader firehose .elf du SoC".into()],
                legal_note: "Dump du CHIFFRÉ (FBE) — lisible si AFU, sinon conservé. Mandat requis.".into(),
                is_destructive: false, success_probability: 0.70,
            });
            order += 1;
        }
        "iOS-checkm8" => {
            steps.push(ImageAcquisitionStep {
                order: order, target: "Mémoire iOS (DFU checkm8)".into(),
                method: "Acquisition DFU checkm8 (A5–A11)".into(),
                description: "Exploit DFU matériel (checkm8) : acquisition du système de fichiers iOS sur appareil VERROUILLÉ, sans toucher au Secure Enclave ni au code. Utilisé par checkra1n (outil certifié).".into(),
                image_output: "Image du système de fichiers iOS (montage + extraction)".into(),
                command_hint: "checkra1n / ipwndfu (DFU)".into(),
                requires_hardware: false,
                hardware_needed: vec!["Câble USB (données)".into()],
                legal_note: "Acquisition légale sur mandat ; le Secure Enclave et le verrou restent intacts — on ne déchiffre PAS les données protégées par l'enclave sans code.".into(),
                is_destructive: false, success_probability: 0.80,
            });
            order += 1;
        }
        "iOS-modern" => {
            steps.push(ImageAcquisitionStep {
                order: order, target: "Sauvegarde / Cloud".into(),
                method: "Sauvegarde iTunes/Finder ou iCloud (réquisition)".into(),
                description: "A12+ : pas de checkm8 fiable. Extraction via sauvegarde autorisée (mot de passe légal si chiffrée) ou réquisition iCloud auprès d'Apple.".into(),
                image_output: "Sauvegarde iOS (.zip) / données iCloud".into(),
                command_hint: "idevicebackup2 / réquisition".into(),
                requires_hardware: false,
                hardware_needed: vec![],
                legal_note: "Sauvegarde chiffrée = mot de passe fourni légalement ; iCloud = réquisition judiciaire au fournisseur.".into(),
                is_destructive: false, success_probability: 0.60,
            });
            order += 1;
        }
        _ => {
            // Android générique / autre : dépend du SoC.
            steps.push(ImageAcquisitionStep {
                order: order, target: "Dépend du SoC".into(),
                method: "Analyse du chipset pour BROM/EDL".into(),
                description: "Plateforme non identifiée par VID/PID. Détecter le SoC (ro.board.platform / VID) pour choisir BROM (MTK) ou EDL (Qualcomm), ou basculer sur SD/SIM/chip-off.".into(),
                image_output: "—".into(),
                command_hint: "FIDO-LABS → « Détecter le chipset »".into(),
                requires_hardware: false,
                hardware_needed: vec![],
                legal_note: "Identifier d'abord la plateforme pour une acquisition ciblée.".into(),
                is_destructive: false, success_probability: 0.50,
            });
            order += 1;
        }
    }

    // Méthodes physiques avancées (dernier recours) communes à toutes les plateformes.
    steps.push(ImageAcquisitionStep {
        order: order, target: "Puce eMMC/UFS (ISP)".into(),
        method: "ISP (In-System Programming) — lecture directe de la puce".into(),
        description: "Connexion sur les pads ISP de la puce de stockage sans la démonter. Dump complet eMMC/UFS (chiffré si FBE).".into(),
        image_output: "Dump eMMC/UFS brut".into(),
        command_hint: "Station ISP + pinouts du modèle".into(),
        requires_hardware: true,
        hardware_needed: vec!["Station ISP".into(), "Pinouts du modèle".into(), "Expertise soudure".into()],
        legal_note: "Technique avancée, risque d'endommagement. Mandat obligatoire.".into(),
        is_destructive: false, success_probability: 0.55,
    });
    order += 1;
    steps.push(ImageAcquisitionStep {
        order: order, target: "Puce eMMC/UFS (chip-off)".into(),
        method: "Chip-off — déchippage de la puce".into(),
        description: "Démontage de la puce de stockage et lecture sur lecteur externe. Dernier recours (irréversible).".into(),
        image_output: "Dump eMMC/UFS brut".into(),
        command_hint: "Station de rebillage + lecteur eMMC/UFS".into(),
        requires_hardware: true,
        hardware_needed: vec!["Station de rebillage".into(), "Lecteur eMMC/UFS".into(), "Expertise hardware".into()],
        legal_note: "DESTRUCTIF — dernier recours. Mandat obligatoire.".into(),
        is_destructive: true, success_probability: 0.50,
    });

    // Recommandation principale.
    let best = if platform == "iOS-checkm8" {
        "Acquisition DFU checkm8 de la mémoire iOS (A5–A11) — SANS déverrouiller.".to_string()
    } else {
        "Imager la carte SIM + la carte SD (sources non protégées par le verrou), puis dump physique BROM/EDL du chiffré si mandat.".to_string()
    };

    LockedAcquisitionPlan {
        device_state: state.to_string(),
        platform: platform.to_string(),
        device_label: format!("{} (VID:{} PID:{}, serial: {})", platform, vid, pid, serial),
        image_acquisition: steps,
        best_effort: best,
        legal_requirements: legal,
    }
}

/// Construit le plan d'acquisition d'image pour un appareil verrouillé/non débogable,
/// selon la plateforme et l'état détectés (vid/pid/adb_state fournis par l'UI).
#[tauri::command]
pub fn locked_acquisition_plan(
    state: String, vid: String, pid: String, serial: String,
    is_ios: bool, checkm8: bool, adb_state: String,
) -> LockedAcquisitionPlan {
    let platform = classify_platform(&vid, &pid, &adb_state, is_ios, checkm8);
    build_locked_plan(&state, &platform, &vid, &pid, &serial)
}

/// Déclenche l'acquisition d'une image de carte SIM via le module SIM.
/// Documente la commande sous-jacente ; le lecteur physique reste requis.
#[tauri::command]
pub fn acquire_sim_image() -> Result<serde_json::Value, String> {
    super::sim::detect_sim_readers()
}

/// Acquisition RÉELLE d'une image d'un support amovible (carte SD / stockage USB /
/// lecteur SIM exposé comme périphérique bloc). Réutilise l'imagerie SD existante :
/// copie forensique + SHA256 + manifeste de chaîne de garde. Ne dépend PAS du verrou
/// de l'appareil — la source est physique et retirée du téléphone verrouillé.
#[tauri::command]
pub fn acquire_removable_image(source: String, dest: String, case_id: String) -> Result<serde_json::Value, String> {
    // Réutilise l'imagerie SD (mêmes conventions : copie brute + SHA256 + manifeste).
    let msg = crate::extractors::sdcard::sd_extractor::image_sd_card(&source, &dest, &case_id)?;
    let sha = if let Some(m) = std::fs::read_to_string(format!("{}.manifest.txt", dest)).ok() {
        m.lines().find(|l| l.starts_with("SHA256:"))
            .map(|l| l.trim_start_matches("SHA256:").trim().to_string())
            .unwrap_or_default()
    } else { String::new() };
    Ok(serde_json::json!({
        "success": true,
        "source": source,
        "dest": dest,
        "sha256": sha,
        "summary": msg,
        "note": "Image acquise depuis un support amovible — indépendante du verrou de l'appareil. Chaîne de garde : SHA256 + manifeste.",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_mtk_inclut_brom_et_sim_sd() {
        let plan = build_locked_plan("LOCKED", "Android-MTK", "0e8d", "0003", "S1");
        let methods: Vec<String> = plan.image_acquisition.iter().map(|s| s.method.clone()).collect();
        assert!(methods.iter().any(|m| m.contains("SIM")), "SIM inclus");
        assert!(methods.iter().any(|m| m.contains("SD")), "SD inclus");
        assert!(methods.iter().any(|m| m.contains("BROM")), "BROM inclus");
        assert!(plan.image_acquisition.last().unwrap().method.to_lowercase().contains("chip-off"));
    }

    #[test]
    fn plan_qualcomm_inclut_edl() {
        let plan = build_locked_plan("LOCKED", "Android-Qualcomm", "05c6", "9008", "S2");
        assert!(plan.image_acquisition.iter().any(|s| s.method.contains("EDL")));
    }

    #[test]
    fn plan_ios_checkm8_prioritaire() {
        let plan = build_locked_plan("LOCKED", "iOS-checkm8", "05ac", "12a8", "UDID1");
        assert!(plan.best_effort.contains("checkm8"));
        assert!(plan.image_acquisition.iter().any(|s| s.method.contains("checkm8")));
    }

    #[test]
    fn platform_classification() {
        assert_eq!(classify_platform("0e8d", "0003", "", false, false), "Android-MTK");
        assert_eq!(classify_platform("05c6", "9008", "", false, false), "Android-Qualcomm");
        assert_eq!(classify_platform("05ac", "12a8", "", true, true), "iOS-checkm8");
        assert_eq!(classify_platform("05ac", "12a8", "", true, false), "iOS-modern");
    }

    #[test]
    fn image_un_support_amovible() {
        // Fichier source factice (représente une carte SD/support amovible).
        let root = std::env::temp_dir().join(format!("fido_img_{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("sd.img");
        std::fs::write(&src, vec![0x55u8; 4096]).unwrap();
        let dst = root.join("sd_copy.img");
        let r = crate::extractors::sdcard::sd_extractor::image_sd_card(
            src.to_string_lossy().as_ref(), dst.to_string_lossy().as_ref(), "CASE-TEST",
        ).expect("image SD");
        assert!(r.contains("SHA256"));
        // Le manifeste de chaîne de garde est écrit.
        assert!(std::fs::read_to_string(format!("{}.manifest.txt", dst.to_string_lossy())).is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }
}
