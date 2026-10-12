// CHIPSETS MODULE v1 - Base de profils chipsets Android récents (MediaTek, Qualcomm,
// Samsung Exynos, Google Tensor, Unisoc, HiSilicon) + détection automatique.
// Usage légal uniquement avec autorisation judiciaire.
//
// La détection combine 3 signaux :
//   1) VID/PID USB des modes spéciaux (BROM/EDL) → plateforme (MTK/Qualcomm) ;
//   2) Props ADB (`ro.board.platform`, `ro.hardware`, `ro.soc.model`,
//      `ro.product.board`, `ro.chipname`) → identification du SoC précis ;
//   3) Correspondance fabricant/modèle connu → marque + profil d'extraction.
// Aucun contournement : on identifie et on documente, rien de plus.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChipsetProfile {
    pub soc: String,                // nom commercial du SoC (ex. "Dimensity 7200")
    pub vendor: String,             // MediaTek | Qualcomm | Samsung | Google | Unisoc | HiSilicon | Autre
    pub platform: String,           // plateforme/proc (ex. "MT6885", "SM8350", "Tensor G2")
    pub codename: String,           // nom interne (ex. "k6853v1_64")
    pub cpu: String,                // description CPU
    pub gpu: String,
    pub process: String,            // finesse de gravure (nm)
    pub android_min: String,        // version Android minimale courante
    pub extraction_modes: Vec<String>, // BROM / EDL / ADB / Fastboot…
    pub brom_vid_pid: Vec<String>,  // couples VID:PID pour le mode BROM (MTK)
    pub edl_vid_pid: Vec<String>,   // couples VID:PID pour le mode EDL (Qualcomm)
    pub notes: String,              // remarques d'acquisition
    pub recent: bool,               // flag « génération récente »
}

