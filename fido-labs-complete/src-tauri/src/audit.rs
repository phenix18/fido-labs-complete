// AUDIT LOG v2
use rusqlite::{Connection, Result as SqlResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub operator: String,
    pub action: String,
    pub target: String,
    pub hash: String,
    pub case_id: String,
}

pub static mut AUDIT_DB_PATH_PUB: String = String::new();
static mut AUDIT_DB_PATH: String = String::new();

pub fn init_audit_db(app_dir: &str) -> SqlResult<()> {
    let p = format!("{}/audit.db", app_dir);
    unsafe {
        AUDIT_DB_PATH = p.clone();
        AUDIT_DB_PATH_PUB = p.clone();
    }
    let conn = Connection::open(unsafe { &AUDIT_DB_PATH })?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            operator TEXT NOT NULL,
            action TEXT NOT NULL,
            target TEXT NOT NULL,
            hash TEXT NOT NULL,
            case_id TEXT NOT NULL
        );
    ")?;
    Ok(())
}

fn get_db() -> SqlResult<Connection> {
    Connection::open(unsafe { &AUDIT_DB_PATH })
}

#[tauri::command]
pub fn log_action(operator: String, action: String, target: String, case_id: String) -> Result<String, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();
    let hash = fnv_hash(&format!("{}{}{}{}{}", &timestamp, &operator, &action, &target, &case_id));
    conn.execute(
        "INSERT INTO audit_log (id, timestamp, operator, action, target, hash, case_id) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        (&id, &timestamp, &operator, &action, &target, &hash, &case_id),
    ).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn get_audit_log(case_id: String) -> Result<Vec<AuditEntry>, String> {
    let conn = get_db().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, operator, action, target, hash, case_id FROM audit_log WHERE case_id = ?1 OR ?1 = '' ORDER BY timestamp DESC LIMIT 500"
    ).map_err(|e| e.to_string())?;
    let entries: Vec<AuditEntry> = stmt.query_map([&case_id], |row| {
        Ok(AuditEntry {
            id: row.get(0)?, timestamp: row.get(1)?, operator: row.get(2)?,
            action: row.get(3)?, target: row.get(4)?, hash: row.get(5)?, case_id: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok()).collect();
    Ok(entries)
}

fn fnv_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{:016x}", h)
}
