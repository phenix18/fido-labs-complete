use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone, NaiveDateTime};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineEvent {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub source: String,
    pub actor: String,
    pub description: String,
    pub metadata: HashMap<String, String>,
    #[serde(skip)]
    pub sort_key: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimelineResult {
    pub case_id: String,
    pub total_count: usize,
    pub by_source: HashMap<String, usize>,
    pub by_day: HashMap<String, usize>,
    pub date_range: (String, String),
    pub events: Vec<TimelineEvent>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventCorrelation {
    pub primary_event: TimelineEvent,
    pub related_events: Vec<TimelineEvent>,
    pub correlation_type: String,
    pub time_gap_seconds: i64,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActivityGap {
    pub start: String,
    pub end: String,
    pub duration_hours: f64,
    pub description: String,
}

fn store() -> &'static Mutex<HashMap<String, Vec<TimelineEvent>>> {
    static S: OnceLock<Mutex<HashMap<String, Vec<TimelineEvent>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

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
    if s.chars().count() <= n { s } else { format!("{}...", s.chars().take(n).collect::<String>()) }
}

fn join(dir: &str, file: &str) -> String {
    if dir.is_empty() { file.to_string() }
    else { format!("{}/{}", dir.trim_end_matches(|c| c == '/' || c == '\\'), file) }
}

fn make_event(
    ts_raw: &str, source: &str, etype: &str,
    desc: String, actor: String, metadata: HashMap<String, String>,
) -> TimelineEvent {
    let (timestamp, sort_key) = match parse_ts(ts_raw) {
        Some(dt) => (dt.to_rfc3339(), dt.timestamp()),
        None => (ts_raw.to_string(), 0),
    };
    TimelineEvent {
        id: Uuid::new_v4().to_string(),
        timestamp,
        source: source.into(),
        event_type: etype.into(),
        description: desc,
        actor,
        metadata,
        sort_key,
    }
}

fn compute_by_day(events: &[TimelineEvent]) -> HashMap<String, usize> {
    let mut by_day: HashMap<String, usize> = HashMap::new();
    for e in events {
        if let Some(dt) = parse_ts(&e.timestamp) {
            let key = dt.format("%Y-%m-%d").to_string();
            *by_day.entry(key).or_insert(0) += 1;
        }
    }
    by_day
}

fn compute_date_range(events: &[TimelineEvent]) -> (String, String) {
    let first = events.iter().find(|e| e.sort_key > 0).map(|e| e.timestamp.clone()).unwrap_or_default();
    let last = events.iter().rev().find(|e| e.sort_key > 0).map(|e| e.timestamp.clone()).unwrap_or_default();
    (first, last)
}

fn extract_sms_calls(
    extraction: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>,
) {
    let dir = extraction.get("output_dir").and_then(|x| x.as_str()).unwrap_or("");
    if let Some(sms) = extraction.get("sms").and_then(|a| a.as_array()) {
        for m in sms {
            let date = sj(m, "date");
            let contact = sj(m, "contact");
            let dir_lbl = sj(m, "direction");
            let body = sj(m, "body");
            let received = {
                let d = dir_lbl.to_lowercase();
                d.starts_with("re") || d.contains("\u{e7}u") || d.contains("recu")
            };
            let actor = if received { contact.clone() } else { "Appareil".to_string() };
            let target = if received { "Appareil".to_string() } else { contact.clone() };
            let mut meta = HashMap::new();
            meta.insert("case_id".into(), case_id.into());
            meta.insert("target".into(), target);
            meta.insert("evidence_path".into(), join(dir, "sms.json"));
            events.push(make_event(
                &date, "SMS", "MESSAGE",
                format!("SMS {} : {}", dir_lbl.to_lowercase(), truncate(&body, 120)),
                actor, meta,
            ));
        }
    }
    if let Some(calls) = extraction.get("calls").and_then(|a| a.as_array()) {
        for c in calls {
            let date = sj(c, "date");
            let num = sj(c, "number");
            let typ = sj(c, "type");
            let dur = sj(c, "duration");
            let outgoing = typ.eq_ignore_ascii_case("Sortant") || typ.eq_ignore_ascii_case("outgoing");
            let actor = if outgoing { "Appareil".to_string() } else { num.clone() };
            let target = if outgoing { num.clone() } else { "Appareil".to_string() };
            let mut meta = HashMap::new();
            meta.insert("case_id".into(), case_id.into());
            meta.insert("target".into(), target);
            meta.insert("evidence_path".into(), join(dir, "calls.json"));
            events.push(make_event(
                &date, "CALL", "CALL",
                format!("Appel {} ({})", typ, if dur.is_empty() { "--".into() } else { dur }),
                actor, meta,
            ));
        }
    }
}

fn extract_whatsapp(whatsapp: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    if let Some(arr) = whatsapp.as_array() {
        for m in arr {
            let ts = sj(m, "timestamp");
            let from = sj(m, "from_jid");
            let to = sj(m, "to_jid");
            let body = sj(m, "body");
            let chat = sj(m, "chat_name");
            let deleted = m.get("deleted").and_then(|x| x.as_bool()).unwrap_or(false);
            let prefix = if deleted { "[efface] " } else { "" };
            let chat_lbl = if chat.is_empty() { String::new() } else { format!("{} -- ", chat) };
            let mut meta = HashMap::new();
            meta.insert("case_id".into(), case_id.into());
            meta.insert("target".into(), to);
            meta.insert("evidence_path".into(), "msgstore.db".into());
            events.push(make_event(
                &ts, "WHATSAPP", "MESSAGE",
                format!("{}{}{}", chat_lbl, prefix, truncate(&body, 120)),
                from, meta,
            ));
        }
    }
}

fn extract_money(money: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    if let Some(arr) = money.as_array() {
        for t in arr {
            let date = sj(t, "date");
            let provider = sj(t, "provider");
            let typ = sj(t, "type_tx");
            let montant = t.get("montant").and_then(|x| x.as_i64()).unwrap_or(0);
            let cur = sj(t, "currency");
            let from = sj(t, "from_num");
            let to = sj(t, "to_num");
            let reference = sj(t, "reference");
            let mut meta = HashMap::new();
            meta.insert("case_id".into(), case_id.into());
            meta.insert("target".into(), to);
            meta.insert("evidence_path".into(),
                if reference.is_empty() { "mobile_money".into() } else { reference });
            events.push(make_event(
                &date, "MONEY", "TRANSACTION",
                format!("{} {} {} {}", provider, typ, montant, cur),
                from, meta,
            ));
        }
    }
}

fn extract_browser_history(data: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    let arr = data.as_array().cloned().unwrap_or_else(|| {
        data.get("history").and_then(|v| v.as_array()).cloned().unwrap_or_default()
    });
    for item in arr {
        let ts = { let t = sj(&item, "timestamp"); if t.is_empty() { let t2 = sj(&item, "date"); if t2.is_empty() { sj(&item, "last_visit") } else { t2 } } else { t } };
        let url = sj(&item, "url");
        let title = sj(&item, "title");
        let visits = item.get("visits").and_then(|v| v.as_i64()).unwrap_or(1);
        let mut meta = HashMap::new();
        meta.insert("case_id".into(), case_id.into());
        meta.insert("url".into(), url.clone());
        meta.insert("visits".into(), visits.to_string());
        events.push(make_event(
            &ts, "BROWSER", "BROWSE",
            if title.is_empty() { truncate(&url, 120) } else { truncate(&title, 120) },
            "Appareil".into(), meta,
        ));
    }
}

fn extract_geolocation(data: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    let arr = data.as_array().cloned().unwrap_or_else(|| {
        data.get("locations").and_then(|v| v.as_array()).cloned().unwrap_or_default()
    });
    for item in arr {
        let ts = { let t = sj(&item, "timestamp"); if t.is_empty() { sj(&item, "date") } else { t } };
        let lat = item.get("lat").or_else(|| item.get("latitude")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lon = item.get("lon").or_else(|| item.get("longitude")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let place = sj(&item, "place");
        let mut meta = HashMap::new();
        meta.insert("case_id".into(), case_id.into());
        meta.insert("lat".into(), lat.to_string());
        meta.insert("lon".into(), lon.to_string());
        if !place.is_empty() { meta.insert("place".into(), place.clone()); }
        let desc = if place.is_empty() {
            format!("Position GPS ({:.6}, {:.6})", lat, lon)
        } else {
            format!("Position GPS: {} ({:.6}, {:.6})", place, lat, lon)
        };
        events.push(make_event(&ts, "GEOLOCATION", "LOCATION", desc, "Appareil".into(), meta));
    }
}

fn extract_emails(data: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    let arr = data.as_array().cloned().unwrap_or_else(|| {
        data.get("emails").and_then(|v| v.as_array()).cloned().unwrap_or_default()
    });
    for item in arr {
        let ts = { let t = sj(&item, "timestamp"); if t.is_empty() { let t2 = sj(&item, "date"); if t2.is_empty() { sj(&item, "sent_date") } else { t2 } } else { t } };
        let from = { let t = sj(&item, "from"); if t.is_empty() { sj(&item, "from_addr") } else { t } };
        let to = { let t = sj(&item, "to"); if t.is_empty() { sj(&item, "to_addr") } else { t } };
        let subject = sj(&item, "subject");
        let mut meta = HashMap::new();
        meta.insert("case_id".into(), case_id.into());
        meta.insert("target".into(), to);
        meta.insert("evidence_path".into(), "email_dump".into());
        events.push(make_event(
            &ts, "EMAIL", "EMAIL",
            if subject.is_empty() { "(sans objet)".into() } else { truncate(&subject, 120) },
            from, meta,
        ));
    }
}

fn extract_call_history(data: &serde_json::Value, case_id: &str, events: &mut Vec<TimelineEvent>) {
    let arr = data.as_array().cloned().unwrap_or_else(|| {
        data.get("calls").and_then(|v| v.as_array()).cloned().unwrap_or_default()
    });
    for item in arr {
        let ts = { let t = sj(&item, "timestamp"); if t.is_empty() { sj(&item, "date") } else { t } };
        let num = sj(&item, "number");
        let typ = sj(&item, "type");
        let dur = sj(&item, "duration");
        let outgoing = typ.eq_ignore_ascii_case("Sortant") || typ.eq_ignore_ascii_case("outgoing");
        let actor = if outgoing { "Appareil".to_string() } else { num.clone() };
        let target = if outgoing { num.clone() } else { "Appareil".to_string() };
        let mut meta = HashMap::new();
        meta.insert("case_id".into(), case_id.into());
        meta.insert("target".into(), target);
        meta.insert("evidence_path".into(), "call_history.json".into());
        events.push(make_event(
            &ts, "CALL", "CALL",
            format!("Appel {} ({})", typ, if dur.is_empty() { "--".into() } else { dur }),
            actor, meta,
        ));
    }
}

fn finalize_timeline(
    case_id: String, mut events: Vec<TimelineEvent>, limit: Option<usize>,
) -> TimelineResult {
    events.sort_by(|a, b| match (a.sort_key, b.sort_key) {
        (0, 0) => std::cmp::Ordering::Equal,
        (0, _) => std::cmp::Ordering::Greater,
        (_, 0) => std::cmp::Ordering::Less,
        (x, y) => x.cmp(&y),
    });

    let total = events.len();
    let mut by_source: HashMap<String, usize> = HashMap::new();
    for e in &events { *by_source.entry(e.source.clone()).or_insert(0) += 1; }

    let by_day = compute_by_day(&events);
    let date_range = compute_date_range(&events);

    if let Ok(mut g) = store().lock() {
        g.insert(case_id.clone(), events.clone());
    }

    let max = limit.unwrap_or(1000);
    events.truncate(max);

    TimelineResult { case_id, total_count: total, by_source, by_day, date_range, events }
}

#[tauri::command]
pub fn build_timeline(
    case_id: String,
    extraction: Option<serde_json::Value>,
    whatsapp: Option<serde_json::Value>,
    money: Option<serde_json::Value>,
    limit: Option<usize>,
) -> Result<TimelineResult, String> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    if let Some(ex) = extraction.as_ref() {
        extract_sms_calls(ex, &case_id, &mut events);
    }
    if let Some(wa) = whatsapp.as_ref() {
        extract_whatsapp(wa, &case_id, &mut events);
    }
    if let Some(mo) = money.as_ref() {
        extract_money(mo, &case_id, &mut events);
    }

    Ok(finalize_timeline(case_id, events, limit))
}

#[tauri::command]
pub fn build_comprehensive_timeline(
    case_id: String,
    extraction: Option<serde_json::Value>,
    whatsapp: Option<serde_json::Value>,
    money: Option<serde_json::Value>,
    browser_history: Option<serde_json::Value>,
    geolocation: Option<serde_json::Value>,
    emails: Option<serde_json::Value>,
    call_history: Option<serde_json::Value>,
    limit: Option<usize>,
) -> Result<TimelineResult, String> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    if let Some(ex) = extraction.as_ref() {
        extract_sms_calls(ex, &case_id, &mut events);
    }
    if let Some(wa) = whatsapp.as_ref() {
        extract_whatsapp(wa, &case_id, &mut events);
    }
    if let Some(mo) = money.as_ref() {
        extract_money(mo, &case_id, &mut events);
    }
    if let Some(bh) = browser_history.as_ref() {
        extract_browser_history(bh, &case_id, &mut events);
    }
    if let Some(geo) = geolocation.as_ref() {
        extract_geolocation(geo, &case_id, &mut events);
    }
    if let Some(em) = emails.as_ref() {
        extract_emails(em, &case_id, &mut events);
    }
    if let Some(ch) = call_history.as_ref() {
        extract_call_history(ch, &case_id, &mut events);
    }

    Ok(finalize_timeline(case_id, events, limit))
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

#[tauri::command]
pub fn correlate_events(timeline_result: serde_json::Value) -> Vec<EventCorrelation> {
    let events_val = timeline_result.get("events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut events: Vec<TimelineEvent> = Vec::new();
    for e in &events_val {
        if let Ok(ev) = serde_json::from_value::<TimelineEvent>(e.clone()) {
            if ev.sort_key > 0 {
                events.push(ev);
            }
        }
    }
    events.sort_by_key(|e| e.sort_key);

    let mut correlations: Vec<EventCorrelation> = Vec::new();
    let window_secs = 3600i64;

    for i in 0..events.len() {
        let primary = &events[i];
        for j in (i + 1)..events.len() {
            let candidate = &events[j];
            let gap = candidate.sort_key - primary.sort_key;
            if gap > window_secs { break; }
            if gap < 0 { continue; }

            let ctype = match (primary.source.as_str(), candidate.source.as_str()) {
                ("CALL", "SMS") | ("SMS", "CALL") => Some("CALL_THEN_MESSAGE"),
                ("BROWSER", "CALL") | ("BROWSER", "SMS") | ("BROWSER", "WHATSAPP") => Some("BROWSE_THEN_CONTACT"),
                ("CALL", "MONEY") | ("SMS", "MONEY") => Some("CONTACT_THEN_TRANSACTION"),
                ("EMAIL", "CALL") | ("EMAIL", "SMS") => Some("EMAIL_THEN_CONTACT"),
                ("GEOLOCATION", "CALL") | ("GEOLOCATION", "SMS") => Some("LOCATION_THEN_CONTACT"),
                _ => None,
            };

            if let Some(ct) = ctype {
                let desc = match ct {
                    "CALL_THEN_MESSAGE" => format!("Communication suivie d'un message dans l'heure"),
                    "BROWSE_THEN_CONTACT" => format!("Navigation web suivie d'un contact dans l'heure"),
                    "CONTACT_THEN_TRANSACTION" => format!("Communication suivie d'une transaction financiere"),
                    "EMAIL_THEN_CONTACT" => format!("Email suivi d'un contact dans l'heure"),
                    "LOCATION_THEN_CONTACT" => format!("Changement de position suivi d'un contact"),
                    _ => format!("Correlation temporelle detectee"),
                };
                correlations.push(EventCorrelation {
                    primary_event: primary.clone(),
                    related_events: vec![candidate.clone()],
                    correlation_type: ct.into(),
                    time_gap_seconds: gap,
                    description: desc,
                });
            }
        }
    }
    correlations
}

#[tauri::command]
pub fn detect_gaps(timeline_result: serde_json::Value, min_gap_hours: f64) -> Vec<ActivityGap> {
    let events_val = timeline_result.get("events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut events: Vec<TimelineEvent> = Vec::new();
    for e in &events_val {
        if let Ok(ev) = serde_json::from_value::<TimelineEvent>(e.clone()) {
            if ev.sort_key > 0 {
                events.push(ev);
            }
        }
    }
    events.sort_by_key(|e| e.sort_key);

    let min_secs = (min_gap_hours * 3600.0) as i64;
    let mut gaps: Vec<ActivityGap> = Vec::new();

    for i in 1..events.len() {
        let prev = &events[i - 1];
        let curr = &events[i];
        let gap_secs = curr.sort_key - prev.sort_key;
        if gap_secs > min_secs {
            let gap_hours = gap_secs as f64 / 3600.0;
            let start_dt = Utc.timestamp_opt(prev.sort_key, 0).single();
            let end_dt = Utc.timestamp_opt(curr.sort_key, 0).single();
            let (start_str, end_str) = match (start_dt, end_dt) {
                (Some(s), Some(e)) => (s.to_rfc3339(), e.to_rfc3339()),
                _ => (prev.timestamp.clone(), curr.timestamp.clone()),
            };
            gaps.push(ActivityGap {
                start: start_str,
                end: end_str,
                duration_hours: gap_hours,
                description: format!("Inactivite entre {} et {} ({:.1} h)",
                    prev.source, curr.source, gap_hours),
            });
        }
    }
    gaps
}

#[tauri::command]
pub fn filter_timeline(
    timeline_result: serde_json::Value,
    sources: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    actor: Option<String>,
    event_type: Option<String>,
) -> Vec<TimelineEvent> {
    let events_val = timeline_result.get("events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut filtered: Vec<TimelineEvent> = Vec::new();

    for e in &events_val {
        let ev = match serde_json::from_value::<TimelineEvent>(e.clone()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(ref srcs) = sources {
            if !srcs.is_empty() && !srcs.contains(&ev.source) {
                continue;
            }
        }

        if let Some(ref df) = date_from {
            if let (Some(evt_dt), Some(from_dt)) = (parse_ts(&ev.timestamp), parse_ts(df)) {
                if evt_dt < from_dt { continue; }
            }
        }

        if let Some(ref dt) = date_to {
            if let (Some(evt_dt), Some(to_dt)) = (parse_ts(&ev.timestamp), parse_ts(dt)) {
                if evt_dt > to_dt { continue; }
            }
        }

        if let Some(ref a) = actor {
            if !a.is_empty() && !ev.actor.to_lowercase().contains(&a.to_lowercase()) {
                continue;
            }
        }

        if let Some(ref et) = event_type {
            if !et.is_empty() && !ev.event_type.to_lowercase().contains(&et.to_lowercase()) {
                continue;
            }
        }

        filtered.push(ev);
    }
    filtered
}

#[tauri::command]
pub fn get_timeline_stats(timeline_result: serde_json::Value) -> serde_json::Value {
    let events_val = timeline_result.get("events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut events: Vec<TimelineEvent> = Vec::new();
    for e in &events_val {
        if let Ok(ev) = serde_json::from_value::<TimelineEvent>(e.clone()) {
            events.push(ev);
        }
    }

    let mut by_source: HashMap<String, usize> = HashMap::new();
    let mut by_day: HashMap<String, usize> = HashMap::new();
    let mut by_actor: HashMap<String, usize> = HashMap::new();

    for e in &events {
        *by_source.entry(e.source.clone()).or_insert(0) += 1;
        *by_actor.entry(e.actor.clone()).or_insert(0) += 1;
        if let Some(dt) = parse_ts(&e.timestamp) {
            let key = dt.format("%Y-%m-%d").to_string();
            *by_day.entry(key).or_insert(0) += 1;
        }
    }

    let mut by_source_sorted: Vec<(String, usize)> = by_source.into_iter().collect();
    by_source_sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut by_day_sorted: Vec<(String, usize)> = by_day.into_iter().collect();
    by_day_sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut by_actor_sorted: Vec<(String, usize)> = by_actor.into_iter().collect();
    by_actor_sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut top_actors: HashMap<String, usize> = HashMap::new();
    for (k, v) in by_actor_sorted.iter().take(10) {
        top_actors.insert(k.clone(), *v);
    }

    let mut busiest_days: HashMap<String, usize> = HashMap::new();
    let mut day_counts: Vec<(String, usize)> = by_day_sorted.clone();
    day_counts.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, v) in day_counts.iter().take(10) {
        busiest_days.insert(k.clone(), *v);
    }

    serde_json::json!({
        "total_events": events.len(),
        "by_source": by_source_sorted,
        "by_day": by_day_sorted,
        "top_actors": top_actors,
        "busiest_days": busiest_days,
        "unique_actors": by_actor_sorted.len(),
        "unique_days": by_day_sorted.len(),
    })
}

#[tauri::command]
pub fn export_timeline_csv(timeline_result: serde_json::Value) -> String {
    let events_val = timeline_result.get("events").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut csv = String::from("id,timestamp,source,event_type,actor,description\n");

    for e in &events_val {
        if let Ok(ev) = serde_json::from_value::<TimelineEvent>(e.clone()) {
            let desc = ev.description.replace("\"", "\"\"").replace(",", ";");
            csv.push_str(&format!("{},\"{}\",{},{},{},\"{}\"\n",
                ev.id, ev.timestamp, ev.source, ev.event_type, ev.actor, desc));
        }
    }
    csv
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
        assert_eq!(r.total_count, 5, "total evenements");
        assert_eq!(r.by_source.get("SMS"), Some(&2));
        assert_eq!(r.by_source.get("CALL"), Some(&1));
        assert_eq!(r.by_source.get("WHATSAPP"), Some(&1));
        assert_eq!(r.by_source.get("MONEY"), Some(&1));
        assert!(r.events.first().unwrap().sort_key <= r.events.last().unwrap().sort_key, "ordre croissant");
        assert!(r.by_day.contains_key("2024-03-15"), "by_day doit contenir 2024-03-15");
        let ev = get_timeline_events("A".into(), None).expect("relecture");
        assert_eq!(ev.len(), 5);
    }

    #[test]
    fn comprehensive_timeline_inclut_nouvelles_sources() {
        let extraction = serde_json::json!({
            "output_dir": "/cases/B",
            "sms": [],
            "calls": []
        });
        let whatsapp = serde_json::json!([]);
        let money = serde_json::json!([]);
        let browser = serde_json::json!([
            {"timestamp":"2024-03-15T10:00:00Z","url":"https://example.com","title":"Example","visits":1}
        ]);
        let geo = serde_json::json!([
            {"timestamp":"2024-03-15T10:05:00Z","lat":12.34,"lon":-1.56,"place":"Hotel"}
        ]);
        let emails = serde_json::json!([
            {"timestamp":"2024-03-15T10:10:00Z","from":"a@x.com","to":"b@y.com","subject":"Meeting"}
        ]);
        let calls = serde_json::json!([
            {"timestamp":"2024-03-15T10:15:00Z","number":"+22600000000","type":"Sortant","duration":"1:00"}
        ]);
        let r = build_comprehensive_timeline(
            "B".into(), Some(extraction), Some(whatsapp), Some(money),
            Some(browser), Some(geo), Some(emails), Some(calls), None,
        ).expect("timeline");
        assert_eq!(r.total_count, 4);
        assert!(r.by_source.contains_key("BROWSER"));
        assert!(r.by_source.contains_key("GEOLOCATION"));
        assert!(r.by_source.contains_key("EMAIL"));
        assert!(r.by_source.contains_key("CALL"));
    }

    #[test]
    fn detecte_correlations_appel_sms() {
        let events = vec![
            TimelineEvent { id: "1".into(), timestamp: "2024-03-15T08:00:00Z".into(), event_type: "CALL".into(), source: "CALL".into(), actor: "Appareil".into(), description: "Appel".into(), metadata: HashMap::new(), sort_key: 1710489600 },
            TimelineEvent { id: "2".into(), timestamp: "2024-03-15T08:05:00Z".into(), event_type: "MESSAGE".into(), source: "SMS".into(), actor: "+226".into(), description: "SMS".into(), metadata: HashMap::new(), sort_key: 1710489900 },
            TimelineEvent { id: "3".into(), timestamp: "2024-03-15T09:00:00Z".into(), event_type: "MESSAGE".into(), source: "WHATSAPP".into(), actor: "x".into(), description: "WA".into(), metadata: HashMap::new(), sort_key: 1710493200 },
        ];
        let mut sorted = events.clone();
        sorted.sort_by_key(|e| e.sort_key);
        let mut correlations: Vec<EventCorrelation> = Vec::new();
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let gap = sorted[j].sort_key - sorted[i].sort_key;
                if gap > 3600 { break; }
                if (sorted[i].source.as_str(), sorted[j].source.as_str()) == ("CALL", "SMS") {
                    correlations.push(EventCorrelation { primary_event: sorted[i].clone(), related_events: vec![sorted[j].clone()], correlation_type: "CALL_THEN_MESSAGE".into(), time_gap_seconds: gap, description: "Communication suivie d'un message".into() });
                }
            }
        }
        assert!(!correlations.is_empty());
        assert!(correlations.iter().any(|c| c.correlation_type == "CALL_THEN_MESSAGE"));
    }

    #[test]
    fn detecte_ecarts_activite() {
        let events = vec![
            TimelineEvent { id: "1".into(), timestamp: "2024-03-15T08:00:00Z".into(), event_type: "CALL".into(), source: "CALL".into(), actor: "A".into(), description: "x".into(), metadata: HashMap::new(), sort_key: 1710489600 },
            TimelineEvent { id: "2".into(), timestamp: "2024-03-15T14:00:00Z".into(), event_type: "MESSAGE".into(), source: "SMS".into(), actor: "B".into(), description: "y".into(), metadata: HashMap::new(), sort_key: 1710511200 },
        ];
        let min_secs = (4.0 * 3600.0) as i64;
        let mut gaps: Vec<ActivityGap> = Vec::new();
        for i in 1..events.len() {
            let gap_secs = events[i].sort_key - events[i - 1].sort_key;
            if gap_secs > min_secs {
                gaps.push(ActivityGap { start: events[i - 1].timestamp.clone(), end: events[i].timestamp.clone(), duration_hours: gap_secs as f64 / 3600.0, description: "Inactivite detectee".into() });
            }
        }
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].duration_hours >= 5.9);
    }

    #[test]
    fn filtre_par_source_et_acteur() {
        let timeline = serde_json::json!({
            "events": [
                {"id":"1","timestamp":"2024-03-15T08:00:00Z","event_type":"CALL","source":"CALL","actor":"Alice","description":"x","metadata":{},"sort_key":1710489600},
                {"id":"2","timestamp":"2024-03-15T09:00:00Z","event_type":"MESSAGE","source":"SMS","actor":"Bob","description":"y","metadata":{},"sort_key":1710493200}
            ]
        });
        let filtered = filter_timeline(
            timeline.clone(),
            Some(vec!["SMS".into()]),
            None, None, None, None,
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source, "SMS");

        let filtered2 = filter_timeline(
            timeline.clone(),
            None, None, None, Some("Alice".into()), None,
        );
        assert_eq!(filtered2.len(), 1);
        assert_eq!(filtered2[0].actor, "Alice");
    }

    #[test]
    fn export_csv_format_valide() {
        let timeline = serde_json::json!({
            "events": [
                {"id":"1","timestamp":"2024-03-15T08:00:00Z","event_type":"CALL","source":"CALL","actor":"A","description":"Test, with comma","metadata":{},"sort_key":1710489600}
            ]
        });
        let csv = export_timeline_csv(timeline);
        assert!(csv.contains("id,timestamp,source,event_type,actor,description"));
        assert!(csv.contains("Test; with comma"));
    }
}
