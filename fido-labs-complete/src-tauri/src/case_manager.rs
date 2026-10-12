// CASE MANAGER - Gestion des dossiers d'enquête
#![allow(static_mut_refs)]
use rusqlite::{Connection, Result as SqlResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use hex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub reference: String,
    pub operator: String,
    pub created_at: String,
    pub status: String, // OUVERT, FERME, ARCHIVE
    pub notes: String,
}

static mut CASES_DB_PATH: String = String::new();

pub fn init_cases_db(app_dir: &str) -> SqlResult<()> {
    unsafe { CASES_DB_PATH = format!("{}/cases.db", app_dir); }
    let path = unsafe { CASES_DB_PATH.clone() };
    let conn = Connection::open(&path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS cases (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            reference TEXT NOT NULL,
            operator TEXT NOT NULL,
            created_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'OUVERT',
            notes TEXT NOT NULL DEFAULT ''
        );
    ")?;
    Ok(())
}

fn get_db() -> SqlResult<Connection> {
    let path = unsafe { CASES_DB_PATH.clone() };
    Connection::open(&path)
}

#[tauri::command]
pub fn create_case(name: String, reference: String, operator: String, notes: String) -> Result<Case, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    let case = Case {
        id: Uuid::new_v4().to_string(),
        name,
        reference,
        operator,
        created_at: Utc::now().to_rfc3339(),
        status: "OUVERT".into(),
        notes,
    };
    conn.execute(
        "INSERT INTO cases (id, name, reference, operator, created_at, status, notes) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        (&case.id, &case.name, &case.reference, &case.operator, &case.created_at, &case.status, &case.notes),
    ).map_err(|e| e.to_string())?;
    Ok(case)
}

#[tauri::command]
pub fn list_cases() -> Result<Vec<Case>, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, name, reference, operator, created_at, status, notes FROM cases ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;
    let cases: Vec<Case> = stmt.query_map([], |row| {
        Ok(Case {
            id: row.get(0)?,
            name: row.get(1)?,
            reference: row.get(2)?,
            operator: row.get(3)?,
            created_at: row.get(4)?,
            status: row.get(5)?,
            notes: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    Ok(cases)
}

#[tauri::command]
pub fn get_case(id: String) -> Result<Case, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, reference, operator, created_at, status, notes FROM cases WHERE id = ?1",
        [&id],
        |row| Ok(Case {
            id: row.get(0)?,
            name: row.get(1)?,
            reference: row.get(2)?,
            operator: row.get(3)?,
            created_at: row.get(4)?,
            status: row.get(5)?,
            notes: row.get(6)?,
        })
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_chain_of_custody(case_id: String, output_path: String) -> Result<String, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    let case = conn.query_row(
        "SELECT id, name, reference, operator, created_at, status, notes FROM cases WHERE id = ?1",
        [&case_id],
        |row| Ok(Case {
            id: row.get(0)?,
            name: row.get(1)?,
            reference: row.get(2)?,
            operator: row.get(3)?,
            created_at: row.get(4)?,
            status: row.get(5)?,
            notes: row.get(6)?,
        })
    ).map_err(|e| e.to_string())?;

    let audit_path = unsafe { crate::audit::AUDIT_DB_PATH_PUB.clone() };
    let audit_conn = Connection::open(&audit_path)
        .unwrap_or_else(|_| Connection::open(":memory:").unwrap());

    let mut audit_entries = Vec::new();
    if let Ok(mut stmt) = audit_conn.prepare(
        "SELECT timestamp, operator, action, target, hash FROM audit_log WHERE case_id = ?1 ORDER BY timestamp ASC"
    ) {
        let _ = stmt.query_map([&case_id], |row| {
            Ok(format!("  [{}] {} | {} | {} | hash:{}",
                row.get::<_,String>(0)?, row.get::<_,String>(1)?,
                row.get::<_,String>(2)?, row.get::<_,String>(3)?,
                row.get::<_,String>(4)?))
        }).map(|rows| {
            for r in rows.flatten() { audit_entries.push(r); }
        });
    }

    let report = format!(
        "=== FIDO-LABS - CHAIN OF CUSTODY REPORT ===\n\
        Généré: {}\n\
        \n\
        DOSSIER: {}\n\
        Référence judiciaire: {}\n\
        Opérateur: {}\n\
        Ouvert le: {}\n\
        Statut: {}\n\
        Notes: {}\n\
        \n\
        === JOURNAL D'AUDIT ({} entrées) ===\n\
        {}\n\
        \n\
        === FIN DU RAPPORT ===\n",
        Utc::now().to_rfc3339(),
        case.name, case.reference, case.operator,
        case.created_at, case.status, case.notes,
        audit_entries.len(),
        audit_entries.join("\n")
    );

    std::fs::write(&output_path, &report).map_err(|e| e.to_string())?;
    Ok(format!("Rapport exporté vers {}", output_path))
}

/// Sauvegarde des données extraites dans un fichier sécurisé avec chaîne de garde SHA256.
#[tauri::command]
pub fn save_extracted_data(data: String, output_path: String, case_id: String) -> Result<String, String> {
    let json = data;
    std::fs::create_dir_all(std::path::Path::new(&output_path).parent().unwrap())
        .map_err(|e| format!("Création dossier : {}", e))?;
    std::fs::write(&output_path, &json).map_err(|e| format!("Écriture : {}", e))?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&json);
    let sha = hex::encode(hasher.finalize());
    let manifest = serde_json::json!({
        "tool": "FIDO-LABS v3",
        "case_id": case_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "file": output_path,
        "sha256": sha,
        "size": json.len(),
    });
    let manifest_path = format!("{}.manifest.json", output_path);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default())
        .map_err(|e| format!("Manifeste : {}", e))?;
    Ok(format!("Sauvegarde créée : {} (SHA256: {})", output_path, sha))
}