/// Recherche par VID/PID (mode spécial USB). Retourne un profil plateforme générique
/// quand le SoC exact n'est pas connu.
pub fn detect_by_vidpid(vid: &str, pid: &str) -> Option<ChipsetProfile> {
    let (vid, pid) = (vid.to_lowercase(), pid.to_lowercase());
    match (vid.as_str(), pid.as_str()) {
        // MediaTek BROM (VID 0e8d)
        ("0e8d", _) => Some(generic_mtk()),
        // Qualcomm EDL / fastboot / diagnostic (VID 05c6)
        ("05c6", "9008") | ("05c6", "900e") | ("05c6", "9025") | ("05c6", _) => Some(generic_qcom()),
        // Samsung
        ("04e8", _) => Some(ChipsetProfile {
            soc: "Samsung (Exynos/Snapdragon)".into(), vendor: "Samsung".into(),
            platform: "—".into(), codename: "—".into(),
            cpu: "Dépend du modèle (Exynos ou Snapdragon)".into(), gpu: "—".into(),
            process: "—".into(), android_min: "8.0".into(),
            extraction_modes: vec!["ADB".into(), "EDL (modèles Snapdragon)".into()],
            brom_vid_pid: vec![], edl_vid_pid: vec!["05c6:9008".into()],
            notes: "Samsung EDL (Qualcomm) : nécessite un loader firehose selon modèle ; les Exynos n'ont pas d'EDL public.".into(),
            recent: true,
        }),
        // Google Pixel (VID 18d1)
        ("18d1", _) => Some(ChipsetProfile {
            soc: "Google Tensor".into(), vendor: "Google".into(),
            platform: "gs".into(), codename: "tensor".into(),
            cpu: "ARM Cortex-X/A (8 cœurs)".into(), gpu: "Mali-G78/Immortalis".into(),
            process: "5 nm".into(), android_min: "12".into(),
            extraction_modes: vec!["ADB".into(), "Fastboot".into()],
            brom_vid_pid: vec![], edl_vid_pid: vec![],
            notes: "Pixel : pas de BROM/EDL public exploitable. Extraction logique ADB ou fastboot (bootloader déverrouillé).".into(),
            recent: true,
        }),
        // Xiaomi (VID 2717)
        ("2717", _) => Some(ChipsetProfile {
            soc: "Xiaomi (MTK ou Snapdragon)".into(), vendor: "Xiaomi".into(),
            platform: "—".into(), codename: "—".into(),
            cpu: "Dépend du modèle".into(), gpu: "—".into(), process: "—".into(),
            android_min: "8.0".into(),
            extraction_modes: vec!["ADB".into(), "BROM (modèles MTK)".into(), "EDL (modèles Snapdragon)".into()],
            brom_vid_pid: vec!["0e8d:0003".into()], edl_vid_pid: vec!["05c6:9008".into()],
            notes: "Xiaomi/Redmi/POCO : varient selon SoC — détecter le SoC via props ADB pour la bonne méthode.".into(),
            recent: true,
        }),
        // Huawei (VID 12d1)
        ("12d1", _) => Some(ChipsetProfile {
            soc: "Huawei (HiSilicon Kirin)".into(), vendor: "HiSilicon".into(),
            platform: "kirin".into(), codename: "—".into(),
            cpu: "ARM Cortex (Kirin)".into(), gpu: "Mali".into(), process: "7/5 nm".into(),
            android_min: "8.0".into(),
            extraction_modes: vec!["ADB".into()],
            brom_vid_pid: vec![], edl_vid_pid: vec![],
            notes: "HiSilicon : pas de point d'accès BROM/EDL public fiable — extraction logique ADB privilégiée.".into(),
            recent: true,
        }),
        // OnePlus / Oppo (VID 22d9) et autres marques
        ("22d9", _) => Some(ChipsetProfile {
            soc: "Oppo/OnePlus/Realme".into(), vendor: "Qualcomm/MediaTek".into(),
            platform: "—".into(), codename: "—".into(),
            cpu: "Dépend du modèle".into(), gpu: "—".into(), process: "—".into(),
            android_min: "8.0".into(),
            extraction_modes: vec!["ADB".into(), "EDL (Snapdragon)".into(), "BROM (MTK)".into()],
            brom_vid_pid: vec!["0e8d:0003".into()], edl_vid_pid: vec!["05c6:9008".into()],
            notes: "BBK Electronics : détecter le SoC exact pour choisir BROM vs EDL.".into(),
            recent: true,
        }),
        _ => None,
    }
}

fn generic_mtk() -> ChipsetProfile {
    ChipsetProfile {
        soc: "MediaTek (modèle à identifier)".into(), vendor: "MediaTek".into(),
        platform: "MTK".into(), codename: "—".into(),
        cpu: "ARM (dépend du SoC)".into(), gpu: "Mali / PowerVR".into(),
        process: "—".into(), android_min: "7.0".into(),
        extraction_modes: vec!["BROM (VID 0e8d)".into(), "ADB".into()],
        brom_vid_pid: vec!["0e8d:0003".into(), "0e8d:2000".into()], edl_vid_pid: vec![],
        notes: "Mode BROM MediaTek : test point ou trou de téléchargement pour forcer l'accès ; mtkclient requis.".into(),
        recent: true,
    }
}

fn generic_qcom() -> ChipsetProfile {
    ChipsetProfile {
        soc: "Qualcomm Snapdragon (à identifier)".into(), vendor: "Qualcomm".into(),
        platform: "SM".into(), codename: "—".into(),
        cpu: "ARM Kryo".into(), gpu: "Adreno".into(), process: "—".into(),
        android_min: "7.0".into(),
        extraction_modes: vec!["EDL 9008".into(), "ADB".into()],
        brom_vid_pid: vec![], edl_vid_pid: vec!["05c6:9008".into()],
        notes: "Mode EDL 9008 Qualcomm : loader firehose .elf spécifique au SoC requis ; câble EDL ou test point.".into(),
        recent: true,
    }
}

