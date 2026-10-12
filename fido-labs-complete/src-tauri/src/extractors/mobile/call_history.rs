use chrono::Timelike;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct CallEvent {
    pub id: String,
    pub phone_number: String,
    pub contact_name: String,
    pub call_type: CallType,
    pub direction: CallDirection,
    pub date: String,
    pub duration: String,
    pub duration_seconds: i64,
    pub location: String,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub is_sim1: bool,
    pub is_sim2: bool,
    pub is_international: bool,
    pub is_roaming: bool,
    pub category: CallCategory,
    pub risk_level: RiskLevel,
    pub notes: String,
    pub source: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CallType {
    Voice, Video, Conference, Missed, Voicemail,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CallDirection {
    Incoming, Outgoing, Missed, Rejected, Blocked, Voicemail, Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CallCategory {
    Personal, Professional, Family, Unknown, Spam, Suspicious,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum RiskLevel {
    Low, Medium, High, Critical,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CallSummary {
    pub total_calls: i64,
    pub total_duration: String,
    pub incoming_count: i64,
    pub outgoing_count: i64,
    pub missed_count: i64,
    pub unique_contacts: usize,
    pub by_category: HashMap<String, i64>,
    pub by_risk_level: HashMap<String, i64>,
    pub top_contacts: Vec<(String, i64)>,
    pub date_range: (String, String),
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CallHistoryResult {
    pub case_id: String,
    pub output_dir: String,
    pub calls: Vec<CallEvent>,
    pub summary: CallSummary,
    pub by_date: HashMap<String, i64>,
    pub by_contact: Vec<(String, i64)>,
    pub total_calls: i64,
    pub total_duration: String,
    pub sha256: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CallPattern {
    pub pattern_type: String,
    pub phone_number: String,
    pub description: String,
    pub confidence: f64,
    pub related_calls: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CallChain {
    pub chain: Vec<String>,
    pub start_time: String,
    pub end_time: String,
    pub description: String,
}

fn classify_call(number: &str, _duration: i64) -> (CallCategory, RiskLevel) {
    let num = number.replace('+', "").replace(' ', "");
    let mut category = CallCategory::Unknown;
    let mut risk = RiskLevel::Low;
    if num.contains("0800") || num.contains("080") || num.contains("surcharge") {
        category = CallCategory::Spam;
        risk = RiskLevel::Medium;
    } else if num.contains("15") || num.contains("17") || num.contains("18") {
        category = CallCategory::Professional;
        risk = RiskLevel::Low;
    } else if num.len() > 12 {
        category = CallCategory::Personal;
        risk = RiskLevel::Low;
    } else {
        category = CallCategory::Personal;
        risk = RiskLevel::Low;
    }
    if num.starts_with("226") { category = CallCategory::Professional; }
    (category, risk)
}

fn categorize_direction(t: &str) -> CallDirection {
    match t {
        "1" => CallDirection::Incoming,
        "2" => CallDirection::Outgoing,
        "3" => CallDirection::Missed,
        "4" => CallDirection::Voicemail,
        "5" => CallDirection::Rejected,
        "6" => CallDirection::Blocked,
        _ => CallDirection::Unknown,
    }
}

fn categorize_type(t: &str) -> CallType {
    match t {
        "1" => CallType::Voice,
        "2" => CallType::Voice,
        "3" => CallType::Missed,
        "4" => CallType::Voicemail,
        _ => CallType::Voice,
    }
}

fn categorize_call_type(t: &str) -> CallType {
    match t {
        "1" => CallType::Voice,
        "2" => CallType::Video,
        "3" => CallType::Missed,
        "4" => CallType::Voicemail,
        _ => CallType::Voice,
    }
}

fn parse_calls(source_path: &str) -> Result<Vec<CallEvent>, String> {
    let path = std::path::PathBuf::from(source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let mut calls = Vec::new();
    let text = std::fs::read_to_string(&path).map_err(|_| format!("Impossible de lire {}", source_path))?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("Row:") || line.is_empty() { continue; }
        let mut phone = String::new();
        let mut date = String::new();
        let mut duration = String::new();
        let mut call_type = "1";
        let mut contact_name = String::new();
        let mut sim_slot = String::new();
        for field in line.split(',').filter(|f| f.trim().contains('=')) {
            let parts: Vec<&str> = field.splitn(2, '=').collect();
            if parts.len() != 2 { continue; }
            let key = parts[0].trim();
            let val = parts[1].trim();
            match key {
                "number" | "address" => phone = val.to_string(),
                "date" => date = val.to_string(),
                "duration" => duration = val.to_string(),
                "type" => call_type = val,
                "name" | "contact_name" => contact_name = val.to_string(),
                "sim_id" | "sim_slot" => sim_slot = val.to_string(),
                _ => {}
            }
        }
        if phone.is_empty() { continue; }
        let dur_secs = duration.parse::<i64>().unwrap_or(0);
        let dir = categorize_direction(call_type);
        let ctype = categorize_call_type(call_type);
        let (cat, risk) = classify_call(&phone, dur_secs);
        let dur_str = format!("{}:{:02}", dur_secs / 60, dur_secs % 60);
        let (lat, lon) = (None, None);
        calls.push(CallEvent {
            id: Uuid::new_v4().to_string(),
            phone_number: phone.clone(),
            contact_name: if contact_name.is_empty() { "Inconnu".into() } else { contact_name },
            call_type: ctype,
            direction: dir,
            date: date.clone(),
            duration: dur_str,
            duration_seconds: dur_secs,
            location: String::new(),
            location_lat: lat,
            location_lon: lon,
            is_sim1: sim_slot == "0" || sim_slot.is_empty(),
            is_sim2: sim_slot == "1",
            is_international: phone.starts_with("+") || phone.len() > 12,
            is_roaming: false,
            category: cat,
            risk_level: risk,
            notes: String::new(),
            source: source_path.to_string(),
            sha256: String::new(),
        });
    }
    for c in &mut calls {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}:{}", c.phone_number, c.date, c.duration, format!("{:?}", c.direction), format!("{:?}", c.category)).as_bytes());
        c.sha256 = hex::encode(hasher.finalize());
    }
    Ok(calls)
}

fn parse_call_timestamp(date: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(ms) = date.parse::<i64>() {
        return chrono::DateTime::from_timestamp(ms / 1000, (ms % 1_000) as u32 * 1_000_000);
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S") {
        return Some(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
    }
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|naive| chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc))
}

fn day_key(date: &str) -> String {
    if date.len() >= 10 { date[..10].to_string() } else { date.to_string() }
}

fn normalize_number(n: &str) -> String {
    n.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn numbers_match(a: &str, b: &str) -> bool {
    let na = normalize_number(a);
    let nb = normalize_number(b);
    if na.is_empty() || nb.is_empty() { return false; }
    if na == nb { return true; }
    na.len() >= 8 && nb.len() >= 8 && na.len().abs_diff(nb.len()) <= 3 && (na.ends_with(&nb) || nb.ends_with(&na))
}

fn dynamic_risk(call: &CallEvent, frequency: usize) -> RiskLevel {
    let mut score = 0i64;
    if frequency >= 10 { score += 3; } else if frequency >= 5 { score += 2; } else if frequency >= 2 { score += 1; }
    if call.duration_seconds > 600 { score += 2; } else if call.duration_seconds > 120 { score += 1; }
    if let Some(t) = parse_call_timestamp(&call.date) {
        let h = t.hour();
        if h >= 23 || h < 5 { score += 3; } else if h >= 21 || h < 7 { score += 1; }
    }
    if call.is_international { score += 1; }
    if call.is_roaming { score += 1; }
    match score {
        0..=2 => RiskLevel::Low,
        3..=4 => RiskLevel::Medium,
        5..=6 => RiskLevel::High,
        _ => RiskLevel::Critical,
    }
}

fn ensure_output_dir(output_dir: &str) -> Result<String, String> {
    let base = output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)).to_string();
    std::fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    Ok(base)
}

#[tauri::command]
pub fn extract_call_history(case_id: String, source_path: String, output_dir: String) -> Result<CallHistoryResult, String> {
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/call_history_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    std::fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let calls = parse_calls(&source_path)?;
    let total_calls = calls.len() as i64;
    let total_secs: i64 = calls.iter().map(|c| c.duration_seconds).sum();
    let total_duration = format!("{}:{:02}", total_secs / 60, total_secs % 60);
    let incoming = calls.iter().filter(|c| c.direction == CallDirection::Incoming).count() as i64;
    let outgoing = calls.iter().filter(|c| c.direction == CallDirection::Outgoing).count() as i64;
    let missed = calls.iter().filter(|c| c.direction == CallDirection::Missed).count() as i64;
    let unique_contacts = calls.iter().map(|c| c.phone_number.clone()).collect::<std::collections::HashSet<_>>().len();
    let mut by_category: HashMap<String, i64> = HashMap::new();
    let mut by_risk: HashMap<String, i64> = HashMap::new();
    let mut by_date: HashMap<String, i64> = HashMap::new();
    let mut contact_counts: HashMap<String, i64> = HashMap::new();
    for c in &calls {
        *by_category.entry(format!("{:?}", c.category)).or_insert(0) += 1;
        *by_risk.entry(format!("{:?}", c.risk_level)).or_insert(0) += 1;
        *by_date.entry(c.date.clone()).or_insert(0) += 1;
        *contact_counts.entry(c.phone_number.clone()).or_insert(0) += 1;
    }
    let mut top_contacts: Vec<(String, i64)> = contact_counts.into_iter().collect();
    top_contacts.sort_by(|a, b| b.1.cmp(&a.1)); top_contacts.truncate(20);
    let dates: Vec<&String> = by_date.keys().collect();
    let date_range = if !dates.is_empty() { (dates[0].clone(), dates[dates.len()-1].clone()) } else { (String::new(), String::new()) };
    let summary = CallSummary {
        total_calls, total_duration: total_duration.clone(), incoming_count: incoming, outgoing_count: outgoing, missed_count: missed,
        unique_contacts, by_category, by_risk_level: by_risk, top_contacts: top_contacts.clone(), date_range,
        sha256: String::new(),
    };
    let json = serde_json::to_string_pretty(&calls).unwrap_or_default();
    std::fs::write(format!("{}/call_history.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    let mut by_date_sorted: HashMap<String, i64> = HashMap::new();
    for c in &calls { *by_date_sorted.entry(day_key(&c.date)).or_insert(0) += 1; }
    let mut by_contact_sorted: Vec<(String, i64)> = top_contacts.clone();
    by_contact_sorted.sort_by(|a, b| b.1.cmp(&a.1)); by_contact_sorted.truncate(50);
    Ok(CallHistoryResult {
        case_id, output_dir: base, calls: calls.iter().take(5000).cloned().collect(), summary,
        by_date: by_date_sorted, by_contact: by_contact_sorted, total_calls, total_duration, sha256: sha, success: true,
    })
}

#[tauri::command]
pub fn get_call_statistics(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let result = extract_call_history(case_id, source_path, output_dir)?;
    Ok(serde_json::json!({"success": true, "case_id": result.case_id, "summary": serde_json::to_value(result.summary).unwrap(), "by_date": result.by_date, "top_contacts": result.by_contact, "total_calls": result.total_calls, "total_duration": result.total_duration}))
}

#[tauri::command]
pub fn detect_patterns(source_path: String, output_dir: String) -> Result<Vec<CallPattern>, String> {
    let calls = parse_calls(&source_path)?;
    let base = ensure_output_dir(&output_dir)?;
    let mut by_number: HashMap<String, Vec<&CallEvent>> = HashMap::new();
    for c in &calls { by_number.entry(c.phone_number.clone()).or_default().push(c); }
    let mut patterns: Vec<CallPattern> = Vec::new();
    for (number, group) in &by_number {
        let mut sorted: Vec<&CallEvent> = group.clone();
        sorted.sort_by(|a, b| a.date.cmp(&b.date));
        let mut days: Vec<String> = sorted.iter().map(|c| day_key(&c.date)).collect();
        days.sort();
        days.dedup();
        if days.len() >= 3 {
            patterns.push(CallPattern {
                pattern_type: "daily_regular".into(),
                phone_number: number.clone(),
                description: format!("Appels réguliers vers ce numéro sur {} jours distincts", days.len()),
                confidence: 0.9,
                related_calls: sorted.iter().map(|c| c.id.clone()).collect(),
            });
        }
        let mut timestamps: Vec<chrono::DateTime<chrono::Utc>> = sorted.iter().filter_map(|c| parse_call_timestamp(&c.date)).collect();
        timestamps.sort();
        let mut burst = 0usize;
        for i in 0..timestamps.len() {
            let window = timestamps.iter().filter(|t| (**t - timestamps[i]).num_seconds().abs() <= 3600).count();
            if window >= 5 { burst = window; break; }
        }
        if burst >= 5 {
            patterns.push(CallPattern {
                pattern_type: "burst".into(),
                phone_number: number.clone(),
                description: format!("Rafale de {} appels en moins d'une heure", burst),
                confidence: 0.85,
                related_calls: sorted.iter().map(|c| c.id.clone()).collect(),
            });
        }
        let unusual: Vec<String> = sorted.iter().filter_map(|c| {
            parse_call_timestamp(&c.date).and_then(|t| {
                let h = t.hour();
                if h >= 23 || h < 5 { Some(c.id.clone()) } else { None }
            })
        }).collect();
        if !unusual.is_empty() {
            patterns.push(CallPattern {
                pattern_type: "unusual_hours".into(),
                phone_number: number.clone(),
                description: format!("{} appel(s) entre 23h00 et 05h00", unusual.len()),
                confidence: 0.7,
                related_calls: unusual,
            });
        }
        if sorted.len() == 1 {
            patterns.push(CallPattern {
                pattern_type: "one_time".into(),
                phone_number: number.clone(),
                description: "Numéro contacté une seule fois".into(),
                confidence: 0.5,
                related_calls: vec![sorted[0].id.clone()],
            });
        }
        let short: Vec<String> = sorted.iter().filter(|c| c.duration_seconds >= 0 && c.duration_seconds < 10).map(|c| c.id.clone()).collect();
        if short.len() >= 3 {
            patterns.push(CallPattern {
                pattern_type: "short_duration".into(),
                phone_number: number.clone(),
                description: format!("{} appels de moins de 10 secondes", short.len()),
                confidence: 0.75,
                related_calls: short,
            });
        }
    }
    patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    let json = serde_json::to_string_pretty(&patterns).unwrap_or_default();
    std::fs::write(format!("{}/call_patterns.json", base), &json).ok();
    Ok(patterns)
}

#[tauri::command]
pub fn resolve_contacts(source_path: String, contacts: Vec<serde_json::Value>, output_dir: String) -> Result<serde_json::Value, String> {
    let calls = parse_calls(&source_path)?;
    let base = ensure_output_dir(&output_dir)?;
    let mut resolved: Vec<serde_json::Value> = Vec::new();
    let mut matched_numbers: HashMap<String, (String, String)> = HashMap::new();
    for c in &calls {
        let mut match_name = String::new();
        let mut match_phone = String::new();
        for contact in &contacts {
            let name = contact.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let phones: Vec<String> = contact.get("phones")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|p| p.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            for ph in phones {
                if numbers_match(&c.phone_number, &ph) {
                    match_name = name.clone();
                    match_phone = ph.clone();
                    break;
                }
            }
            if !match_name.is_empty() { break; }
        }
        if !match_name.is_empty() {
            matched_numbers.entry(c.phone_number.clone()).or_insert((match_name.clone(), match_phone.clone()));
        }
        resolved.push(serde_json::json!({
            "id": c.id,
            "phone_number": c.phone_number,
            "resolved_name": if match_name.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(match_name.clone()) },
            "resolved_phone": if match_phone.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(match_phone) },
            "date": c.date,
            "direction": format!("{:?}", c.direction),
            "duration_seconds": c.duration_seconds,
            "matched": !match_name.is_empty(),
        }));
    }
    let match_count = resolved.iter().filter(|r| r.get("matched").and_then(|m| m.as_bool()).unwrap_or(false)).count();
    let result = serde_json::json!({
        "total_calls": resolved.len(),
        "matched_calls": match_count,
        "unmatched_calls": resolved.len() - match_count,
        "unique_matched_numbers": matched_numbers.len(),
        "resolved_numbers": matched_numbers,
        "calls": resolved,
    });
    let json = serde_json::to_string_pretty(&result).unwrap_or_default();
    std::fs::write(format!("{}/resolved_contacts.json", base), &json).ok();
    Ok(result)
}

#[tauri::command]
pub fn detect_call_chains(source_path: String, output_dir: String) -> Result<Vec<CallChain>, String> {
    let calls = parse_calls(&source_path)?;
    let base = ensure_output_dir(&output_dir)?;
    let mut timed: Vec<(chrono::DateTime<chrono::Utc>, &CallEvent)> = calls.iter()
        .filter_map(|c| parse_call_timestamp(&c.date).map(|t| (t, c)))
        .collect();
    timed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut chains: Vec<CallChain> = Vec::new();
    let mut cur_numbers: Vec<String> = Vec::new();
    let mut cur_times: Vec<chrono::DateTime<chrono::Utc>> = Vec::new();
    let mut last: Option<chrono::DateTime<chrono::Utc>> = None;
    for (t, c) in &timed {
        if let Some(l) = last {
            if (*t - l).num_seconds() > 600 {
                if cur_numbers.len() >= 2 {
                    let start = cur_times.first().cloned().unwrap_or(*t);
                    let end = cur_times.last().cloned().unwrap_or(*t);
                    let desc = cur_numbers.iter()
                        .map(|n| n.as_str())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    chains.push(CallChain {
                        chain: std::mem::take(&mut cur_numbers),
                        start_time: start.to_rfc3339(),
                        end_time: end.to_rfc3339(),
                        description: desc,
                    });
                } else {
                    cur_numbers.clear();
                }
                cur_times.clear();
            }
        }
        cur_numbers.push(c.phone_number.clone());
        cur_times.push(*t);
        last = Some(*t);
    }
    if cur_numbers.len() >= 2 {
        let start = cur_times.first().cloned().unwrap_or_default();
        let end = cur_times.last().cloned().unwrap_or_default();
        let desc = cur_numbers.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(" -> ");
        chains.push(CallChain {
            chain: cur_numbers,
            start_time: start.to_rfc3339(),
            end_time: end.to_rfc3339(),
            description: desc,
        });
    }
    chains.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    let json = serde_json::to_string_pretty(&chains).unwrap_or_default();
    std::fs::write(format!("{}/call_chains.json", base), &json).ok();
    Ok(chains)
}

#[tauri::command]
pub fn get_call_timeline(source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let calls = parse_calls(&source_path)?;
    let base = ensure_output_dir(&output_dir)?;
    let mut freq: HashMap<String, usize> = HashMap::new();
    for c in &calls { *freq.entry(c.phone_number.clone()).or_insert(0) += 1; }
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for c in &calls {
        let risk = dynamic_risk(c, freq.get(&c.phone_number).copied().unwrap_or(1));
        let ts = parse_call_timestamp(&c.date);
        entries.push(serde_json::json!({
            "id": c.id,
            "timestamp": ts.map(|t| t.to_rfc3339()).unwrap_or_else(|| c.date.clone()),
            "phone_number": c.phone_number,
            "contact_name": c.contact_name,
            "direction": format!("{:?}", c.direction),
            "call_type": format!("{:?}", c.call_type),
            "duration_seconds": c.duration_seconds,
            "hour": ts.map(|t| t.hour()),
            "is_night": ts.map(|t| { let h = t.hour(); h >= 23 || h < 5 }).unwrap_or(false),
            "is_international": c.is_international,
            "is_roaming": c.is_roaming,
            "category": format!("{:?}", c.category),
            "base_risk": format!("{:?}", c.risk_level),
            "dynamic_risk": format!("{:?}", risk),
            "frequency": freq.get(&c.phone_number).copied().unwrap_or(1),
        }));
    }
    entries.sort_by(|a, b| {
        let ta = a.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        let tb = b.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        ta.cmp(tb)
    });
    let mut by_date: HashMap<String, i64> = HashMap::new();
    for c in &calls { *by_date.entry(day_key(&c.date)).or_insert(0) += 1; }
    let mut by_risk: HashMap<String, i64> = HashMap::new();
    for e in &entries {
        *by_risk.entry(e.get("dynamic_risk").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string()).or_insert(0) += 1;
    }
    let result = serde_json::json!({
        "total_calls": entries.len(),
        "by_date": by_date,
        "by_dynamic_risk": by_risk,
        "entries": entries,
    });
    let json = serde_json::to_string_pretty(&result).unwrap_or_default();
    std::fs::write(format!("{}/call_timeline.json", base), &json).ok();
    Ok(result)
}

#[tauri::command]
pub fn export_calls_csv(source_path: String, output_dir: String) -> Result<String, String> {
    let calls = parse_calls(&source_path)?;
    let base = ensure_output_dir(&output_dir)?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let path = format!("{}/calls_{}.csv", base, ts);
    let mut csv = String::from("id,phone_number,contact_name,call_type,direction,date,duration_seconds,category,risk_level,is_international,is_roaming,sha256\n");
    for c in &calls {
        csv.push_str(&format!("\"{}\",\"{}\",\"{}\",\"{:?}\",\"{:?}\",\"{}\",{},\"{:?}\",\"{:?}\",{},{},\"{}\"\n",
            c.id, c.phone_number, c.contact_name, c.call_type, c.direction, c.date, c.duration_seconds,
            c.category, c.risk_level, c.is_international, c.is_roaming, c.sha256));
    }
    std::fs::write(&path, &csv).map_err(|e| format!("Écriture CSV {}: {}", path, e))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_categorize_direction() {
        assert!(matches!(categorize_direction("1"), CallDirection::Incoming));
        assert!(matches!(categorize_direction("2"), CallDirection::Outgoing));
        assert!(matches!(categorize_direction("3"), CallDirection::Missed));
    }
    #[test] fn test_classify_call() {
        let (cat, risk) = classify_call("+22670123456", 120);
        assert!(matches!(cat, CallCategory::Professional));
    }
    #[test] fn test_normalize_number() {
        assert_eq!(normalize_number("+226 70 12 34 56"), "22670123456");
        assert_eq!(normalize_number("0800123456"), "0800123456");
    }
    #[test] fn test_numbers_match() {
        assert!(numbers_match("+22670123456", "70123456"));
        assert!(numbers_match("0800123456", "0800123456"));
        assert!(!numbers_match("70123456", "71123456"));
    }
    #[test] fn test_dynamic_risk() {
        let call = CallEvent {
            id: Uuid::new_v4().to_string(),
            phone_number: "+22670123456".into(),
            contact_name: "Test".into(),
            call_type: CallType::Voice,
            direction: CallDirection::Outgoing,
            date: "1693483200000".into(),
            duration: "5:00".into(),
            duration_seconds: 300,
            location: String::new(),
            location_lat: None,
            location_lon: None,
            is_sim1: true,
            is_sim2: false,
            is_international: false,
            is_roaming: false,
            category: CallCategory::Personal,
            risk_level: RiskLevel::Low,
            notes: String::new(),
            source: String::new(),
            sha256: String::new(),
        };
        let risk = dynamic_risk(&call, 12);
        assert!(matches!(risk, RiskLevel::Medium));
    }
    #[test] fn test_parse_call_timestamp() {
        assert!(parse_call_timestamp("1693483200000").is_some());
        assert!(parse_call_timestamp("2023-09-01").is_some());
        assert!(parse_call_timestamp("2023-09-01T12:00:00Z").is_some());
        assert!(parse_call_timestamp("invalid").is_none());
    }
}
