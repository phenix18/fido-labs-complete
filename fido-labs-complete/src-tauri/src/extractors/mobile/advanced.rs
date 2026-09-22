// ADVANCED MOBILE MODULE v3 - Écran cassé, MDP oublié, Extraction physique + logique
// Usage légal uniquement avec autorisation judiciaire

use serde::{Deserialize, Serialize};
use std::process::Command;
use uuid::Uuid;

// ─── ÉTATS AVANCÉS ───────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PhoneState {
    On,                 // Allumé + déverrouillé
    OnLocked,           // Allumé + verrouillé (PIN/pattern/biom)
    Off,                // Éteint → BROM (MTK) ou EDL (Qualcomm)
    BrokenScreen,       // Écran cassé, USB fonctionnel
    BrokenScreenLocked, // Écran cassé ET verrouillé
    BootloaderFastboot, // Mode fastboot accessible
    RecoveryMode,       // Mode recovery TWRP/stock
    DeepSleep,          // BFU (Before First Unlock) - extraction limitée
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ExtractionType {
    // ── LOGIQUE ──
    AdbLogical,           // adb backup / adb pull (app data si root)
    AdbFull,              // adb backup full avec agent APK
    iTunesBackup,         // iOS - iTunes/Finder backup chiffré ou non
    iCloudBackup,         // iOS - iCloud (nécessite identifiants légaux)
    // ── PHYSIQUE ──
    BromDump,             // MTK BROM - lecture raw flash via test point
    EdlDump,              // Qualcomm EDL 9008 - lecture UFS/eMMC via QFIL
    FastbootImage,        // fastboot dump (si bootloader déverrouillé)
    JtagDump,             // JTAG hardware probe - très avancé
    IspDump,              // ISP (In-System Programming) - soudure puce
    ChipOff,              // Déchippage - puce eMMC/UFS lue séparément
    // ── CLOUD ──
    GoogleAccount,        // Sauvegarde Google (ordonnance nécessaire)
    SamsungCloud,         // Samsung Cloud
    HuaweiCloud,          // Huawei Cloud
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExtractionPlan {
    pub id: String,
    pub device_state: String,
    pub recommended_methods: Vec<ExtractionStep>,
    pub physical_equipment_needed: Vec<String>,
    pub legal_requirements: Vec<String>,
    pub estimated_success_rate: f32,
    pub risk_level: String, // LOW, MEDIUM, HIGH, CRITICAL
    pub alternative_sources: Vec<String>, // extraction SANS contournement du verrou
}

/// Sources de données exploitables SANS déverrouiller, décoder ni contourner le verrou.
/// Principe forensique : on ne casse pas le verrou — on récupère les données là où
/// elles sont accessibles autrement, ou on acquiert le chiffré tel quel.
fn legitimate_sources(state: &str) -> Vec<String> {
    let mut v = vec![
        "Carte SIM — contacts, SMS, ICCID/IMSI, derniers numéros : lecture directe au lecteur SIM (aucun verrou appareil impliqué).".into(),
        "Carte SD amovible — photos, documents, médias WhatsApp : retirer et imager/carver directement (souvent non chiffrée).".into(),
        "Sauvegarde cloud (Google / Apple iCloud / Samsung) — sur réquisition judiciaire au fournisseur : les données sont hors de l'appareil, le verrou ne s'applique pas.".into(),
        "Ordinateurs & supports associés — sauvegardes iTunes/adb, tokens de session, fichiers synchronisés sur un PC saisi.".into(),
        "Objets liés au compte — montre connectée, autre appareil de la personne synchronisé sur le même compte.".into(),
    ];
    if state.contains("LOCKED") || state.contains("OFF") || state == "ADB_UNAUTHORIZED" || state == "UNKNOWN" {
        v.push("Dump physique CHIFFRÉ (BROM/EDL/ISP/chip-off) — on acquiert la partition telle quelle SANS la déchiffrer. Contenu lisible seulement si l'appareil était déjà déverrouillé depuis l'allumage (état AFU) ; sinon on conserve le chiffré pour analyse ultérieure légale.".into());
        v.push("Code d'accès obtenu LÉGALEMENT — fourni par le propriétaire, ou par réquisition/mandat. Ce n'est PAS un contournement : c'est la voie légale prioritaire.".into());
    }
    v
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExtractionStep {
    pub order: u32,
    pub method: String,
    pub description: String,
    pub command_hint: String,
    pub success_probability: f32,
    pub data_expected: Vec<String>,
    pub requires_hardware: bool,
    pub hardware_needed: Vec<String>,
    pub legal_note: String,
    pub is_destructive: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BrokenScreenOptions {
    pub otg_mouse_possible: bool,  // souris OTG pour naviguer
    pub adb_already_enabled: bool, // ADB autorisé avant cassure
    pub scrcpy_available: bool,    // mirroring via scrcpy
    pub usb_data_works: bool,      // le port USB fonctionne encore
    pub backup_display_possible: bool, // branchement moniteur externe possible
    pub isp_required: bool,        // ISP/JTAG si USB mort
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PasswordRecoveryOptions {
    pub owner_present: bool,       // propriétaire peut donner le code
    pub judicial_order: bool,      // mandat judiciaire en main
    pub cloud_backup_available: bool, // sauvegarde cloud accessible
    pub edl_method_possible: bool, // EDL/BROM bypass possible (Qualcomm/MTK)
    pub device_age_days: u32,      // délai depuis dernier usage
    pub mdm_enrolled: bool,        // inscrit dans un MDM entreprise
}

// ─── DIAGNOSTIC ──────────────────────────────────────────────────

#[tauri::command]
pub fn diagnose_device(
    vid: String, pid: String, adb_output: String, screen_broken: bool, password_locked: bool,
) -> Result<ExtractionPlan, String> {
    let state = determine_state(&vid, &pid, &adb_output, screen_broken, password_locked);
    let plan = build_extraction_plan(&state, screen_broken, password_locked);
    Ok(plan)
}

fn determine_state(vid: &str, pid: &str, adb_out: &str, screen_broken: bool, locked: bool) -> String {
    match (vid, pid) {
        ("0e8d", "0003") | ("0e8d", "2000") => "OFF_BROM_MTK".into(),
        ("05c6", "9008") => "OFF_EDL_QUALCOMM".into(),
        ("18d1", "d001") | ("05c6", "9025") => "BOOTLOADER".into(),
        _ if adb_out.contains("device") && !locked => "ON".into(),
        _ if adb_out.contains("device") && locked => "ON_LOCKED".into(),
        _ if adb_out.contains("unauthorized") => "ADB_UNAUTHORIZED".into(),
        _ if adb_out.contains("offline") => "OFF".into(),
        _ if screen_broken && locked => "BROKEN_SCREEN_LOCKED".into(),
        _ if screen_broken => "BROKEN_SCREEN".into(),
        _ => "UNKNOWN".into(),
    }
}

fn build_extraction_plan(state: &str, screen_broken: bool, locked: bool) -> ExtractionPlan {
    let mut steps = Vec::new();
    let mut equipment = Vec::new();
    let mut legal = Vec::new();
    let mut success_rate = 0.0f32;

    match state {
        // ── Appareil allumé, déverrouillé ──────────────────────
        "ON" => {
            steps.push(ExtractionStep {
                order: 1, method: "ADB Logical Full".into(),
                description: "Extraction logique complète via ADB (SMS, contacts, appels, apps, médias)".into(),
                command_hint: "adb backup -apk -shared -all -f backup.ab".into(),
                success_probability: 0.95, data_expected: vec!["SMS".into(),"Contacts".into(),"Appels".into(),"Photos".into(),"Apps".into(),"WhatsApp".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "Autorisation propriétaire ou mandat requis".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 2, method: "Agent APK (FIDO-LABS Agent)".into(),
                description: "Installer APK forensique temporaire pour extraction étendue (sans root)".into(),
                command_hint: "adb install fido-agent.apk && adb shell am start com.fidolabs.agent".into(),
                success_probability: 0.90, data_expected: vec!["Galerie".into(),"Documents".into(),"DB apps".into(),"Comptes".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "APK retiré après extraction — trace dans audit log".into(), is_destructive: false,
            });
            success_rate = 0.95;
        }

        // ── Allumé + verrouillé ────────────────────────────────
        "ON_LOCKED" => {
            legal.push("Demander le code PIN/pattern au propriétaire".into());
            legal.push("Mandat judiciaire si refus".into());
            steps.push(ExtractionStep {
                order: 1, method: "Demande code propriétaire".into(),
                description: "Méthode légale prioritaire: demander le déverrouillage au propriétaire.".into(),
                command_hint: "— (procédure légale)".into(),
                success_probability: 0.70, data_expected: vec!["Accès complet".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "Obligatoire avant toute tentative technique. FIDO-LABS logue le refus éventuel.".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 2, method: "MDM Wipe (si entreprise)".into(),
                description: "Si appareil inscrit dans MDM, effacer via MDM et récupérer la sauvegarde cloud.".into(),
                command_hint: "— (via console MDM)".into(),
                success_probability: 0.50, data_expected: vec!["Cloud backup".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "Nécessite accès MDM entreprise".into(), is_destructive: true,
            });
            steps.push(ExtractionStep {
                order: 3, method: "EDL/BROM Hardware Bypass (Qualcomm/MTK)".into(),
                description: "Sur certains chipsets, accès au mode de téléchargement hardware contourne le verrou.".into(),
                command_hint: "→ Voir module BROM/EDL ci-dessous".into(),
                success_probability: 0.60, data_expected: vec!["Partition data brute".into(),"userdata.img".into()],
                requires_hardware: true, hardware_needed: vec!["Câble EDL".into(),"Box forensique (Cellebrite/Oxygen)".into(),"Test point selon modèle".into()], legal_note: "Mandat judiciaire requis".into(), is_destructive: false,
            });
            success_rate = 0.60;
        }

        // ── BROM MTK ────────────────────────────────────────────
        "OFF_BROM_MTK" => {
            equipment.push("Câble USB de qualité".into());
            equipment.push("MTKClient ou SP Flash Tool".into());
            equipment.push("Test point (selon modèle) pour forcer BROM".into());
            legal.push("Autorisation judiciaire pour lecture partition chiffrée".into());
            steps.push(ExtractionStep {
                order: 1, method: "MTK BROM Handshake".into(),
                description: "Connexion BROM via VID 0e8d:0003 — lecture directe eMMC sans démarrage OS.".into(),
                command_hint: "python mtkclient/mtk r userdata userdata.bin".into(),
                success_probability: 0.75, data_expected: vec!["userdata (chiffré Android FBE)".into(),"bootloader".into(),"preloader".into()],
                requires_hardware: true, hardware_needed: vec!["MTKClient".into(),"Câble USB".into(),"Test point éventuel".into()], legal_note: "Lecture brute — déchiffrement nécessite clés Android FDE/FBE".into(), is_destructive: false,
            });
            success_rate = 0.75;
        }

        // ── EDL Qualcomm ─────────────────────────────────────────
        "OFF_EDL_QUALCOMM" => {
            equipment.push("Câble EDL (court-circuit D+ D-)".into());
            equipment.push("QFIL / QPST".into());
            equipment.push("Firehose programmer (.elf selon SoC)".into());
            legal.push("Autorisation judiciaire obligatoire".into());
            steps.push(ExtractionStep {
                order: 1, method: "Qualcomm EDL 9008".into(),
                description: "Mode Emergency Download — lecture UFS/eMMC via protocole Sahara/Firehose.".into(),
                command_hint: "qfil /storageType:ufs /noprompt /Firehose:prog_firehose_ddr.elf".into(),
                success_probability: 0.70, data_expected: vec!["userdata.img".into(),"system.img".into(),"persist".into()],
                requires_hardware: true, hardware_needed: vec!["Câble EDL".into(),"Programmer Firehose pour le SoC ciblé".into(),"QFIL/QPST".into()], legal_note: "Partition chiffrée Android FBE — déchiffrement post-extraction nécessaire".into(), is_destructive: false,
            });
            success_rate = 0.70;
        }

        // ── Écran cassé ──────────────────────────────────────────
        "BROKEN_SCREEN" => {
            equipment.push("Adaptateur OTG".into());
            equipment.push("Souris USB".into());
            legal.push("Autorisation propriétaire ou mandat".into());
            steps.push(ExtractionStep {
                order: 1, method: "Souris OTG + Navigation écran".into(),
                description: "Brancher souris via adaptateur OTG. Si déverrouillé ou sans verrouillage, naviguer et activer ADB.".into(),
                command_hint: "adb shell settings put global development_settings_enabled 1".into(),
                success_probability: 0.80, data_expected: vec!["Accès UI complet".into(),"ADB activable".into()],
                requires_hardware: true, hardware_needed: vec!["Câble OTG".into(),"Souris USB".into()], legal_note: "Autorisation requise".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 2, method: "Moniteur externe (MHL/USB-C)".into(),
                description: "Certains téléphones supportent la sortie vidéo via USB-C DisplayPort — brancher un écran externe.".into(),
                command_hint: "— (câble USB-C → HDMI + écran)".into(),
                success_probability: 0.40, data_expected: vec!["Interface visible sur écran ext.".into()],
                requires_hardware: true, hardware_needed: vec!["Câble USB-C HDMI".into(),"Écran externe".into()], legal_note: "".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 3, method: "Scrcpy (si ADB actif)".into(),
                description: "Si le débogage USB était déjà activé, scrcpy permet mirroring + contrôle sans écran.".into(),
                command_hint: "scrcpy -s SERIAL --no-display --record sortie.mp4".into(),
                success_probability: 0.85, data_expected: vec!["Contrôle complet de l'appareil".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "ADB doit être déjà autorisé sur cet appareil".into(), is_destructive: false,
            });
            success_rate = 0.80;
        }

        // ── Écran cassé + verrouillé ─────────────────────────────
        "BROKEN_SCREEN_LOCKED" => {
            equipment.push("Adaptateur OTG + souris".into());
            equipment.push("Câble EDL / test point (selon chipset)".into());
            equipment.push("Box forensique certifiée (Cellebrite UFED, MSAB XRY)".into());
            legal.push("Mandat judiciaire OBLIGATOIRE".into());
            legal.push("Rapport d'état de l'appareil avant toute intervention".into());
            steps.push(ExtractionStep {
                order: 1, method: "Vérifier ADB pré-autorisé".into(),
                description: "Si le débogage USB était activé avant la cassure, ADB fonctionne encore.".into(),
                command_hint: "adb devices".into(),
                success_probability: 0.35, data_expected: vec!["Accès logique complet si ADB actif".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "Vérifier avant toute autre méthode".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 2, method: "OTG + Blind Navigation".into(),
                description: "Souris OTG + navigation à l'aveugle selon layout Android connu du modèle (tap positions).".into(),
                command_hint: "adb shell input tap X Y  # positions selon modèle exact".into(),
                success_probability: 0.30, data_expected: vec!["Déverrouillage si positions correctes".into()],
                requires_hardware: true, hardware_needed: vec!["Adaptateur OTG".into(),"Souris USB".into()], legal_note: "Mandat requis".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 3, method: "BROM/EDL Hardware (MTK/Qualcomm)".into(),
                description: "Accès hardware direct sans OS — lit la partition data brute (chiffrée).".into(),
                command_hint: "→ Voir modules BROM/EDL".into(),
                success_probability: 0.65, data_expected: vec!["userdata.img (chiffré FBE)".into()],
                requires_hardware: true, hardware_needed: vec!["Câble EDL".into(),"Programmeur Firehose".into(),"Test point".into()], legal_note: "Mandat judiciaire obligatoire — résultat chiffré nécessite déchiffrement".into(), is_destructive: false,
            });
            steps.push(ExtractionStep {
                order: 4, method: "ISP (In-System Programming)".into(),
                description: "Connexion directe sur les pads ISP de la puce eMMC/UFS sans démonter la puce.".into(),
                command_hint: "— (matériel spécialisé ISP + pinouts selon modèle)".into(),
                success_probability: 0.55, data_expected: vec!["Dump complet eMMC/UFS".into()],
                requires_hardware: true, hardware_needed: vec!["Station ISP".into(),"Pinouts du modèle".into(),"Expertise soudure".into()], legal_note: "Technique avancée — risque d'endommagement".into(), is_destructive: true,
            });
            steps.push(ExtractionStep {
                order: 5, method: "Chip-Off (dernier recours)".into(),
                description: "Déchippage de la puce eMMC/UFS pour lecture sur lecteur externe. Irréversible.".into(),
                command_hint: "— (déchippage + lecteur eMMC spécialisé)".into(),
                success_probability: 0.50, data_expected: vec!["Dump eMMC brut".into(),"Reconstituable si non chiffré".into()],
                requires_hardware: true, hardware_needed: vec!["Station de rebillage".into(),"Lecteur eMMC/UFS".into(),"Expertise hardware".into()], legal_note: "DESTRUCTIF — à utiliser uniquement si toutes les autres méthodes ont échoué. Mandat obligatoire.".into(), is_destructive: true,
            });
            success_rate = 0.65;
        }

        _ => {
            steps.push(ExtractionStep {
                order: 1, method: "Diagnostic complet".into(),
                description: "Brancher l'appareil et analyser VID/PID USB pour déterminer le mode.".into(),
                command_hint: "adb devices && lsusb".into(),
                success_probability: 0.50, data_expected: vec!["Identification état".into()],
                requires_hardware: false, hardware_needed: vec![], legal_note: "".into(), is_destructive: false,
            });
            success_rate = 0.50;
        }
    }

    ExtractionPlan {
        id: Uuid::new_v4().to_string(),
        device_state: state.into(),
        recommended_methods: steps,
        physical_equipment_needed: equipment,
        legal_requirements: if legal.is_empty() { vec!["Autorisation du propriétaire ou mandat judiciaire".into()] } else { legal },
        estimated_success_rate: success_rate,
        risk_level: match state {
            "BROKEN_SCREEN_LOCKED" => "HIGH".into(),
            "OFF_BROM_MTK" | "OFF_EDL_QUALCOMM" => "MEDIUM".into(),
            _ => "LOW".into(),
        },
        alternative_sources: legitimate_sources(state),
    }
}

#[tauri::command]
pub fn get_broken_screen_options(serial: String) -> Result<BrokenScreenOptions, String> {
    // Tester ADB
    let adb_ok = Command::new("adb").args(["-s", &serial, "shell", "echo", "ok"]).output()
        .map(|o| o.status.success()).unwrap_or(false);

    // Tester scrcpy disponible
    let scrcpy_ok = Command::new("scrcpy").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false);

    Ok(BrokenScreenOptions {
        otg_mouse_possible: true, // toujours possible si port USB OK
        adb_already_enabled: adb_ok,
        scrcpy_available: scrcpy_ok,
        usb_data_works: adb_ok,
        backup_display_possible: false, // non détectable sans test physique
        isp_required: !adb_ok,
    })
}

#[tauri::command]
pub fn attempt_adb_unlock_navigation(serial: String, device_model: String) -> Result<String, String> {
    // Navigation à l'aveugle selon positions connues (démos uniquement)
    let layouts: std::collections::HashMap<&str, (i32, i32)> = [
        ("samsung", (540, 1800)),
        ("tecno",   (540, 1700)),
        ("infinix", (540, 1750)),
        ("xiaomi",  (540, 1900)),
        ("generic", (540, 1600)),
    ].iter().cloned().collect();

    let model_lower = device_model.to_lowercase();
    let brand = layouts.keys().find(|&&k| model_lower.contains(k)).unwrap_or(&"generic");
    let (x, y) = layouts[brand];

    // Simuler tap sur position de swipe
    let result = Command::new("adb")
        .args(["-s", &serial, "shell", "input", "swipe",
               &x.to_string(), &(y+200).to_string(),
               &x.to_string(), &(y-200).to_string(), "300"])
        .output()
        .map(|_| format!("Swipe envoyé sur ({},{}) pour modèle {} ({})", x, y, device_model, brand))
        .map_err(|e| format!("ADB indisponible: {}", e))?;

    Ok(result)
}

#[tauri::command]
pub fn start_scrcpy_session(serial: String) -> Result<String, String> {
    // Lancer scrcpy en arrière-plan (mirroring)
    Command::new("scrcpy")
        .args(["-s", &serial, "--window-title", "FIDO-LABS Mirroring",
               "--stay-awake", "--disable-screensaver"])
        .spawn()
        .map(|_| format!("scrcpy démarré pour appareil {}", serial))
        .map_err(|_| "scrcpy non installé — téléchargez depuis github.com/Genymobile/scrcpy".into())
}

#[tauri::command]
pub fn get_extraction_matrix() -> Vec<ExtractionMatrixRow> {
    vec![
        ExtractionMatrixRow { chipset:"MTK (MediaTek)".into(), method:"BROM (VID 0e8d:0003)".into(), state:"OFF/locked".into(), tool:"MTKClient".into(), data_access:"Raw eMMC (chiffré FBE)".into(), difficulty:"Medium".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Qualcomm".into(), method:"EDL 9008".into(), state:"OFF/locked".into(), tool:"QFIL + Firehose".into(), data_access:"UFS dump (chiffré)".into(), difficulty:"Medium".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Any Android".into(), method:"ADB Logical".into(), state:"ON + déverrouillé".into(), tool:"adb/adb backup".into(), data_access:"Apps, SMS, Contacts, Médias".into(), difficulty:"Easy".into(), hardware_required:false },
        ExtractionMatrixRow { chipset:"Any Android".into(), method:"Agent APK".into(), state:"ON + déverrouillé".into(), tool:"FIDO-LABS Agent".into(), data_access:"Base de données apps, galerie".into(), difficulty:"Easy".into(), hardware_required:false },
        ExtractionMatrixRow { chipset:"Any".into(), method:"OTG + scrcpy".into(), state:"Écran cassé (ADB déjà actif)".into(), tool:"scrcpy".into(), data_access:"Interface complète mirrorée".into(), difficulty:"Easy".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Any".into(), method:"OTG + souris aveugle".into(), state:"Écran cassé + verrouillé".into(), tool:"adb input".into(), data_access:"Partiel (si positions correctes)".into(), difficulty:"Hard".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Any".into(), method:"ISP".into(), state:"Tout état (USB mort)".into(), tool:"ISP pinouts + station".into(), data_access:"Dump complet eMMC/UFS".into(), difficulty:"Expert".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Any".into(), method:"Chip-Off".into(), state:"Dernier recours".into(), tool:"Station rebillage + lecteur eMMC".into(), data_access:"Dump brut (DESTRUCTIF)".into(), difficulty:"Expert".into(), hardware_required:true },
        ExtractionMatrixRow { chipset:"Apple (A-series)".into(), method:"iTunes Backup".into(), state:"iOS déverrouillé".into(), tool:"libimobiledevice".into(), data_access:"Messages, contacts, applis".into(), difficulty:"Easy".into(), hardware_required:false },
        ExtractionMatrixRow { chipset:"Apple A5-A11 (checkm8)".into(), method:"checkm8 DFU".into(), state:"DFU mode".into(), tool:"checkra1n".into(), data_access:"Full filesystem iOS".into(), difficulty:"Medium".into(), hardware_required:false },
    ]
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExtractionMatrixRow {
    pub chipset: String,
    pub method: String,
    pub state: String,
    pub tool: String,
    pub data_access: String,
    pub difficulty: String,
    pub hardware_required: bool,
}

// ─── EXTRACTION PHYSIQUE MediaTek (BROM) via mtkclient ────────────────────────
// Pilote l'outil open-source mtkclient (bkerler/mtkclient) : lit la mémoire flash
// en mode BootROM, SANS déverrouiller ni contourner le verrou. Le dump obtenu est
// la partition telle quelle (chiffrée si FBE — lisible seulement en état AFU).

fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Localise le python du venv et le script mtk.py (env override puis chemins candidats).
fn resolve_mtk() -> Result<(String, String), String> {
    if let Ok(script) = std::env::var("FIDO_MTK_SCRIPT") {
        if std::path::Path::new(&script).exists() {
            let py = std::env::var("FIDO_MTK_PYTHON").unwrap_or_else(|_| "python".into());
            return Ok((py, script));
        }
    }
    // Layout dev / installeur (venv à côté du script)
    let candidates = [
        ("E:/fido-labs-complete/fido-labs-complete/tools/mtk-venv/Scripts/python.exe",
         "E:/fido-labs-complete/fido-labs-complete/tools/mtkclient-main/mtk.py"),
        ("tools/mtk-venv/Scripts/python.exe", "tools/mtkclient-main/mtk.py"),
        ("mtkclient/mtk-venv/Scripts/python.exe", "mtkclient/mtk.py"),
    ];
    for (py, script) in candidates {
        if std::path::Path::new(script).exists() {
            let python = if std::path::Path::new(py).exists() { py.to_string() } else { "python".into() };
            return Ok((python, script.to_string()));
        }
    }
    Err("mtkclient introuvable. Installez-le (tools/mtkclient-main) ou définissez la variable FIDO_MTK_SCRIPT.".into())
}

/// Extraction physique MediaTek via mtkclient (mode BROM).
/// mode: "gpt" (lister partitions), "userdata" (dump userdata), "full" (toutes partitions).
#[tauri::command]
pub fn extract_physical_mtk(case_id: String, output_dir: String, mode: String) -> Result<serde_json::Value, String> {
    use sha2::Digest;

    let (python, script) = resolve_mtk()?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/mtk_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), sanitize_id(&case_id), ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;

    let args: Vec<String> = match mode.as_str() {
        "gpt"      => vec!["printgpt".into()],
        "userdata" => vec!["r".into(), "userdata".into(), format!("{}/userdata.bin", base)],
        "full"     => vec!["rl".into(), base.clone()],
        other      => return Err(format!("Mode inconnu: {}", other)),
    };

    let script_dir = std::path::Path::new(&script).parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let out = Command::new(&python)
        .arg(&script)
        .args(&args)
        .current_dir(&script_dir)
        .output()
        .map_err(|e| format!("Lancement mtkclient échoué ({}): {}", python, e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    std::fs::write(
        format!("{}/mtk_log.txt", base),
        format!("$ {} {} {}\n\n=== STDOUT ===\n{}\n=== STDERR ===\n{}", python, script, args.join(" "), stdout, stderr),
    ).ok();

    // Hasher les dumps produits (chaîne de garde)
    let mut dumps = Vec::new();
    for entry in walkdir::WalkDir::new(&base).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".bin") || name.ends_with(".img") {
            if let Ok(data) = std::fs::read(p) {
                let mut h = sha2::Sha256::new();
                h.update(&data);
                dumps.push(serde_json::json!({
                    "file": p.to_string_lossy(),
                    "size": data.len(),
                    "sha256": hex::encode(h.finalize()),
                }));
            }
        }
    }

    let ok = out.status.success();
    let n = stdout.chars().count();
    let excerpt: String = stdout.chars().skip(n.saturating_sub(1500)).collect();
    Ok(serde_json::json!({
        "success": ok,
        "summary": if ok {
            format!("Extraction physique MTK terminée — {} dump(s)", dumps.len())
        } else {
            "mtkclient n'a pas abouti — appareil en mode BROM ? pilote USB (UsbDk/libusb) installé ? Voir le log.".to_string()
        },
        "mode": mode,
        "output_dir": base,
        "dumps": dumps,
        "log": excerpt,
    }))
}

// ─── EXTRACTION PHYSIQUE Qualcomm (EDL 9008) via edl (bkerler/edl) ─────────────
// Mode Emergency Download (Sahara/Firehose). Nécessite un loader firehose
// (prog_firehose_*.elf) spécifique au SoC pour la plupart des appareils.

fn resolve_edl() -> Result<(String, String), String> {
    if let Ok(script) = std::env::var("FIDO_EDL_SCRIPT") {
        if std::path::Path::new(&script).exists() {
            let py = std::env::var("FIDO_EDL_PYTHON").unwrap_or_else(|_| "python".into());
            return Ok((py, script));
        }
    }
    let candidates = [
        ("E:/fido-labs-complete/fido-labs-complete/tools/edl-venv/Scripts/python.exe",
         "E:/fido-labs-complete/fido-labs-complete/tools/edl-master/edl.py"),
        ("tools/edl-venv/Scripts/python.exe", "tools/edl-master/edl.py"),
    ];
    for (py, script) in candidates {
        if std::path::Path::new(script).exists() {
            let python = if std::path::Path::new(py).exists() { py.to_string() } else { "python".into() };
            return Ok((python, script.to_string()));
        }
    }
    Err("edl introuvable (tools/edl-master) — définissez FIDO_EDL_SCRIPT.".into())
}

/// Extraction physique Qualcomm via edl (mode EDL 9008).
/// mode: "gpt" | "userdata" | "full". loader: chemin optionnel vers un firehose .elf.
#[tauri::command]
pub fn extract_physical_edl(case_id: String, output_dir: String, mode: String, loader: String) -> Result<serde_json::Value, String> {
    use sha2::Digest;

    let (python, script) = resolve_edl()?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/edl_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == '\\'), sanitize_id(&case_id), ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Création {}: {}", base, e))?;

    let mut args: Vec<String> = match mode.as_str() {
        "gpt"      => vec!["printgpt".into()],
        "userdata" => vec!["r".into(), "userdata".into(), format!("{}/userdata.bin", base)],
        "full"     => vec!["rl".into(), base.clone()],
        other      => return Err(format!("Mode inconnu: {}", other)),
    };
    if !loader.trim().is_empty() {
        args.push("--loader".into());
        args.push(loader.trim().to_string());
    }

    let script_dir = std::path::Path::new(&script).parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let out = Command::new(&python)
        .arg(&script)
        .args(&args)
        .current_dir(&script_dir)
        .output()
        .map_err(|e| format!("Lancement edl échoué ({}): {}", python, e))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    std::fs::write(
        format!("{}/edl_log.txt", base),
        format!("$ {} {} {}\n\n=== STDOUT ===\n{}\n=== STDERR ===\n{}", python, script, args.join(" "), stdout, stderr),
    ).ok();

    let mut dumps = Vec::new();
    for entry in walkdir::WalkDir::new(&base).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".bin") || name.ends_with(".img") {
            if let Ok(data) = std::fs::read(p) {
                let mut h = sha2::Sha256::new();
                h.update(&data);
                dumps.push(serde_json::json!({
                    "file": p.to_string_lossy(),
                    "size": data.len(),
                    "sha256": hex::encode(h.finalize()),
                }));
            }
        }
    }

    let ok = out.status.success();
    let n = stdout.chars().count();
    let excerpt: String = stdout.chars().skip(n.saturating_sub(1500)).collect();
    Ok(serde_json::json!({
        "success": ok,
        "summary": if ok {
            format!("Extraction EDL terminée — {} dump(s)", dumps.len())
        } else {
            "edl n'a pas abouti — appareil en mode EDL 9008 ? Loader firehose (.elf) fourni ? Voir le log.".to_string()
        },
        "mode": mode,
        "output_dir": base,
        "dumps": dumps,
        "log": excerpt,
    }))
}
