// EVENT CALENDAR MODULE - Calendrier des événements forensiques
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;
use once_cell::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start: String,
    pub end: String,
    pub category: String,   // Extraction|Analysis|Transfer|Hearing|Deadline|Meeting|Reminder|Custom
    pub status: String,     // Planned|InProgress|Completed|Cancelled|Overdue
    pub case_id: String,
    pub assigned_to: String,
    pub location: String,
    pub priority: String,   // Low|Normal|High|Critical
    pub reminders: Vec<Reminder>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub id: String,
    pub event_id: String,
    pub remind_at: String,
    pub sent: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeeklySummary {
    pub week_start: String,
    pub total: usize,
    pub by_category: serde_json::Value,
    pub overdue: usize,
    pub upcoming: usize,
}

static CAL_EVENTS: Mutex<Vec<CalendarEvent>> = Mutex::new(Vec::new());

#[tauri::command]
pub fn create_event(
    title: String, description: String, start: String, end: String,
    category: String, case_id: String, assigned_to: String, location: String,
    priority: String,
) -> Result<CalendarEvent, String> {
    let ev = CalendarEvent {
        id: Uuid::new_v4().to_string(),
        title, description, start, end, category,
        status: "Planned".into(),
        case_id, assigned_to, location, priority,
        reminders: Vec::new(),
    };
    CAL_EVENTS.lock().unwrap().push(ev.clone());
    Ok(ev)
}

#[tauri::command]
pub fn get_events(case_id: String, date_from: String, date_to: String) -> Result<Vec<CalendarEvent>, String> {
    let db = CAL_EVENTS.lock().unwrap();
    let events: Vec<CalendarEvent> = db.iter()
        .filter(|e| {
            let match_case = case_id.is_empty() || e.case_id == case_id;
            let match_date = if date_from.is_empty() {
                true
            } else {
                e.start >= date_from && (date_to.is_empty() || e.start <= date_to)
            };
            match_case && match_date
        })
        .cloned()
        .collect();
    Ok(events)
}

#[tauri::command]
pub fn get_events_by_day(date: String, case_id: String) -> Result<Vec<CalendarEvent>, String> {
    let date_end = format!("{}T23:59:59", date);
    get_events(case_id, date.clone(), date_end)
}

#[tauri::command]
pub fn update_event_status(event_id: String, status: String) -> Result<CalendarEvent, String> {
    let mut db = CAL_EVENTS.lock().unwrap();
    let ev = db.iter_mut().find(|e| e.id == event_id)
        .ok_or_else(|| "Event non trouvé".to_string())?;
    ev.status = status;
    Ok(ev.clone())
}

#[tauri::command]
pub fn add_reminder(event_id: String, remind_at: String) -> Result<Reminder, String> {
    let mut db = CAL_EVENTS.lock().unwrap();
    let ev = db.iter_mut().find(|e| e.id == event_id)
        .ok_or_else(|| "Event non trouvé".to_string())?;
    let r = Reminder {
        id: Uuid::new_v4().to_string(),
        event_id: event_id.clone(),
        remind_at,
        sent: false,
    };
    ev.reminders.push(r.clone());
    Ok(r)
}

#[tauri::command]
pub fn get_weekly_summary(date: String, case_id: String) -> Result<WeeklySummary, String> {
    // date_to = date + 7 jours (comparaison string ISO fonctionne sur YYYY-MM-DD)
    let events = get_events(case_id, date.clone(), format!("{}z", date))?;
    let by_cat = serde_json::json!({
        "extraction": events.iter().filter(|e| e.category == "Extraction").count(),
        "analysis":   events.iter().filter(|e| e.category == "Analysis").count(),
        "transfer":   events.iter().filter(|e| e.category == "Transfer").count(),
        "hearing":    events.iter().filter(|e| e.category == "Hearing").count(),
        "deadline":   events.iter().filter(|e| e.category == "Deadline").count(),
        "meeting":    events.iter().filter(|e| e.category == "Meeting").count(),
    });
    let overdue  = events.iter().filter(|e| e.status == "Overdue").count();
    let upcoming = events.iter().filter(|e| e.status == "Planned" || e.status == "InProgress").count();
    Ok(WeeklySummary {
        week_start: date,
        total: events.len(),
        by_category: by_cat,
        overdue,
        upcoming,
    })
}
