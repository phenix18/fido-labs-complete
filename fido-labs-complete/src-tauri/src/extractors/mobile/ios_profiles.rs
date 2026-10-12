// IOS PROFILES MODULE v1 - Base de profils iPhone/iPad récents + capacités d'extraction.
// Usage légal uniquement avec autorisation judiciaire.
//
// Pour chaque appareil iOS on documente :
//   - identifiants de modèle (product_type, ex. "iPhone14,5") et noms commerciaux ;
//   - chipset (A-series) + année ;
//   - capacités d'extraction RÉELLES, en particulier la compatibilité checkm8/checkm30
//     (exploit DFU matériel) qui permet l'acquisition de la mémoire sur appareils
//     verrouillés / non débogables — en respectant le cadre légal (aucun bypass du
//     verrou d'usage, acquisition du système de fichiers chiffré sous le Secure Enclave
//     qui reste intact).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IosProfile {
    pub product_types: Vec<String>, // ex. ["iPhone14,5"]
    pub name: String,               // nom commercial (ex. "iPhone 13")
    pub generation: String,         // ex. "iPhone 13"
    pub chipset: String,            // ex. "A15 Bionic"
    pub chipset_arch: String,       // ex. "A15"
    pub year: u32,
    pub checkm8: bool,              // compatible checkm8 (A5–A11) — acquisition DFU matérielle
    pub checkm30: bool,             // compatible checkm30 / blackbird (A12+) — limité
    pub extraction_modes: Vec<String>, // iTunes/Finder backup, DFU checkm8, ALEAPP, etc.
    pub notes: String,
}

/// Recherche un profil iOS par identifiant de modèle (product_type).
/// Renvoie aussi une fiche "générique" pour les modèles inconnus récents.
pub fn lookup(product_type: &str) -> Option<IosProfile> {
    let pt = product_type.trim();
    let all = all_profiles();
    all.into_iter().find(|p| p.product_types.iter().any(|x| x.eq_ignore_ascii_case(pt)))
}

/// Profil générique pour un iOS moderne non listé (A12+ : checkm8 non dispo).
pub fn generic_modern() -> IosProfile {
    IosProfile {
        product_types: vec![],
        name: "iPhone/iPad moderne (A12+)".into(),
        generation: "Génération récente".into(),
        chipset: "Apple A-series (A12+)".into(),
        chipset_arch: "A12+".into(),
        year: 2018,
        checkm8: false,
        checkm30: false,
        extraction_modes: vec![
            "Sauvegarde iTunes/Finder (légal, mot de passe si chiffrée)".into(),
            "iCloud (réquisition fournisseur)".into(),
        ],
        notes: "A12+ : pas d'exploit DFU matériel public fiable (checkm8 limité). L'acquisition passe par une sauvegarde autorisée, l'iCloud (sur réquisition) ou un outil certifié avec mandat.".into(),
    }
}