/// Exécute `adb -s SERIAL shell getprop PROP` et renvoie la valeur (vide si échec).
fn adb_getprop(serial: &str, prop: &str) -> String {
    Command::new(&super::find_adb())
        .args(["-s", serial, "shell", "getprop", prop])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Base de correspondance plateforme/proc → SoC commercial (générations récentes).
/// Clés : valeurs de ro.board.platform / ro.soc.model / ro.chipname possibles.
fn match_soc(platform: &str) -> Option<ChipsetProfile> {
    let p = platform.to_lowercase();
    // ── MediaTek (Dimensity / Helio) ──────────────────────────────
    let mtk = |name: &str, proc: &str, cpu: &str, gpu: &str, nm: &str| ChipsetProfile {
        soc: name.into(), vendor: "MediaTek".into(), platform: proc.into(), codename: p.clone(),
        cpu: cpu.into(), gpu: gpu.into(), process: nm.into(), android_min: "11".into(),
        extraction_modes: vec!["BROM (VID 0e8d)".into(), "ADB".into()],
        brom_vid_pid: vec!["0e8d:0003".into(), "0e8d:2000".into()], edl_vid_pid: vec![],
        notes: "MediaTek récent : mode BROM accessible via test point ; mtkclient (bkerler) supporte la plupart des SoC.".into(),
        recent: true,
    };
    // Dimensity 9000+
    if p.contains("mt6983") || p.contains("t6983") { return Some(mtk("Dimensity 9000/9000+", "MT6983", "1x Cortex-X2 + 3x A710 + 4x A510", "Mali-G710 MC10", "4 nm")); }
    if p.contains("mt6980") || p.contains("t6980") { return Some(mtk("Dimensity 9200", "MT6985", "1x Cortex-X3 + 3x A715 + 4x A510", "Immortalis-G715 MC11", "4 nm")); }
    if p.contains("mt6985") || p.contains("t6985") { return Some(mtk("Dimensity 9300", "MT6989", "4x Cortex-X4 + 4x A720", "Immortalis-G720 MC12", "4 nm")); }
    if p.contains("mt6989") || p.contains("t6989") { return Some(mtk("Dimensity 9400", "MT6991", "1x Cortex-X925 + 3x X4 + 4x A720", "Immortalis-G925", "3 nm")); }
    // Dimensity 8000/8300
    if p.contains("mt6895") || p.contains("t6895") { return Some(mtk("Dimensity 8100/8200", "MT6895", "4x Cortex-A78 + 4x A55", "Mali-G610 MC6", "5 nm")); }
    if p.contains("mt6897") || p.contains("t6897") { return Some(mtk("Dimensity 8300", "MT6897", "1x Cortex-X3 + 3x A715 + 4x A510", "Mali-G615 MC6", "4 nm")); }
    // Dimensity 7000
    if p.contains("mt6878") || p.contains("t6878") { return Some(mtk("Dimensity 7200/7300", "MT6878", "2x Cortex-A715 + 6x A510", "Mali-G610 MC4", "4 nm")); }
    if p.contains("mt6877") || p.contains("t6877") { return Some(mtk("Dimensity 1300/1200", "MT6893", "1x Cortex-A78 + 3x A78 + 4x A55", "Mali-G77 MC9", "6 nm")); }
    if p.contains("mt6873") || p.contains("t6873") { return Some(mtk("Dimensity 720/1000", "MT6885", "4x Cortex-A77 + 4x A55", "Mali-G77 MC9", "7 nm")); }
    if p.contains("mt6885") || p.contains("t6885") { return Some(mtk("Dimensity 1000/1100", "MT6885", "4x Cortex-A77 + 4x A55", "Mali-G77 MC9", "7 nm")); }
    if p.contains("mt6886") || p.contains("t6886") { return Some(mtk("Dimensity 1200", "MT6893", "1x Cortex-A78 + 3x A78 + 4x A55", "Mali-G77 MC9", "6 nm")); }
    if p.contains("mt6893") || p.contains("t6893") { return Some(mtk("Dimensity 1200", "MT6893", "1x Cortex-A78 + 3x A78 + 4x A55", "Mali-G77 MC9", "6 nm")); }
    if p.contains("mt6833") || p.contains("t6833") { return Some(mtk("Dimensity 700/810", "MT6833", "2x Cortex-A76 + 6x A55", "Mali-G57 MC2", "7 nm")); }
    if p.contains("mt6835") || p.contains("t6835") { return Some(mtk("Dimensity 6100+/6300", "MT6835", "2x Cortex-A76 + 6x A55", "Mali-G57 MC2", "6 nm")); }
    if p.contains("mt6855") || p.contains("t6855") { return Some(mtk("Dimensity 6020/6080", "MT6833", "2x Cortex-A76 + 6x A55", "Mali-G57 MC2", "7 nm")); }
    if p.contains("mt6765") || p.contains("t6765") { return Some(mtk("Helio G35/G37", "MT6765", "4x Cortex-A53 + 4x A53", "PowerVR GE8320", "12 nm")); }
    if p.contains("mt6768") || p.contains("t6768") { return Some(mtk("Helio G70/G80", "MT6768", "2x Cortex-A75 + 6x A55", "Mali-G52 MC2", "12 nm")); }
    if p.contains("mt6769") || p.contains("t6769") { return Some(mtk("Helio G85/G90", "MT6769", "2x Cortex-A75 + 6x A55", "Mali-G52 MC2", "12 nm")); }
    if p.contains("mt6789") || p.contains("t6789") { return Some(mtk("Helio G99", "MT6789", "2x Cortex-A76 + 6x A55", "Mali-G57 MC2", "6 nm")); }
    if p.contains("mt6785") || p.contains("t6785") { return Some(mtk("Helio G95/G96", "MT6785", "2x Cortex-A76 + 6x A55", "Mali-G76 MC4", "12 nm")); }
    if p.contains("mt6833") || p.contains("t6833") { return Some(mtk("Dimensity 700/810", "MT6833", "2x Cortex-A76 + 6x A55", "Mali-G57 MC2", "7 nm")); }

    // ── Qualcomm Snapdragon ───────────────────────────────────────
    let qcom = |name: &str, proc: &str, cpu: &str, gpu: &str, nm: &str| ChipsetProfile {
        soc: name.into(), vendor: "Qualcomm".into(), platform: proc.into(), codename: p.clone(),
        cpu: cpu.into(), gpu: gpu.into(), process: nm.into(), android_min: "10".into(),
        extraction_modes: vec!["EDL 9008".into(), "ADB".into()],
        brom_vid_pid: vec![], edl_vid_pid: vec!["05c6:9008".into()],
        notes: "Snapdragon : mode EDL 9008 via câble EDL/test point ; loader firehose (prog_firehose_ddr.elf) spécifique au SoC requis.".into(),
        recent: true,
    };
    if p.contains("sm8750") || p.contains("sm8650") { return Some(qcom("Snapdragon 8 Gen 4/5", "SM8750", "Oryon (2+6)", "Adreno 830", "3 nm")); }
    if p.contains("sm8650") { return Some(qcom("Snapdragon 8 Gen 3", "SM8650", "1x Cortex-X4 + 5x A720 + 2x A520", "Adreno 750", "4 nm")); }
    if p.contains("sm8550") { return Some(qcom("Snapdragon 8 Gen 2", "SM8550", "1x Cortex-X3 + 4x A715 + 3x A510", "Adreno 740", "4 nm")); }
    if p.contains("sm8475") || p.contains("sm8450") { return Some(qcom("Snapdragon 8+ Gen 1 / 8 Gen 1", "SM8475", "1x Cortex-X2 + 3x A710 + 4x A510", "Adreno 730", "4 nm")); }
    if p.contains("sm8550") { return Some(qcom("Snapdragon 8 Gen 2", "SM8550", "1x Cortex-X3 + 4x A715 + 3x A510", "Adreno 740", "4 nm")); }
    if p.contains("sm8350") { return Some(qcom("Snapdragon 888", "SM8350", "1x Cortex-X1 + 3x A78 + 4x A55", "Adreno 660", "5 nm")); }
    if p.contains("sm8250") { return Some(qcom("Snapdragon 865/870", "SM8250", "1x Cortex-A77 + 3x A77 + 4x A55", "Adreno 650", "7 nm")); }
    if p.contains("sm8150") { return Some(qcom("Snapdragon 855/860", "SM8150", "1x Cortex-A76 + 3x A76 + 4x A55", "Adreno 640", "7 nm")); }
    if p.contains("sm7780") || p.contains("sm7325") { return Some(qcom("Snapdragon 778G/782G", "SM7325", "1x Cortex-A78 + 3x A78 + 4x A55", "Adreno 642L", "6 nm")); }
    if p.contains("sm7325") { return Some(qcom("Snapdragon 7 Gen 1", "SM7450", "1x Cortex-A710 + 3x A710 + 4x A510", "Adreno 644", "4 nm")); }
    if p.contains("sm7450") { return Some(qcom("Snapdragon 7 Gen 1", "SM7450", "1x Cortex-A710 + 3x A710 + 4x A510", "Adreno 644", "4 nm")); }
    if p.contains("sm6375") || p.contains("sm6375") { return Some(qcom("Snapdragon 695", "SM6375", "2x Cortex-A78 + 6x A55", "Adreno 619", "6 nm")); }
    if p.contains("sm6225") || p.contains("sm6225") { return Some(qcom("Snapdragon 680/685", "SM6225", "4x Cortex-A73 + 4x A53", "Adreno 610", "6 nm")); }
    if p.contains("sm6125") || p.contains("sm6125") { return Some(qcom("Snapdragon 665", "SM6125", "4x Cortex-A73 + 4x A53", "Adreno 610", "11 nm")); }
    if p.contains("sm7150") { return Some(qcom("Snapdragon 730/732", "SM7150", "2x Cortex-A76 + 6x A55", "Adreno 618", "8 nm")); }
    if p.contains("sm4350") { return Some(qcom("Snapdragon 480/690", "SM4350", "2x Cortex-A76 + 6x A55", "Adreno 619", "8 nm")); }
    if p.contains("sm4250") { return Some(qcom("Snapdragon 460", "SM4250", "4x Cortex-A73 + 4x A53", "Adreno 610", "11 nm")); }

    // ── Google Tensor ─────────────────────────────────────────────
    let tensor = |name: &str, cpu: &str, gpu: &str, nm: &str| ChipsetProfile {
        soc: name.into(), vendor: "Google".into(), platform: "gs".into(), codename: p.clone(),
        cpu: cpu.into(), gpu: gpu.into(), process: nm.into(), android_min: "12".into(),
        extraction_modes: vec!["ADB".into(), "Fastboot".into()],
        brom_vid_pid: vec![], edl_vid_pid: vec![],
        notes: "Tensor : pas de point d'accès BROM/EDL public — ADB/fastboot uniquement.".into(),
        recent: true,
    };
    if p.contains("gs201") { return Some(tensor("Tensor G2", "2x Cortex-X1 + 2x A78 + 4x A55", "Mali-G710 MP7", "5 nm")); }
    if p.contains("gs101") { return Some(tensor("Tensor G1", "2x Cortex-X1 + 2x A78 + 4x A55", "Mali-G78 MP20", "5 nm")); }
    if p.contains("gs202") { return Some(tensor("Tensor G3", "1x Cortex-X3 + 4x A715 + 4x A510", "Mali-G715 MP7", "4 nm")); }
    if p.contains("gs203") || p.contains("zuma") { return Some(tensor("Tensor G4", "1x Cortex-X4 + 3x A720 + 4x A520", "Mali-G715 MP7", "4 nm")); }
    if p.contains("gordon_peak") || p.contains("gordon") { return Some(tensor("Tensor G5", "1x Cortex-X4 + 5x A720", "Mali-G815", "3 nm")); }

    // ── Samsung Exynos ────────────────────────────────────────────
    let exynos = |name: &str, proc: &str, cpu: &str, gpu: &str, nm: &str| ChipsetProfile {
        soc: name.into(), vendor: "Samsung".into(), platform: proc.into(), codename: p.clone(),
        cpu: cpu.into(), gpu: gpu.into(), process: nm.into(), android_min: "10".into(),
        extraction_modes: vec!["ADB".into()],
        brom_vid_pid: vec![], edl_vid_pid: vec![],
        notes: "Exynos : pas d'EDL public ; extraction logique ADB privilégiée.".into(),
        recent: true,
    };
    if p.contains("s5e9945") || p.contains("exynos9945") { return Some(exynos("Exynos 2400", "S5E9945", "1x Cortex-X4 + 2x A720 + 3x A720 + 4x A520", "Xclipse 940", "4 nm")); }
    if p.contains("s5e9935") || p.contains("exynos9935") { return Some(exynos("Exynos 2200", "S5E9925", "1x Cortex-X2 + 3x A710 + 4x A510", "Xclipse 920 (AMD RDNA2)", "4 nm")); }
    if p.contains("s5e8825") || p.contains("exynos1280") { return Some(exynos("Exynos 1280", "S5E8825", "2x Cortex-A78 + 6x A55", "Mali-G68 MC4", "5 nm")); }
    if p.contains("s5e8835") || p.contains("exynos1380") { return Some(exynos("Exynos 1380", "S5E8835", "4x Cortex-A78 + 4x A55", "Mali-G68 MP5", "5 nm")); }
    if p.contains("s5e8845") || p.contains("exynos1480") { return Some(exynos("Exynos 1480", "S5E8845", "4x Cortex-A78 + 4x A55", "Xclipse 530", "4 nm")); }
    if p.contains("s5e8535") || p.contains("exynos1330") { return Some(exynos("Exynos 1330", "S5E8535", "2x Cortex-A78 + 6x A55", "Mali-G68 MC2", "5 nm")); }

    // ── Unisoc (Spreadtrum) ───────────────────────────────────────
    if p.contains("ums9230") || p.contains("t606") { return Some(ChipsetProfile {
        soc: "Unisoc T606/T612".into(), vendor: "Unisoc".into(), platform: "UMS9230".into(),
        codename: p.clone(), cpu: "2x Cortex-A75 + 6x A55".into(), gpu: "Mali-G57 MP1".into(),
        process: "12 nm".into(), android_min: "10".into(),
        extraction_modes: vec!["ADB".into()], brom_vid_pid: vec![], edl_vid_pid: vec![],
        notes: "Unisoc : pas de point d'accès public standard — ADB logique.".into(), recent: true,
    }); }
    if p.contains("ums9230") || p.contains("t610") { return Some(ChipsetProfile {
        soc: "Unisoc T610/T616".into(), vendor: "Unisoc".into(), platform: "UMS9230".into(),
        codename: p.clone(), cpu: "2x Cortex-A75 + 6x A55".into(), gpu: "Mali-G57 MP1".into(),
        process: "12 nm".into(), android_min: "9".into(),
        extraction_modes: vec!["ADB".into()], brom_vid_pid: vec![], edl_vid_pid: vec![],
        notes: "Unisoc : pas de point d'accès public standard — ADB logique.".into(), recent: true,
    }); }

    None
}

/// Détecte le SoC d'un appareil Android connecté via ADB (serial fourni) et renvoie
/// le profil chipsets complet + les valeurs brutes des props.
#[tauri::command]
pub fn detect_chipset(serial: String) -> Result<serde_json::Value, String> {
    let board = adb_getprop(&serial, "ro.board.platform");
    let soc   = adb_getprop(&serial, "ro.soc.model");
    let chip  = adb_getprop(&serial, "ro.chipname");
    let hw    = adb_getprop(&serial, "ro.hardware");
    let pboard= adb_getprop(&serial, "ro.product.board");
    let brand = adb_getprop(&serial, "ro.product.brand");
    let model = adb_getprop(&serial, "ro.product.model");
    let abi   = adb_getprop(&serial, "ro.product.cpu.abi");

    // Clés candidates, du plus précis au moins précis.
    let candidates = [&soc, &chip, &board, &hw, &pboard];
    let mut profile: Option<ChipsetProfile> = None;
    for c in candidates.iter().filter(|c| !c.is_empty()) {
        if let Some(p) = match_soc(c) { profile = Some(p); break; }
    }

    // Sinon plateforme générique par props connues.
    let profile = profile.or_else(|| {
        let joined = [board.clone(), hw.clone(), soc.clone(), chip.clone(), pboard.clone()].join(" ").to_lowercase();
        if joined.contains("mt") || joined.contains("mediatek") { Some(generic_mtk()) }
        else if joined.contains("sm") || joined.contains("qcom") || joined.contains("msm") { Some(generic_qcom()) }
        else { None }
    });

    Ok(serde_json::json!({
        "serial": serial,
        "brand": brand, "model": model, "abi": abi,
        "props": { "ro.board.platform": board, "ro.soc.model": soc, "ro.chipname": chip,
                   "ro.hardware": hw, "ro.product.board": pboard },
        "profile": profile,
        "detected": profile.is_some(),
    }))
}

/// Catalogue complet des chipsets récents connus (utile pour la matrice d'extraction).
#[tauri::command]
pub fn list_recent_chipsets() -> Vec<ChipsetProfile> {
    let mut v = Vec::new();
    for p in [
        // MediaTek
        "mt6989", "mt6985", "mt6983", "mt6897", "mt6895", "mt6878", "mt6835", "mt6789", "mt6768",
        // Qualcomm
        "sm8750", "sm8650", "sm8550", "sm8475", "sm8350", "sm8250", "sm7450", "sm7325", "sm6375",
        "sm6225", "sm4350",
        // Google
        "gordon_peak", "gs203", "gs202", "gs201",
        // Samsung
        "s5e9945", "s5e9935", "s5e8845", "s5e8835",
        // Unisoc
        "ums9230",
    ] {
        if let Some(prof) = match_soc(p) { v.push(prof); }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecte_mediatek_dimensity() {
        let p = match_soc("mt6895").expect("SoC MTK reconnu");
        assert_eq!(p.vendor, "MediaTek");
        assert!(p.soc.to_lowercase().contains("dimensity"));
        assert!(p.brom_vid_pid.iter().any(|x| x == "0e8d:0003"));
    }

    #[test]
    fn detecte_qualcomm_snapdragon() {
        let p = match_soc("SM8550").expect("SoC QC reconnu"); // insensible à la casse
        assert_eq!(p.vendor, "Qualcomm");
        assert!(p.soc.to_lowercase().contains("snapdragon"));
        assert!(p.edl_vid_pid.iter().any(|x| x == "05c6:9008"));
    }

    #[test]
    fn detecte_tensor_google() {
        let p = match_soc("gs201").expect("Tensor reconnu");
        assert_eq!(p.vendor, "Google");
        assert!(p.soc.to_lowercase().contains("tensor"));
        assert!(p.edl_vid_pid.is_empty(), "Tensor : pas d'EDL public");
    }

    #[test]
    fn detecte_par_vidpid_mode_special() {
        // MTK BROM
        let mtk = detect_by_vidpid("0e8d", "0003").expect("BROM MTK reconnu");
        assert_eq!(mtk.vendor, "MediaTek");
        // Qualcomm EDL
        let qc = detect_by_vidpid("05c6", "9008").expect("EDL QC reconnu");
        assert_eq!(qc.vendor, "Qualcomm");
        // Inconnu
        assert!(detect_by_vidpid("ffff", "ffff").is_none());
    }
}
