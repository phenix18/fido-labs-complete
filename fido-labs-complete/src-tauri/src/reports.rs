// REPORTS MODULE - Rapports généraux et spécifiques par utilisateur
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub enum ReportType {
    General,
    SpecificUser,
    DeviceReport,
    TimelineReport,
    CustodyReport,
    MoneyFlowReport,
    EvidenceCatalog,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ReportFormat {
    PDF, DOCX, TXT, HTML, JSON,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Report {
    pub id: String,
    pub name: String,
    pub report_type: String,
    pub format: String,
    pub case_id: String,
    pub author: String,
    pub created_at: String,
    pub file_path: String,
    pub sections: Vec<String>,
    pub filters: ReportFilters,
    pub page_count: u32,
    pub hash_sha256: String,
    pub signed: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReportFilters {
    pub date_from: String,
    pub date_to: String,
    pub operators: Vec<String>,
    pub devices: Vec<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub min_severity: String,
    pub include_deleted: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub report_type: String,
    pub description: String,
    pub sections: Vec<String>,
}

static REPORTS: Mutex<Vec<Report>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn generate_report(
    name: String, report_type: String, format: String, case_id: String, author: String,
    filters_json: String,
) -> Result<Report, String> {
    let sections = match report_type.as_str() {
        "SpecificUser"    => vec!["Identité suspect".into(), "Activité temporelle".into(), "Transactions financières".into(), "Communications".into(), "Localisation".into(), "Conclusion".into()],
        "DeviceReport"    => vec!["Informations techniques".into(), "Données extraites".into(), "Applications installées".into(), "Historique des actions".into(), "Hash de vérification".into()],
        "TimelineReport"  => vec!["Chronologie complète".into(), "Événements par source".into(), "Corrélations".into(), "Anomalies".into()],
        "CustodyReport"   => vec!["Identité preuve".into(), "Chaîne de garde complète".into(), "Transferts".into(), "Conditions stockage".into(), "Destitution".into()],
        "MoneyFlowReport" => vec!["Résumé financier".into(), "Graphe flux".into(), "Comptes impliqués".into(), "Transactions détaillées".into(), "Anomalies".into()],
        "EvidenceCatalog" => vec!["Catalogue preuves".into(), "Types".into(), "Sources".into(), "Hashs".into(), "Liens".into()],
        _                 => vec!["Résumé".into(), "Méthodologie".into(), "Preuves collectées".into(), "Analyse timeline".into(), "Résultats".into(), "Annexes".into()],
    };
    let ext = match format.as_str() { "DOCX" => "docx", "TXT" => "txt", "HTML" => "html", "JSON" => "json", _ => "pdf" };
    let filters: ReportFilters = serde_json::from_str(&filters_json).unwrap_or_default();
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();
    let file_path = format!("reports/{}_{}.{}", case_id, id, ext);
    let report = Report {
        id, name, report_type, format, case_id, author,
        created_at: now.to_rfc3339(), file_path, sections, filters,
        page_count: 0,
        hash_sha256: format!("{:x}", now.timestamp_nanos_opt().unwrap_or(0)),
        signed: false,
    };
    REPORTS.lock().unwrap().push(report.clone());
    Ok(report)
}

#[tauri::command]
pub fn list_reports(case_id: String) -> Result<Vec<Report>, String> {
    let rpts = REPORTS.lock().unwrap();
    Ok(rpts.iter().filter(|r| case_id.is_empty() || r.case_id == case_id).cloned().collect())
}

#[tauri::command]
pub fn get_report(report_id: String) -> Result<Report, String> {
    REPORTS.lock().unwrap()
        .iter().find(|r| r.id == report_id)
        .cloned().ok_or_else(|| "Rapport non trouvé".to_string())
}

#[tauri::command]
pub fn sign_report(report_id: String, _operator: String) -> Result<Report, String> {
    let mut rpts = REPORTS.lock().unwrap();
    let r = rpts.iter_mut().find(|x| x.id == report_id)
        .ok_or_else(|| "Non trouvé".to_string())?;
    r.signed = true;
    r.hash_sha256 = format!("{:x}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    Ok(r.clone())
}

#[tauri::command]
pub fn generate_specific_user_report(
    user_identifier: String, case_id: String, author: String,
) -> Result<Report, String> {
    let filters = serde_json::json!({
        "keywords": [user_identifier.clone()],
        "categories": ["PHONE","SMS","MONEY","APP"],
        "min_severity": "Normal"
    }).to_string();
    generate_report(
        format!("Rapport spécifique - {}", user_identifier),
        "SpecificUser".into(), "PDF".into(), case_id, author, filters
    )
}

#[tauri::command]
pub fn generate_general_report(case_id: String, author: String) -> Result<Report, String> {
    generate_report(
        format!("Rapport Général - {}", case_id),
        "General".into(), "PDF".into(), case_id, author, "{}".into()
    )
}

#[tauri::command]
pub fn get_available_templates() -> Result<Vec<ReportTemplate>, String> {
    Ok(vec![
        ReportTemplate { id:"tpl-gen".into(), name:"Rapport Général d'Enquête".into(), report_type:"General".into(), description:"Synthèse complète de l'enquête".into(), sections:vec!["Résumé".into(),"Méthodologie".into(),"Preuves".into(),"Analyse".into(),"Résultats".into(),"Annexes".into()] },
        ReportTemplate { id:"tpl-user".into(), name:"Rapport Suspect Individuel".into(), report_type:"SpecificUser".into(), description:"Focus sur un individu spécifique".into(), sections:vec!["Identité".into(),"Activité".into(),"Finance".into(),"Comms".into(),"Localisation".into(),"Conclusion".into()] },
        ReportTemplate { id:"tpl-device".into(), name:"Rapport Appareil".into(), report_type:"DeviceReport".into(), description:"Détails techniques et données".into(), sections:vec!["Info techniques".into(),"Données".into(),"Apps".into(),"Actions".into(),"Hashes".into()] },
        ReportTemplate { id:"tpl-timeline".into(), name:"Rapport Timeline".into(), report_type:"TimelineReport".into(), description:"Chronologie détaillée".into(), sections:vec!["Chronologie".into(),"Événements".into(),"Corrélations".into(),"Anomalies".into()] },
        ReportTemplate { id:"tpl-custody".into(), name:"Rapport Chain of Custody".into(), report_type:"CustodyReport".into(), description:"Traçabilité des preuves".into(), sections:vec!["Identité".into(),"Chaîne".into(),"Transferts".into(),"Stockage".into(),"Destitution".into()] },
        ReportTemplate { id:"tpl-money".into(), name:"Rapport Flux Financiers".into(), report_type:"MoneyFlowReport".into(), description:"Analyse financière détaillée".into(), sections:vec!["Résumé".into(),"Graphe".into(),"Comptes".into(),"Transactions".into(),"Anomalies".into()] },
    ])
}
