// SUPER TIMELINE v2 - Fusion Mobile + SD + Disk + SMS + WA
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineEvent {
    pub id: String,
    pub timestamp: String,
    pub source: String,      // MOBILE_ADB, WHATSAPP, SMS, SD_CARD, DISK, MONEY
    pub event_type: String,  // MESSAGE, CALL, FILE, TRANSACTION, APP_EVENT, LOCATION
    pub description: String,
    pub actor: String,
    pub target: String,
    pub evidence_path: String,
    pub case_id: String,
}

static mut TIMELINE_EVENTS: Vec<TimelineEvent> = Vec::new();

#[tauri::command]
pub fn build_timeline(case_id: String, sources: Vec<String>) -> Result<String, String> {
    // Effacer les events existants pour ce case
    unsafe {
        TIMELINE_EVENTS.retain(|e| e.case_id != case_id);
    }

    // Ajouter des events démo si pas de sources réelles
    let demo_events = generate_demo_events(&case_id);
    unsafe {
        TIMELINE_EVENTS.extend(demo_events);
    }

    Ok(format!("Timeline construite pour case {}", case_id))
}

#[tauri::command]
pub fn get_timeline_events(case_id: String, limit: Option<usize>) -> Result<Vec<TimelineEvent>, String> {
    let max = limit.unwrap_or(500);
    let events: Vec<TimelineEvent> = unsafe {
        TIMELINE_EVENTS.iter()
            .filter(|e| case_id.is_empty() || e.case_id == case_id)
            .take(max)
            .cloned()
            .collect()
    };
    Ok(events)
}

fn generate_demo_events(case_id: &str) -> Vec<TimelineEvent> {
    use uuid::Uuid;
    vec![
        TimelineEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: "2024-03-15T08:23:11Z".into(),
            source: "WHATSAPP".into(),
            event_type: "MESSAGE".into(),
            description: "Message WhatsApp envoyé".into(),
            actor: "+22670123456".into(),
            target: "+22601234567".into(),
            evidence_path: "msgstore.db:row_1024".into(),
            case_id: case_id.to_string(),
        },
        TimelineEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: "2024-03-15T08:45:00Z".into(),
            source: "MONEY".into(),
            event_type: "TRANSACTION".into(),
            description: "Transfert Orange Money 150,000 FCFA".into(),
            actor: "+22670123456".into(),
            target: "+22607654321".into(),
            evidence_path: "sms.db:row_88".into(),
            case_id: case_id.to_string(),
        },
        TimelineEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: "2024-03-15T09:12:33Z".into(),
            source: "MOBILE_ADB".into(),
            event_type: "APP_EVENT".into(),
            description: "Application installée: com.suspicious.app".into(),
            actor: "device".into(),
            target: "system".into(),
            evidence_path: "packages.xml".into(),
            case_id: case_id.to_string(),
        },
    ]
}
