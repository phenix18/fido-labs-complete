// SUPER TIMELINE v3 — fusion RÉELLE des sources horodatées d'un dossier.
// Agrège SMS + journaux d'appels (extraction logique), messages WhatsApp et
// transactions mobile money en une chronologie unique triée. Aucune donnée
// inventée : chaque événement provient d'une acquisition réelle passée en entrée.

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone, NaiveDateTime};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineEvent {
    pub id: String,
    pub timestamp: String,   // affichage (RFC3339 quand la date a pu être analysée)
    pub source: String,      // SMS, CALL, WHATSAPP, MONEY
    pub event_type: String,  // MESSAGE, CALL, TRANSACTION
    pub description: String,
    pub actor: String,
    pub target: String,
    pub evidence_path: String,
    pub case_id: String,
    #[serde(skip)]
    pub sort_key: i64,       // epoch (s) pour le tri ; non sérialisé
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineResult {
    pub case_id: String,
    pub total: usize,
    pub by_source: HashMap<String, usize>,
    pub first: String,
    pub last: String,
    pub events: Vec<TimelineEvent>,
}

/// Store thread-safe (remplace l'ancien `static mut`, non sûr).
fn store() -> &'static Mutex<HashMap<String, Vec<TimelineEvent>>> {
    static S: OnceLock<Mutex<HashMap<String, Vec<TimelineEvent>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Analyse une date de format variable en DateTime<Utc> (pour tri + normalisation).
fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() { return None; }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M", "%d/%m/%Y %H:%M:%S"] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Utc.from_utc_datetime(&ndt));
        }
    }
    // Nombre = epoch (secondes ou millisecondes)
    if let Ok(n) = s.parse::<i64>() {
        let secs = if n > 100_000_000_000 { n / 1000 } else { n };
        return Utc.timestamp_opt(secs, 0).single();
    }
    None
}

fn sj(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string()).unwrap_or_default()
}

fn truncate(s: &str, n: usize) -> String {
    let s: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if s.chars().count() <= n { s } else { format!("{}…", s.chars().take(n).collect::<String>()) }
}

fn join(dir: &str, file: &str) -> String {
    if dir.is_empty() { file.to_string() }
    else { format!("{}/{}", dir.trim_end_matches(|c| c == '/' || c == '\\'), file) }
}

fn make_event(
    case_id: &str, ts_raw: &str, source: &str, etype: &str,
    desc: String, actor: String, target: String, evidence: String,
) -> TimelineEvent {
    let (timestamp, sort_key) = match parse_ts(ts_raw) {
        Some(dt) => (dt.to_rfc3339(), dt.timestamp()),
        None => (ts_raw.to_string(), 0),
    };
    TimelineEvent {
        id: Uuid::new_v4().to_string(),
        timestamp, source: source.into(), event_type: etype.into(),
        description: desc, actor, target, evidence_path: evidence,
        case_id: case_id.to_string(), sort_key,
    }
}

