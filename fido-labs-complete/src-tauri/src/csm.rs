// CSM — Chain of Custody Management
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;
use once_cell::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub struct CustodyStep {
    pub id: String,
    pub step_number: u32,
    pub action: String,
    pub operator: String,
    pub organization: String,
    pub location: String,
    pub timestamp: String,
    pub signature: String,
    pub notes: String,
    pub evidence_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CustodyChain {
    pub id: String,
    pub case_id: String,
    pub evidence_id: String,
    pub evidence_type: String,
    pub description: String,
    pub hash_sha256: String,
    pub size_bytes: u64,
    pub status: String,
    pub steps: Vec<CustodyStep>,
    pub created_at: String,
    pub current_holder: String,
}

static CSM_DB: Mutex<Vec<CustodyChain>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn create_custody(
    case_id: String, evidence_id: String, evidence_type: String,
    description: String, hash_sha256: String, size_bytes: u64, operator: String,
) -> Result<CustodyChain, String> {
    let chain = CustodyChain {
        id: Uuid::new_v4().to_string(),
        case_id,
        evidence_id: evidence_id.clone(),
        evidence_type, description,
        hash_sha256, size_bytes,
        status: "IN_CUSTODY".into(),
        created_at: Utc::now().to_rfc3339(),
        current_holder: operator.clone(),
        steps: vec![CustodyStep {
            id: Uuid::new_v4().to_string(),
            step_number: 1,
            action: "Collection".into(),
            operator,
            organization: "FIDO-LABS".into(),
            location: "Local".into(),
            timestamp: Utc::now().to_rfc3339(),
            signature: format!("sig-{}", Uuid::new_v4()),
            notes: "Preuve collectée".into(),
            evidence_id,
        }],
    };
    CSM_DB.lock().unwrap().push(chain.clone());
    Ok(chain)
}

#[tauri::command]
pub fn add_custody_step(chain_id: String, step: CustodyStep) -> Result<CustodyChain, String> {
    let mut db = CSM_DB.lock().unwrap();
    let c = db.iter_mut().find(|x| x.id == chain_id)
        .ok_or_else(|| "Chain non trouvée".to_string())?;
    let next_num = c.steps.len() as u32 + 1;
    let mut new_step = step;
    new_step.id = Uuid::new_v4().to_string();
    new_step.step_number = next_num;
    new_step.timestamp = Utc::now().to_rfc3339();
    new_step.signature = format!("sig-{}", Uuid::new_v4());
    c.steps.push(new_step);
    Ok(c.clone())
}

#[tauri::command]
pub fn get_custody_chains(case_id: String) -> Result<Vec<CustodyChain>, String> {
    let db = CSM_DB.lock().unwrap();
    Ok(db.iter().filter(|c| case_id.is_empty() || c.case_id == case_id).cloned().collect())
}

#[tauri::command]
pub fn get_custody(chain_id: String) -> Result<CustodyChain, String> {
    CSM_DB.lock().unwrap()
        .iter().find(|c| c.id == chain_id)
        .cloned().ok_or_else(|| "Non trouvée".to_string())
}

#[tauri::command]
pub fn transfer_custody(chain_id: String, new_holder: String, org: String, location: String) -> Result<CustodyChain, String> {
    let mut db = CSM_DB.lock().unwrap();
    let c = db.iter_mut().find(|x| x.id == chain_id)
        .ok_or_else(|| "Chain non trouvée".to_string())?;
    let prev_holder = c.current_holder.clone();
    let ev_id = c.evidence_id.clone();
    c.steps.push(CustodyStep {
        id: Uuid::new_v4().to_string(),
        step_number: c.steps.len() as u32 + 1,
        action: "Transfer".into(),
        operator: prev_holder,
        organization: org,
        location,
        timestamp: Utc::now().to_rfc3339(),
        signature: format!("sig-{}", Uuid::new_v4()),
        notes: format!("Transfert à {}", new_holder),
        evidence_id: ev_id,
    });
    c.current_holder = new_holder;
    c.status = "IN_CUSTODY".into();
    Ok(c.clone())
}