/// Catalogue complet des profils iOS (de l'iPhone 5s à l'iPhone 16 / iPad récents).
pub fn all_profiles() -> Vec<IosProfile> {
    let mut v = Vec::new();
    let mut push = |pt: &[&str], name: &str, gen: &str, chip: &str, arch: &str, year: u32,
                    c8: bool, c30: bool, modes: &[&str], notes: &str| {
        v.push(IosProfile {
            product_types: pt.iter().map(|s| s.to_string()).collect(),
            name: name.into(), generation: gen.into(),
            chipset: chip.into(), chipset_arch: arch.into(), year,
            checkm8: c8, checkm30: c30,
            extraction_modes: modes.iter().map(|s| s.to_string()).collect(),
            notes: notes.into(),
        });
    };

    // ── Compatibles checkm8 (A5–A11) : acquisition DFU matérielle possible ──
    push(&["iPhone6,1","iPhone6,2"], "iPhone 5s", "iPhone 5s", "A7", "A7", 2013, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"],
        "A7 : checkm8 exploitable en DFU — acquisition du filesystem sous Secure Enclave (clés de protection dérivées à la volée, enclave intacte).");
    push(&["iPhone7,2"], "iPhone 6", "iPhone 6", "A8", "A8", 2014, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A8 : checkm8 OK.");
    push(&["iPhone7,1"], "iPhone 6 Plus", "iPhone 6", "A8", "A8", 2014, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A8 : checkm8 OK.");
    push(&["iPhone8,1"], "iPhone 6s", "iPhone 6s", "A9", "A9", 2015, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A9 : checkm8 OK.");
    push(&["iPhone8,2"], "iPhone 6s Plus", "iPhone 6s", "A9", "A9", 2015, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A9 : checkm8 OK.");
    push(&["iPhone8,4"], "iPhone SE (1re)", "iPhone SE", "A9", "A9", 2016, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A9 : checkm8 OK.");
    push(&["iPhone9,1","iPhone9,3"], "iPhone 7", "iPhone 7", "A10 Fusion", "A10", 2016, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A10 : checkm8 OK.");
    push(&["iPhone9,2","iPhone9,4"], "iPhone 7 Plus", "iPhone 7", "A10 Fusion", "A10", 2016, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A10 : checkm8 OK.");
    push(&["iPhone10,1","iPhone10,4"], "iPhone 8", "iPhone 8", "A11 Bionic", "A11", 2017, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A11 : dernier checkm8 fiable.");
    push(&["iPhone10,2","iPhone10,5"], "iPhone 8 Plus", "iPhone 8", "A11 Bionic", "A11", 2017, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A11 : checkm8 OK.");
    push(&["iPhone10,3","iPhone10,6"], "iPhone X", "iPhone X", "A11 Bionic", "A11", 2017, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A11 : checkm8 OK (Face ID intact).");

    // ── A12+ : checkm8 non dispo — extraction via sauvegarde / iCloud ──
    push(&["iPhone11,8"], "iPhone XR", "iPhone XR", "A12 Bionic", "A12", 2018, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A12 : pas de checkm8 fiable.");
    push(&["iPhone11,2"], "iPhone XS", "iPhone XS", "A12 Bionic", "A12", 2018, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A12 : pas de checkm8 fiable.");
    push(&["iPhone11,4","iPhone11,6"], "iPhone XS Max", "iPhone XS", "A12 Bionic", "A12", 2018, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A12 : pas de checkm8 fiable.");
    push(&["iPhone12,1"], "iPhone 11", "iPhone 11", "A13 Bionic", "A13", 2019, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A13 : pas de checkm8 fiable.");
    push(&["iPhone12,3"], "iPhone 11 Pro", "iPhone 11", "A13 Bionic", "A13", 2019, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A13 : pas de checkm8 fiable.");
    push(&["iPhone12,5"], "iPhone 11 Pro Max", "iPhone 11", "A13 Bionic", "A13", 2019, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A13 : pas de checkm8 fiable.");
    push(&["iPhone12,8"], "iPhone SE (2e)", "iPhone SE", "A13 Bionic", "A13", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A13 : pas de checkm8 fiable.");
    push(&["iPhone13,1"], "iPhone 12 mini", "iPhone 12", "A14 Bionic", "A14", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A14 : pas de checkm8 fiable.");
    push(&["iPhone13,2"], "iPhone 12", "iPhone 12", "A14 Bionic", "A14", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A14 : pas de checkm8 fiable.");
    push(&["iPhone13,3"], "iPhone 12 Pro", "iPhone 12", "A14 Bionic", "A14", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A14 : pas de checkm8 fiable.");
    push(&["iPhone13,4"], "iPhone 12 Pro Max", "iPhone 12", "A14 Bionic", "A14", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A14 : pas de checkm8 fiable.");
    push(&["iPhone14,4"], "iPhone 13 mini", "iPhone 13", "A15 Bionic", "A15", 2021, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone14,5"], "iPhone 13", "iPhone 13", "A15 Bionic", "A15", 2021, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone14,2"], "iPhone 13 Pro", "iPhone 13", "A15 Bionic", "A15", 2021, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone14,3"], "iPhone 13 Pro Max", "iPhone 13", "A15 Bionic", "A15", 2021, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone14,7"], "iPhone SE (3e)", "iPhone SE", "A15 Bionic", "A15", 2022, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone14,8"], "iPhone 14", "iPhone 14", "A15 Bionic", "A15", 2022, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A15 : pas de checkm8 fiable.");
    push(&["iPhone15,2"], "iPhone 14 Pro", "iPhone 14", "A16 Bionic", "A16", 2022, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A16 : pas de checkm8 fiable.");
    push(&["iPhone15,3"], "iPhone 14 Pro Max", "iPhone 14", "A16 Bionic", "A16", 2022, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A16 : pas de checkm8 fiable.");
    push(&["iPhone15,4"], "iPhone 15", "iPhone 15", "A16 Bionic", "A16", 2023, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A16 : pas de checkm8 fiable.");
    push(&["iPhone15,5"], "iPhone 15 Plus", "iPhone 15", "A16 Bionic", "A16", 2023, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A16 : pas de checkm8 fiable.");
    push(&["iPhone16,1"], "iPhone 15 Pro", "iPhone 15", "A17 Pro", "A17", 2023, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A17 : pas de checkm8 fiable.");
    push(&["iPhone16,2"], "iPhone 15 Pro Max", "iPhone 15", "A17 Pro", "A17", 2023, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A17 : pas de checkm8 fiable.");
    push(&["iPhone17,3"], "iPhone 16", "iPhone 16", "A18", "A18", 2024, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A18 : pas de checkm8 fiable.");
    push(&["iPhone17,1"], "iPhone 16 Pro", "iPhone 16", "A18 Pro", "A18", 2024, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A18 : pas de checkm8 fiable.");

    // ── iPad (extrait représentatif) ──
    push(&["iPad5,1","iPad5,2"], "iPad mini 4", "iPad mini", "A8", "A8", 2015, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A8 : checkm8 OK.");
    push(&["iPad6,11","iPad6,12"], "iPad (5e)", "iPad", "A9", "A9", 2017, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A9 : checkm8 OK.");
    push(&["iPad7,5","iPad7,6"], "iPad (6e)", "iPad", "A10", "A10", 2018, true, false,
        &["Sauvegarde iTunes/Finder","DFU checkm8 (acquisition mémoire)"], "A10 : checkm8 OK.");
    push(&["iPad11,6","iPad11,7"], "iPad (8e)", "iPad", "A12", "A12", 2020, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A12 : pas de checkm8 fiable.");
    push(&["iPad13,16","iPad13,17"], "iPad (10e)", "iPad", "A14", "A14", 2022, false, false,
        &["Sauvegarde iTunes/Finder","iCloud (réquisition)"], "A14 : pas de checkm8 fiable.");

    v
}

/// Détecte le profil iOS d'après le product_type (ex. "iPhone14,5").
/// Renvoie le profil exact, sinon un profil générique moderne.
#[tauri::command]
pub fn detect_ios_profile(product_type: String) -> serde_json::Value {
    let prof = lookup(&product_type).unwrap_or_else(generic_modern);
    serde_json::json!({
        "product_type": product_type,
        "identified": !prof.product_types.is_empty(),
        "profile": prof,
    })
}

/// Catalogue des profils iOS compatibles checkm8 (acquisition DFU sur verrouillés).
#[tauri::command]
pub fn list_checkm8_devices() -> Vec<IosProfile> {
    all_profiles().into_iter().filter(|p| p.checkm8).collect()
}

/// Catalogue complet des profils iOS connus.
#[tauri::command]
pub fn list_ios_profiles() -> Vec<IosProfile> {
    all_profiles()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifie_iphone_13() {
        let p = lookup("iPhone14,5").expect("iPhone 13 reconnu");
        assert_eq!(p.name, "iPhone 13");
        assert_eq!(p.chipset, "A15 Bionic");
        assert!(!p.checkm8);
    }

    #[test]
    fn identifie_iphone_5s_checkm8() {
        let p = lookup("iPhone6,1").expect("iPhone 5s reconnu");
        assert!(p.checkm8, "A7 doit être checkm8");
        assert!(p.extraction_modes.iter().any(|m| m.contains("checkm8")));
    }

    #[test]
    fn identifie_ipad() {
        let p = lookup("iPad5,1").expect("iPad mini 4 reconnu");
        assert!(p.name.contains("mini"));
        assert!(p.checkm8);
    }

    #[test]
    fn inconnu_retourne_generique() {
        let p = lookup("iPhone99,99").unwrap_or_else(generic_modern);
        assert!(!p.checkm8, "modèle inconnu = générique moderne non checkm8");
    }

    #[test]
    fn case_insensible() {
        let p = lookup("iphone14,5").expect("insensible à la casse");
        assert_eq!(p.name, "iPhone 13");
    }
}
