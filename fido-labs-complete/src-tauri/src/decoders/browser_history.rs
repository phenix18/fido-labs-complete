#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Timelike;
use chrono::Datelike;

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub domain: String,
    pub visit_count: i64,
    pub last_visit: String,
    pub first_visit: String,
    pub visit_duration: String,
    pub category: HistoryCategory,
    pub source: String,
    pub browser: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum HistoryCategory {
    SocialMedia, Video, Shopping, News, Search, Communication, Banking, DarkWeb, Cloud, Developer, Other,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BrowserCookie {
    pub id: String,
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expiry: String,
    pub is_secure: bool,
    pub is_http_only: bool,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryExtractionResult {
    pub case_id: String,
    pub output_dir: String,
    pub entries: Vec<HistoryEntry>,
    pub cookies: Vec<BrowserCookie>,
    pub by_browser: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub total_entries: usize,
    pub total_cookies: usize,
    pub top_domains: Vec<(String, i64)>,
    pub sha256: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchQuery {
    pub id: String,
    pub query: String,
    pub engine: String,
    pub timestamp: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadRecord {
    pub id: String,
    pub url: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub timestamp: String,
    pub state: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BrowsingPattern {
    pub most_active_hour: u32,
    pub most_active_day: String,
    pub avg_session_minutes: f64,
    pub late_night_visits: usize,
    pub weekend_ratio: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InterestProfile {
    pub top_categories: Vec<(String, usize)>,
    pub top_domains: Vec<(String, usize)>,
    pub search_topics: Vec<(String, usize)>,
    pub browsing_patterns: BrowsingPattern,
    pub risk_indicators: Vec<String>,
}

fn categorize_url(url: &str) -> HistoryCategory {
    let u = url.to_lowercase();
    if u.contains("youtube.com") || u.contains("tiktok.com") || u.contains("netflix.com") || u.contains("twitch.tv") || u.contains("vimeo.com") { HistoryCategory::Video }
    else if u.contains("facebook.com") || u.contains("twitter.com") || u.contains("instagram.com") || u.contains("linkedin.com") || u.contains("reddit.com") || x_social(&u) { HistoryCategory::SocialMedia }
    else if u.contains("amazon.com") || u.contains("aliexpress.com") || u.contains("ebay.com") || u.contains("etsy.com") || x_shopping(&u) { HistoryCategory::Shopping }
    else if u.contains("google.com/search") || u.contains("bing.com") || u.contains("duckduckgo.com") || u.contains("yahoo.com/search") || x_search(&u) { HistoryCategory::Search }
    else if u.contains("bank") || u.contains("paypal.com") || u.contains("credit") || x_banking(&u) { HistoryCategory::Banking }
    else if u.contains("darkweb") || u.contains("onion") || u.contains("tor") || x_darkweb(&u) { HistoryCategory::DarkWeb }
    else if u.contains("news") || u.contains("lemonde") || u.contains("franceinter") || u.contains("bfm") || u.contains("cnn.com") || u.contains("bbc.com") || u.contains("reuters.com") { HistoryCategory::News }
    else if u.contains("whatsapp") || u.contains("telegram") || u.contains("discord") || u.contains("slack.com") || x_communication(&u) { HistoryCategory::Communication }
    else if u.contains("drive.google") || u.contains("dropbox.com") || u.contains("onedrive") || u.contains("icloud.com") || x_cloud(&u) { HistoryCategory::Cloud }
    else if u.contains("github.com") || u.contains("stackoverflow.com") || u.contains("gitlab.com") || u.contains("npmjs.com") || x_developer(&u) { HistoryCategory::Developer }
    else { HistoryCategory::Other }
}

fn x_social(u: &str) -> bool { u.contains("snapchat.com") || u.contains("pinterest.com") || u.contains("threads.net") }
fn x_shopping(u: &str) -> bool { u.contains("walmart.com") || u.contains("target.com") || u.contains("bestbuy.com") }
fn x_search(u: &str) -> bool { u.contains("search.yahoo.com") || u.contains("search.brave.com") || u.contains("qwant.com") }
fn x_banking(u: &str) -> bool { u.contains("chase.com") || u.contains("wellsfargo.com") || u.contains("citibank") }
fn x_darkweb(u: &str) -> bool { u.contains(".onion") || u.contains("torproject.org") || u.contains("i2p") }
fn x_communication(u: &str) -> bool { u.contains("signal.org") || u.contains("messenger.com") || u.contains("teams.microsoft.com") }
fn x_cloud(u: &str) -> bool { u.contains("box.com") || u.contains("mega.nz") || u.contains("pcloud.com") }
fn x_developer(u: &str) -> bool { u.contains("crates.io") || u.contains("pypi.org") || u.contains("docker.com") || u.contains("kubernetes.io") }

fn detect_search_engine(url: &str) -> String {
    let u = url.to_lowercase();
    if u.contains("google.com/search") || u.contains("google.fr/search") { "Google".to_string() }
    else if u.contains("bing.com/search") { "Bing".to_string() }
    else if u.contains("duckduckgo.com") { "DuckDuckGo".to_string() }
    else if u.contains("yahoo.com/search") || u.contains("search.yahoo.com") { "Yahoo".to_string() }
    else if u.contains("search.brave.com") { "Brave".to_string() }
    else if u.contains("qwant.com") { "Qwant".to_string() }
    else { "Unknown".to_string() }
}

fn extract_query_from_url(url: &str) -> Option<String> {
    let engine = detect_search_engine(url);
    if engine == "Unknown" { return None; }
    let query_params = ["q", "query", "search", "p"];
    for param in &query_params {
        if let Some(start) = url.find(&format!("{}=", param)) {
            let after = &url[start + param.len() + 1..];
            let end = after.find('&').unwrap_or(after.len());
            let raw = &after[..end];
            let decoded = raw.replace('+', " ").replace("%20", " ");
            if !decoded.is_empty() { return Some(decoded); }
        }
    }
    None
}

fn extract_domain(url: &str) -> String {
    url.split('/').nth(2).unwrap_or("").to_string()
}

fn compute_sha256(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn parse_timestamp_from_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2})").unwrap();
    re.captures(text).map(|c| c[1].to_string())
}

fn estimate_duration_seconds(prev_ts: &str, curr_ts: &str) -> i64 {
    let prev = chrono::DateTime::parse_from_rfc3339(prev_ts).ok();
    let curr = chrono::DateTime::parse_from_rfc3339(curr_ts).ok();
    match (prev, curr) {
        (Some(p), Some(c)) => {
            let diff = (c - p).num_seconds();
            if diff >= 0 && diff < 3600 { diff } else { 0 }
        }
        _ => 0,
    }
}

fn format_duration(seconds: i64) -> String {
    if seconds < 60 { format!("{}s", seconds) }
    else if seconds < 3600 { format!("{}m {}s", seconds / 60, seconds % 60) }
    else { format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60) }
}

fn is_weekend(day: &str) -> bool {
    day == "Saturday" || day == "Sunday"
}

fn is_late_night(hour: u32) -> bool {
    hour >= 0 && hour < 6
}

fn day_name_from_index(idx: u32) -> String {
    match idx {
        0 => "Sunday".to_string(),
        1 => "Monday".to_string(),
        2 => "Tuesday".to_string(),
        3 => "Wednesday".to_string(),
        4 => "Thursday".to_string(),
        5 => "Friday".to_string(),
        6 => "Saturday".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn collect_history_entries(source_path: &str) -> Vec<HistoryEntry> {
    let path = PathBuf::from(source_path);
    let mut entries = Vec::new();
    if !path.exists() { return entries; }
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path_str = entry.path().to_string_lossy().to_lowercase();
        if name.contains("history") || path_str.contains("history") || name.contains("places") {
            if entry.path().extension().map(|e| e == "sqlite" || e == "db").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                let url_re = regex::Regex::new(r#"https?://[^\s"<>]+"#).unwrap();
                for cap in url_re.captures_iter(&text) {
                    let url = cap[0].to_string();
                    let domain = extract_domain(&url);
                    let ts = parse_timestamp_from_text(&text).unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                    entries.push(HistoryEntry {
                        id: Uuid::new_v4().to_string(),
                        url: url.clone(),
                        title: String::new(),
                        domain,
                        visit_count: 1,
                        last_visit: ts.clone(),
                        first_visit: ts.clone(),
                        visit_duration: "0s".into(),
                        category: categorize_url(&url),
                        source: entry.path().to_string_lossy().to_string(),
                        browser: "Chrome".into(),
                        sha256: String::new(),
                    });
                }
            }
        }
    }
    entries
}

fn collect_download_records(source_path: &str) -> Vec<DownloadRecord> {
    let path = PathBuf::from(source_path);
    let mut records = Vec::new();
    if !path.exists() { return records; }
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains("download") || name.contains("history") {
            if entry.path().extension().map(|e| e == "sqlite" || e == "db").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                let url_re = regex::Regex::new(r#"https?://[^\s"<>]+\.(zip|exe|pdf|docx?|xlsx?|pptx?|mp[34]|avi|mkv|tar\.gz|rar|7z|iso|dmg|apk|csv|json|xml|txt|png|jpe?g|gif|webp|svg|mpg|mov|flv|wmv|ogg|wav|flac|epub|mobi)"#).unwrap();
                for cap in url_re.captures_iter(&text) {
                    let url = cap[0].to_string();
                    let file_name = url.rsplit('/').next().unwrap_or("unknown").to_string();
                    let ts = parse_timestamp_from_text(&text).unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                    records.push(DownloadRecord {
                        id: Uuid::new_v4().to_string(),
                        url: url.clone(),
                        file_path: String::new(),
                        file_name,
                        file_size: 0,
                        mime_type: cap[1].to_string(),
                        timestamp: ts,
                        state: "completed".to_string(),
                    });
                }
            }
        }
    }
    records
}

#[tauri::command]
pub fn extract_browser_history(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/browser_history_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let entries: Vec<HistoryEntry> = Vec::new();
    let mut cookies: Vec<BrowserCookie> = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path_str = entry.path().to_string_lossy().to_lowercase();
        if name.contains("history") || path_str.contains("history") || name.contains("places") {
            if entry.path().extension().map(|e| e == "sqlite" || e == "db").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                for cap in regex::Regex::new(r#"(?i)name="([^"]+)"\s+value="([^"]+)""#).unwrap().captures_iter(&text) {
                    cookies.push(BrowserCookie {
                        id: Uuid::new_v4().to_string(), name: cap[1].to_string(),
                        value: cap[2].to_string(), domain: String::new(),
                        path: "/".into(), expiry: String::new(),
                        is_secure: false, is_http_only: false,
                        source: entry.path().to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    let mut domain_counts: HashMap<String, i64> = HashMap::new();
    for e in &entries { *domain_counts.entry(e.domain.clone()).or_insert(0) += e.visit_count; }
    let mut top_domains: Vec<(String, i64)> = domain_counts.into_iter().collect();
    top_domains.sort_by(|a, b| b.1.cmp(&a.1)); top_domains.truncate(20);
    let mut by_browser: HashMap<String, usize> = HashMap::new();
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for e in &entries { *by_browser.entry(e.browser.clone()).or_insert(0) += 1; let cat = format!("{:?}", e.category); *by_category.entry(cat).or_insert(0) += 1; }
    let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
    fs::write(format!("{}/history.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_entries": entries.len(), "total_cookies": cookies.len(), "top_domains": top_domains, "sha256": sha, "by_browser": by_browser, "by_category": by_category, "entries": entries.iter().take(1000).cloned().collect::<Vec<_>>(), "cookies": cookies.iter().take(100).cloned().collect::<Vec<_>>(), "summary": format!("{} entrees de navigation extraites depuis {} navigateurs", entries.len(), by_browser.len()), "note": "Manifeste : history.json"}))
}

#[tauri::command]
pub fn extract_safari_history(case_id: String, backup_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&backup_path);
    if !path.exists() { return Err(format!("Backup iOS introuvable : {}", backup_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/safari_history_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(4).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name == "history.db" || entry.path().to_string_lossy().to_lowercase().contains("history") {
            let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
            for cap in regex::Regex::new(r#"url="([^"]+)""#).unwrap().captures_iter(&text) {
                let url = cap[1].to_string();
                entries.push(HistoryEntry {
                    id: Uuid::new_v4().to_string(), url: url.clone(),
                    title: String::new(), domain: url.split("/").nth(2).unwrap_or("").to_string(),
                    visit_count: 1, last_visit: chrono::Utc::now().to_rfc3339(),
                    first_visit: chrono::Utc::now().to_rfc3339(),
                    visit_duration: "0s".into(), category: categorize_url(&url),
                    source: entry.path().to_string_lossy().to_string(),
                    browser: "Safari".into(), sha256: String::new(),
                });
            }
        }
    }
    for e in &mut entries {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}", e.url, e.domain, e.last_visit).as_bytes());
        e.sha256 = hex::encode(hasher.finalize());
    }
    let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
    fs::write(format!("{}/safari_history.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_entries": entries.len(), "sha256": sha, "entries": entries.iter().take(500).cloned().collect::<Vec<_>>(), "summary": format!("{} historiques Safari extraits", entries.len()), "note": "Manifeste : safari_history.json"}))
}

#[tauri::command]
pub fn extract_search_queries(source_path: String, output_dir: String) -> Result<Vec<SearchQuery>, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/search_queries_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut queries = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains("history") || name.contains("places") {
            if entry.path().extension().map(|e| e == "sqlite" || e == "db").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                let url_re = regex::Regex::new(r#"https?://[^\s"<>]+"#).unwrap();
                for cap in url_re.captures_iter(&text) {
                    let url = cap[0].to_string();
                    let engine = detect_search_engine(&url);
                    if engine == "Unknown" { continue; }
                    if let Some(query) = extract_query_from_url(&url) {
                        let timestamp = parse_timestamp_from_text(&text).unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                        queries.push(SearchQuery {
                            id: Uuid::new_v4().to_string(),
                            query,
                            engine,
                            timestamp,
                            url,
                        });
                    }
                }
            }
        }
    }
    let json = serde_json::to_string_pretty(&queries).unwrap_or_default();
    fs::write(format!("{}/search_queries.json", base), &json).ok();
    Ok(queries)
}

#[tauri::command]
pub fn extract_downloads(source_path: String, output_dir: String) -> Result<Vec<DownloadRecord>, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/downloads_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let records = collect_download_records(&source_path);
    let json = serde_json::to_string_pretty(&records).unwrap_or_default();
    fs::write(format!("{}/downloads.json", base), &json).ok();
    Ok(records)
}

#[tauri::command]
pub fn build_interest_profile(source_path: String, output_dir: String) -> Result<InterestProfile, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/interest_profile_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let entries = collect_history_entries(&source_path);
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut domain_counts: HashMap<String, usize> = HashMap::new();
    let mut search_topic_counts: HashMap<String, usize> = HashMap::new();
    let mut hour_counts: HashMap<u32, usize> = HashMap::new();
    let mut day_counts: HashMap<u32, usize> = HashMap::new();
    let mut late_night_visits = 0usize;
    let mut weekend_visits = 0usize;
    let mut total_visits = 0usize;
    let mut session_durations: Vec<i64> = Vec::new();
    let mut risk_indicators: Vec<String> = Vec::new();
    let mut prev_ts: Option<String> = None;
    for entry in &entries {
        let cat = format!("{:?}", entry.category);
        *category_counts.entry(cat).or_insert(0) += 1;
        *domain_counts.entry(entry.domain.clone()).or_insert(0) += 1;
        if entry.category == HistoryCategory::Search {
            if let Some(query) = extract_query_from_url(&entry.url) {
                let topic = query.split_whitespace().next().unwrap_or("").to_lowercase();
                if !topic.is_empty() { *search_topic_counts.entry(topic).or_insert(0) += 1; }
            }
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&entry.last_visit) {
            let hour = dt.hour();
            let weekday = dt.weekday().num_days_from_sunday();
            *hour_counts.entry(hour).or_insert(0) += 1;
            *day_counts.entry(weekday).or_insert(0) += 1;
            if is_late_night(hour) { late_night_visits += 1; }
            if is_weekend(&day_name_from_index(weekday)) { weekend_visits += 1; }
        }
        total_visits += 1;
        if let Some(ref prev) = prev_ts {
            let dur = estimate_duration_seconds(prev, &entry.last_visit);
            if dur > 0 { session_durations.push(dur); }
        }
        prev_ts = Some(entry.last_visit.clone());
    }
    let mut top_categories: Vec<(String, usize)> = category_counts.into_iter().collect();
    top_categories.sort_by(|a, b| b.1.cmp(&a.1));
    top_categories.truncate(10);
    let mut top_domains: Vec<(String, usize)> = domain_counts.into_iter().collect();
    top_domains.sort_by(|a, b| b.1.cmp(&a.1));
    top_domains.truncate(20);
    let mut search_topics: Vec<(String, usize)> = search_topic_counts.into_iter().collect();
    search_topics.sort_by(|a, b| b.1.cmp(&a.1));
    search_topics.truncate(15);
    let most_active_hour = hour_counts.iter().max_by_key(|(_, v)| *v).map(|(k, _)| *k).unwrap_or(0);
    let most_active_day_idx = day_counts.iter().max_by_key(|(_, v)| *v).map(|(k, _)| *k).unwrap_or(0);
    let most_active_day = day_name_from_index(most_active_day_idx);
    let avg_session_minutes = if !session_durations.is_empty() {
        let total_secs: i64 = session_durations.iter().sum();
        (total_secs as f64 / session_durations.len() as f64) / 60.0
    } else { 0.0 };
    let weekend_ratio = if total_visits > 0 { weekend_visits as f64 / total_visits as f64 } else { 0.0 };
    if late_night_visits > total_visits / 4 && total_visits > 10 {
        risk_indicators.push("Activite nocturne elevee detectee".to_string());
    }
    if top_categories.iter().any(|(c, _)| c == "DarkWeb") {
        risk_indicators.push("Acces a du contenu darkweb".to_string());
    }
    if top_categories.iter().any(|(c, _)| c == "Banking") && top_categories.iter().any(|(c, _)| c == "DarkWeb") {
        risk_indicators.push("Melange activite bancaire et darkweb".to_string());
    }
    if weekend_ratio > 0.5 && total_visits > 20 {
        risk_indicators.push("Navigation principalement le week-end".to_string());
    }
    let profile = InterestProfile {
        top_categories,
        top_domains,
        search_topics,
        browsing_patterns: BrowsingPattern {
            most_active_hour,
            most_active_day,
            avg_session_minutes,
            late_night_visits,
            weekend_ratio,
        },
        risk_indicators,
    };
    let json = serde_json::to_string_pretty(&profile).unwrap_or_default();
    fs::write(format!("{}/interest_profile.json", base), &json).ok();
    Ok(profile)
}

#[tauri::command]
pub fn export_history_csv(source_path: String, output_dir: String) -> Result<String, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/csv_export_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let entries = collect_history_entries(&source_path);
    let mut csv_content = String::from("id,url,title,domain,visit_count,last_visit,first_visit,visit_duration,category,source,browser,sha256\n");
    for entry in &entries {
        let cat = format!("{:?}", entry.category);
        csv_content.push_str(&format!("\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            entry.id, entry.url, entry.title, entry.domain, entry.visit_count,
            entry.last_visit, entry.first_visit, entry.visit_duration, cat,
            entry.source, entry.browser, entry.sha256));
    }
    let csv_path = format!("{}/history_export.csv", base);
    fs::write(&csv_path, &csv_content).map_err(|e| format!("Ecriture CSV: {}", e))?;
    Ok(csv_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_categorize() {
        assert!(matches!(categorize_url("https://www.youtube.com/watch?v=123"), HistoryCategory::Video));
        assert!(matches!(categorize_url("https://www.facebook.com/profile"), HistoryCategory::SocialMedia));
        assert!(matches!(categorize_url("https://www.amazon.com/dp/123"), HistoryCategory::Shopping));
        assert!(matches!(categorize_url("https://google.com/search?q=test"), HistoryCategory::Search));
    }
    #[test] fn test_detect_search_engine() {
        assert_eq!(detect_search_engine("https://www.google.com/search?q=test"), "Google");
        assert_eq!(detect_search_engine("https://www.bing.com/search?q=test"), "Bing");
        assert_eq!(detect_search_engine("https://duckduckgo.com/?q=test"), "DuckDuckGo");
        assert_eq!(detect_search_engine("https://search.yahoo.com/search?p=test"), "Yahoo");
    }
    #[test] fn test_extract_query_from_url() {
        assert_eq!(extract_query_from_url("https://www.google.com/search?q=rust+lang"), Some("rust lang".to_string()));
        assert_eq!(extract_query_from_url("https://www.bing.com/search?q=forensics"), Some("forensics".to_string()));
        assert_eq!(extract_query_from_url("https://example.com/page"), None);
    }
    #[test] fn test_extract_domain() {
        assert_eq!(extract_domain("https://www.google.com/search?q=test"), "www.google.com");
        assert_eq!(extract_domain("https://example.com/path"), "example.com");
    }
    #[test] fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3660), "1h 1m");
    }
    #[test] fn test_is_late_night() {
        assert!(is_late_night(0));
        assert!(is_late_night(5));
        assert!(!is_late_night(6));
        assert!(!is_late_night(23));
    }
    #[test] fn test_is_weekend() {
        assert!(is_weekend("Saturday"));
        assert!(is_weekend("Sunday"));
        assert!(!is_weekend("Monday"));
    }
}