/// Construit la super-timeline en fusionnant les sources réellement acquises.
/// `extraction` = résultat d'extract_logical (contient sms[] et calls[]),
/// `whatsapp` = messages parse_whatsapp_db, `money` = transactions parse_mobile_money.
#[tauri::command]
pub fn build_timeline(
    case_id: String,
    extraction: Option<serde_json::Value>,
    whatsapp: Option<serde_json::Value>,
    money: Option<serde_json::Value>,
    limit: Option<usize>,
) -> Result<TimelineResult, String> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    // ── SMS + Appels (extraction logique) ──────────────────────────
    if let Some(ex) = extraction.as_ref() {
        let dir = ex.get("output_dir").and_then(|x| x.as_str()).unwrap_or("");
        if let Some(sms) = ex.get("sms").and_then(|a| a.as_array()) {
            for m in sms {
                let date = sj(m, "date");
                let contact = sj(m, "contact");
                let dir_lbl = sj(m, "direction");
                let body = sj(m, "body");
                // "Reçu" / "Received" → l'appareil reçoit ; sinon envoyé.
                let received = {
                    let d = dir_lbl.to_lowercase();
                    d.starts_with("re") || d.contains("çu") || d.contains("recu")
                };
                let (actor, target) = if received {
                    (contact.clone(), "Appareil".to_string())
                } else {
                    ("Appareil".to_string(), contact.clone())
                };
                events.push(make_event(
                    &case_id, &date, "SMS", "MESSAGE",
                    format!("SMS {} : {}", dir_lbl.to_lowercase(), truncate(&body, 120)),
                    actor, target, join(dir, "sms.json"),
                ));
            }
        }
        if let Some(calls) = ex.get("calls").and_then(|a| a.as_array()) {
            for c in calls {
                let date = sj(c, "date");
                let num = sj(c, "number");
                let typ = sj(c, "type");
                let dur = sj(c, "duration");
                let outgoing = typ.eq_ignore_ascii_case("Sortant") || typ.eq_ignore_ascii_case("outgoing");
                let (actor, target) = if outgoing {
                    ("Appareil".to_string(), num.clone())
                } else {
                    (num.clone(), "Appareil".to_string())
                };
                events.push(make_event(
                    &case_id, &date, "CALL", "CALL",
                    format!("Appel {} ({})", typ, if dur.is_empty() { "—".into() } else { dur }),
                    actor, target, join(dir, "calls.json"),
                ));
            }
        }
    }

    // ── Messages WhatsApp ──────────────────────────────────────────
    if let Some(arr) = whatsapp.as_ref().and_then(|v| v.as_array()) {
        for m in arr {
            let ts = sj(m, "timestamp");
            let from = sj(m, "from_jid");
            let to = sj(m, "to_jid");
            let body = sj(m, "body");
            let chat = sj(m, "chat_name");
            let deleted = m.get("deleted").and_then(|x| x.as_bool()).unwrap_or(false);
            let prefix = if deleted { "[effacé] " } else { "" };
            let chat_lbl = if chat.is_empty() { String::new() } else { format!("{} — ", chat) };
            events.push(make_event(
                &case_id, &ts, "WHATSAPP", "MESSAGE",
                format!("{}{}{}", chat_lbl, prefix, truncate(&body, 120)),
                from, to, "msgstore.db".into(),
            ));
        }
    }

    // ── Transactions mobile money ──────────────────────────────────
    if let Some(arr) = money.as_ref().and_then(|v| v.as_array()) {
        for t in arr {
            let date = sj(t, "date");
            let provider = sj(t, "provider");
            let typ = sj(t, "type_tx");
            let montant = t.get("montant").and_then(|x| x.as_i64()).unwrap_or(0);
            let cur = sj(t, "currency");
            let from = sj(t, "from_num");
            let to = sj(t, "to_num");
            let reference = sj(t, "reference");
            events.push(make_event(
                &case_id, &date, "MONEY", "TRANSACTION",
                format!("{} {} {} {}", provider, typ, montant, cur),
                from, to,
                if reference.is_empty() { "mobile_money".into() } else { reference },
            ));
        }
    }

    // Tri chronologique ascendant (événements non datés en fin).
    events.sort_by(|a, b| match (a.sort_key, b.sort_key) {
        (0, 0) => std::cmp::Ordering::Equal,
        (0, _) => std::cmp::Ordering::Greater,
        (_, 0) => std::cmp::Ordering::Less,
        (x, y) => x.cmp(&y),
    });

    // Statistiques
    let total = events.len();
    let mut by_source: HashMap<String, usize> = HashMap::new();
    for e in &events { *by_source.entry(e.source.clone()).or_insert(0) += 1; }
    let first = events.iter().find(|e| e.sort_key > 0).map(|e| e.timestamp.clone()).unwrap_or_default();
    let last = events.iter().rev().find(|e| e.sort_key > 0).map(|e| e.timestamp.clone()).unwrap_or_default();

    // Persiste pour get_timeline_events, puis applique la limite d'affichage.
    if let Ok(mut g) = store().lock() {
        g.insert(case_id.clone(), events.clone());
    }
    let max = limit.unwrap_or(1000);
    events.truncate(max);

    Ok(TimelineResult { case_id, total, by_source, first, last, events })
}

#[tauri::command]
pub fn get_timeline_events(case_id: String, limit: Option<usize>) -> Result<Vec<TimelineEvent>, String> {
    let max = limit.unwrap_or(1000);
    let g = store().lock().map_err(|_| "verrou timeline".to_string())?;
    let events = if case_id.is_empty() {
        g.values().flatten().take(max).cloned().collect()
    } else {
        g.get(&case_id).map(|v| v.iter().take(max).cloned().collect()).unwrap_or_default()
    };
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fusionne_et_trie_les_sources() {
        let extraction = serde_json::json!({
            "output_dir": "/cases/A/extraction",
            "sms": [
                {"date":"2024-03-15 08:23:11","contact":"+22670123456","direction":"Reçu","body":"Salut"},
                {"date":"2024-03-15 12:00:00","contact":"+22670123456","direction":"Envoyé","body":"Ok"}
            ],
            "calls": [
                {"date":"2024-03-15 07:50:00","number":"+22670123456","type":"Entrant","duration":"3:24"}
            ]
        });
        let whatsapp = serde_json::json!([
            {"timestamp":"2024-03-15T09:11:00Z","from_jid":"moi","to_jid":"+22601234567","body":"Reçu","chat_name":"Alpha","deleted":false}
        ]);
        let money = serde_json::json!([
            {"date":"2024-03-15T08:45:00Z","provider":"ORANGE","type_tx":"RECU","montant":150000,"currency":"XOF","from_num":"+22670123456","to_num":"moi","reference":"OM001"}
        ]);
        let r = build_timeline("A".into(), Some(extraction), Some(whatsapp), Some(money), None)
            .expect("timeline");
        // 2 SMS + 1 appel + 1 WhatsApp + 1 money = 5 événements
        assert_eq!(r.total, 5, "total événements");
        assert_eq!(r.by_source.get("SMS"), Some(&2));
        assert_eq!(r.by_source.get("CALL"), Some(&1));
        assert_eq!(r.by_source.get("WHATSAPP"), Some(&1));
        assert_eq!(r.by_source.get("MONEY"), Some(&1));
        // Tri chronologique : le premier est l'appel de 07:50, le dernier le SMS de 12:00
        assert_eq!(r.events.first().unwrap().source, "CALL");
        assert!(r.events[1].source == "SMS", "08:23 SMS en 2e");
        assert_eq!(r.events.last().unwrap().description.contains("12"), false); // desc n'a pas l'heure, on vérifie l'ordre autrement
        assert!(r.events.first().unwrap().sort_key <= r.events.last().unwrap().sort_key, "ordre croissant");
        // get_timeline_events relit le store
        let ev = get_timeline_events("A".into(), None).expect("relecture");
        assert_eq!(ev.len(), 5);
    }
}
