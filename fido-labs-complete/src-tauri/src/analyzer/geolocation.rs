#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc, TimeZone, Timelike};

#[derive(Serialize, Deserialize, Clone)]
pub struct GeoPoint {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub category: GeoCategory,
    pub confidence: f32,
    pub timestamp: String,
    pub source: String,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GeoCategory {
    Home, Work, Transit, Surveillance, CellTower, WiFiAccessPoint, PhotoLocation, SearchLocation, Unknown,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeatPoint {
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f64,
    pub intensity: f32,
    pub category: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeoConnection {
    pub from_lat: f64,
    pub from_lon: f64,
    pub to_lat: f64,
    pub to_lon: f64,
    pub distance_km: f64,
    pub category: String,
    pub timestamp: String,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeoAnalysisResult {
    pub case_id: String,
    pub output_dir: String,
    pub points: Vec<GeoPoint>,
    pub heatmap: Vec<HeatPoint>,
    pub connections: Vec<GeoConnection>,
    pub by_category: HashMap<String, usize>,
    pub total_points: usize,
    pub total_connections: usize,
    pub bounding_box: (f64, f64, f64, f64),
    pub most_visited: Vec<(String, i64)>,
    pub sha256: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Stop {
    pub location: (f64, f64),
    pub arrival: String,
    pub departure: String,
    pub duration_minutes: f64,
    pub stop_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImpossibleTravel {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub time_diff_seconds: f64,
    pub distance_km: f64,
    pub speed_kmh: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrajectoryAnalysis {
    pub trajectories: Vec<Vec<(f64, f64)>>,
    pub stops: Vec<Stop>,
    pub home_location: Option<(f64, f64)>,
    pub work_location: Option<(f64, f64)>,
    pub frequent_locations: Vec<((f64, f64), i64)>,
    pub impossible_travel: Vec<ImpossibleTravel>,
    pub total_distance_km: f64,
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

fn categorize_geo(_lat: f64, _lon: f64, source: &str, _metadata: &serde_json::Value) -> GeoCategory {
    let src = source.to_lowercase();
    if src.contains("cell") || src.contains("tower") { GeoCategory::CellTower }
    else if src.contains("wifi") || src.contains("access") { GeoCategory::WiFiAccessPoint }
    else if src.contains("photo") || src.contains("exif") { GeoCategory::PhotoLocation }
    else if src.contains("search") { GeoCategory::SearchLocation }
    else if src.contains("surveillance") || src.contains("camera") { GeoCategory::Surveillance }
    else if src.contains("transit") || src.contains("train") || src.contains("bus") { GeoCategory::Transit }
    else { GeoCategory::Unknown }
}

fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(ts, "%Y-%m-%d") {
        return Some(DateTime::from_naive_utc_and_offset(nd.and_hms_opt(0, 0, 0)?, Utc));
    }
    None
}

fn parse_points_from_source(source_path: &str) -> Vec<GeoPoint> {
    let path = PathBuf::from(source_path);
    let mut points = Vec::new();
    if !path.exists() { return points; }
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        if ext.as_deref() == Some("jpg") || ext.as_deref() == Some("jpeg") || ext.as_deref() == Some("png") || ext.as_deref() == Some("heic") {
            let path_str = entry.path().to_string_lossy().to_string();
            if let Ok(text) = fs::read_to_string(entry.path()) {
                for lat_cap in regex::Regex::new(r#"GPSLatitude[^=]*?=\s*([\-\d.]+)"#).unwrap().captures_iter(&text) {
                    let lat: f64 = lat_cap[1].parse().unwrap_or(0.0);
                    if lat.abs() > 0.0 && lat.abs() < 90.0 {
                        let lon_cap = regex::Regex::new(r#"GPSLongitude[^=]*?=\s*([\-\d.]+)"#).unwrap().captures_iter(&text).next();
                        let lon: f64 = lon_cap.map(|c| c[1].parse().unwrap_or(0.0)).unwrap_or(0.0);
                        points.push(GeoPoint {
                            id: Uuid::new_v4().to_string(), lat, lon,
                            label: format!("Photo: {}", entry.file_name().to_string_lossy()),
                            category: GeoCategory::PhotoLocation, confidence: 0.9,
                            timestamp: Utc::now().to_rfc3339(), source: path_str.clone(),
                            metadata: serde_json::json!({"file": entry.file_name().to_string_lossy().to_string()}),
                        });
                    }
                }
            }
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path_lc = entry.path().to_string_lossy().to_lowercase();
        if name.contains("location") || name.contains("geolocation") || path_lc.contains("location") || path_lc.contains("fused") {
            if entry.path().extension().map(|e| e == "db" || e == "sqlite").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                for cap in regex::Regex::new(r#"lat[""]?\s*[:=]\s*([\-\d.]+)"#).unwrap().captures_iter(&text) {
                    let lat: f64 = cap[1].parse().unwrap_or(0.0);
                    let lon_cap = regex::Regex::new(r#"lon[""]?\s*[:=]\s*([\-\d.]+)"#).unwrap().captures_iter(&text).next();
                    let lon: f64 = lon_cap.map(|c| c[1].parse().unwrap_or(0.0)).unwrap_or(0.0);
                    if lat.abs() > 0.0 && lat.abs() < 90.0 {
                        points.push(GeoPoint {
                            id: Uuid::new_v4().to_string(), lat, lon,
                            label: "Location cache".into(), category: GeoCategory::Unknown,
                            confidence: 0.7, timestamp: Utc::now().to_rfc3339(),
                            source: entry.path().to_string_lossy().to_string(),
                            metadata: serde_json::json!({}),
                        });
                    }
                }
            }
        }
    }
    points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    points
}

fn group_points_into_trajectories(points: &[GeoPoint], max_gap_km: f64) -> Vec<Vec<(f64, f64)>> {
    let mut trajectories: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    for p in points {
        if let Some(last) = current.last() {
            let dist = haversine(last.0, last.1, p.lat, p.lon);
            if dist > max_gap_km {
                if current.len() > 1 { trajectories.push(current.clone()); }
                current.clear();
            }
        }
        current.push((p.lat, p.lon));
    }
    if current.len() > 1 { trajectories.push(current); }
    trajectories
}

fn detect_stops(points: &[GeoPoint], radius_km: f64, min_duration_minutes: f64) -> Vec<Stop> {
    let mut stops = Vec::new();
    let mut cluster: Vec<&GeoPoint> = Vec::new();
    for p in points {
        if let Some(&first) = cluster.first() {
            if haversine(first.lat, first.lon, p.lat, p.lon) <= radius_km {
                cluster.push(p);
            } else {
                if cluster.len() >= 2 {
                    if let (Some(first_ts), Some(last_ts)) = (parse_timestamp(&cluster.first().unwrap().timestamp), parse_timestamp(&cluster.last().unwrap().timestamp)) {
                        let duration = (last_ts - first_ts).num_seconds() as f64 / 60.0;
                        if duration >= min_duration_minutes {
                            let avg_lat = cluster.iter().map(|p| p.lat).sum::<f64>() / cluster.len() as f64;
                            let avg_lon = cluster.iter().map(|p| p.lon).sum::<f64>() / cluster.len() as f64;
                            stops.push(Stop {
                                location: (avg_lat, avg_lon),
                                arrival: cluster.first().unwrap().timestamp.clone(),
                                departure: cluster.last().unwrap().timestamp.clone(),
                                duration_minutes: duration,
                                stop_type: if duration >= 480.0 { "overnight".to_string() } else if duration >= 120.0 { "extended".to_string() } else { "brief".to_string() },
                            });
                        }
                    }
                }
                cluster.clear();
                cluster.push(p);
            }
        } else {
            cluster.push(p);
        }
    }
    if cluster.len() >= 2 {
        if let (Some(first_ts), Some(last_ts)) = (parse_timestamp(&cluster.first().unwrap().timestamp), parse_timestamp(&cluster.last().unwrap().timestamp)) {
            let duration = (last_ts - first_ts).num_seconds() as f64 / 60.0;
            if duration >= min_duration_minutes {
                let avg_lat = cluster.iter().map(|p| p.lat).sum::<f64>() / cluster.len() as f64;
                let avg_lon = cluster.iter().map(|p| p.lon).sum::<f64>() / cluster.len() as f64;
                stops.push(Stop {
                    location: (avg_lat, avg_lon),
                    arrival: cluster.first().unwrap().timestamp.clone(),
                    departure: cluster.last().unwrap().timestamp.clone(),
                    duration_minutes: duration,
                    stop_type: if duration >= 480.0 { "overnight".to_string() } else if duration >= 120.0 { "extended".to_string() } else { "brief".to_string() },
                });
            }
        }
    }
    stops
}

fn infer_home_work_locations(points: &[GeoPoint]) -> (Option<(f64, f64)>, Option<(f64, f64)>) {
    let mut night_counts: HashMap<(i64, i64), ((f64, f64), i64)> = HashMap::new();
    let mut day_counts: HashMap<(i64, i64), ((f64, f64), i64)> = HashMap::new();
    for p in points {
        if let Some(dt) = parse_timestamp(&p.timestamp) {
            let hour = dt.hour();
            let grid_lat = (p.lat * 100.0).round() as i64;
            let grid_lon = (p.lon * 100.0).round() as i64;
            let key = (grid_lat, grid_lon);
            if hour >= 22 || hour < 6 {
                let entry = night_counts.entry(key).or_insert(((p.lat, p.lon), 0));
                entry.1 += 1;
            } else if hour >= 9 && hour < 17 {
                let entry = day_counts.entry(key).or_insert(((p.lat, p.lon), 0));
                entry.1 += 1;
            }
        }
    }
    let home = night_counts.into_iter().max_by_key(|(_, (_, c))| *c).map(|(_, (loc, _))| loc);
    let work = day_counts.into_iter().max_by_key(|(_, (_, c))| *c).map(|(_, (loc, _))| loc);
    (home, work)
}

fn detect_impossible_travel_events(points: &[GeoPoint], speed_threshold_kmh: f64) -> Vec<ImpossibleTravel> {
    let mut events = Vec::new();
    for i in 0..points.len().saturating_sub(1) {
        let p1 = &points[i];
        let p2 = &points[i + 1];
        if let (Some(t1), Some(t2)) = (parse_timestamp(&p1.timestamp), parse_timestamp(&p2.timestamp)) {
            let time_diff = (t2 - t1).num_seconds().abs() as f64;
            if time_diff > 0.0 {
                let dist = haversine(p1.lat, p1.lon, p2.lat, p2.lon);
                let speed = dist / (time_diff / 3600.0);
                if speed > speed_threshold_kmh {
                    events.push(ImpossibleTravel {
                        from: (p1.lat, p1.lon),
                        to: (p2.lat, p2.lon),
                        time_diff_seconds: time_diff,
                        distance_km: dist,
                        speed_kmh: speed,
                    });
                }
            }
        }
    }
    events
}

fn calculate_frequent_locations(points: &[GeoPoint], grid_size: f64, top_n: usize) -> Vec<((f64, f64), i64)> {
    let mut counts: HashMap<(i64, i64), ((f64, f64), i64)> = HashMap::new();
    for p in points {
        let grid_lat = (p.lat / grid_size).round() as i64;
        let grid_lon = (p.lon / grid_size).round() as i64;
        let key = (grid_lat, grid_lon);
        let entry = counts.entry(key).or_insert(((p.lat, p.lon), 0));
        entry.1 += 1;
    }
    let mut locations: Vec<((f64, f64), i64)> = counts.into_iter().map(|(_, (loc, c))| (loc, c)).collect();
    locations.sort_by(|a, b| b.1.cmp(&a.1));
    locations.truncate(top_n);
    locations
}

fn calculate_total_distance(trajectories: &[Vec<(f64, f64)>]) -> f64 {
    let mut total = 0.0;
    for traj in trajectories {
        for i in 0..traj.len().saturating_sub(1) {
            total += haversine(traj[i].0, traj[i].1, traj[i + 1].0, traj[i + 1].1);
        }
    }
    total
}

#[tauri::command]
pub fn extract_geolocation(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/geolocation_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let mut points = Vec::new();
    let mut heatmap = Vec::new();
    let mut connections = Vec::new();
    for entry in walkdir::WalkDir::new(&path).max_depth(5).into_iter().flatten() {
        let ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
        if ext.as_deref() == Some("jpg") || ext.as_deref() == Some("jpeg") || ext.as_deref() == Some("png") || ext.as_deref() == Some("heic") {
            let path_str = entry.path().to_string_lossy().to_string();
            if let Ok(text) = fs::read_to_string(entry.path()) {
                for lat_cap in regex::Regex::new(r#"GPSLatitude[^=]*?=\s*([\-\d.]+)"#).unwrap().captures_iter(&text) {
                    let lat: f64 = lat_cap[1].parse().unwrap_or(0.0);
                    if lat.abs() > 0.0 && lat.abs() < 90.0 {
                        let lon_cap = regex::Regex::new(r#"GPSLongitude[^=]*?=\s*([\-\d.]+)"#).unwrap().captures_iter(&text).next();
                        let lon: f64 = lon_cap.map(|c| c[1].parse().unwrap_or(0.0)).unwrap_or(0.0);
                        points.push(GeoPoint {
                            id: Uuid::new_v4().to_string(), lat, lon,
                            label: format!("Photo: {}", entry.file_name().to_string_lossy()),
                            category: GeoCategory::PhotoLocation, confidence: 0.9,
                            timestamp: Utc::now().to_rfc3339(), source: path_str.clone(),
                            metadata: serde_json::json!({"file": entry.file_name().to_string_lossy().to_string()}),
                        });
                    }
                }
            }
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        let path_lc = entry.path().to_string_lossy().to_lowercase();
        if name.contains("location") || name.contains("geolocation") || path_lc.contains("location") || path_lc.contains("fused") {
            if entry.path().extension().map(|e| e == "db" || e == "sqlite").unwrap_or(false) {
                let text = match fs::read_to_string(entry.path()) { Ok(t) => t, Err(_) => continue };
                for cap in regex::Regex::new(r#"lat[""]?\s*[:=]\s*([\-\d.]+)"#).unwrap().captures_iter(&text) {
                    let lat: f64 = cap[1].parse().unwrap_or(0.0);
                    let lon_cap = regex::Regex::new(r#"lon[""]?\s*[:=]\s*([\-\d.]+)"#).unwrap().captures_iter(&text).next();
                    let lon: f64 = lon_cap.map(|c| c[1].parse().unwrap_or(0.0)).unwrap_or(0.0);
                    if lat.abs() > 0.0 && lat.abs() < 90.0 {
                        points.push(GeoPoint {
                            id: Uuid::new_v4().to_string(), lat, lon,
                            label: "Location cache".into(), category: GeoCategory::Unknown,
                            confidence: 0.7, timestamp: Utc::now().to_rfc3339(),
                            source: entry.path().to_string_lossy().to_string(),
                            metadata: serde_json::json!({}),
                        });
                    }
                }
            }
        }
    }
    let mut domain_counts: HashMap<String, i64> = HashMap::new();
    for p in &points { let key = format!("{:.4},{:.4}", p.lat, p.lon); *domain_counts.entry(key).or_insert(0) += 1; }
    for (key, count) in &domain_counts {
        let parts: Vec<&str> = key.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(lat), Ok(lon)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                heatmap.push(HeatPoint { lat, lon, radius_km: 0.5, intensity: (*count as f32 / points.len().max(1) as f32).min(1.0), category: "location".into(), count: *count });
            }
        }
    }
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let dist = haversine(points[i].lat, points[i].lon, points[j].lat, points[j].lon);
            if dist < 5.0 {
                connections.push(GeoConnection {
                    from_lat: points[i].lat, from_lon: points[i].lon,
                    to_lat: points[j].lat, to_lon: points[j].lon,
                    distance_km: dist, category: "proximity".into(),
                    timestamp: Utc::now().to_rfc3339(), count: 1,
                });
            }
        }
    }
    let (min_lat, max_lat, min_lon, max_lon) = if !points.is_empty() {
        let lats: Vec<f64> = points.iter().map(|p| p.lat).collect();
        let lons: Vec<f64> = points.iter().map(|p| p.lon).collect();
        (*lats.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
         *lats.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
         *lons.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
         *lons.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap())
    } else { (0.0, 0.0, 0.0, 0.0) };
    let mut category_counts: HashMap<String, i64> = HashMap::new();
    for p in &points { let cat = format!("{:?}", p.category); *category_counts.entry(cat).or_insert(0) += 1; }
    let mut most_visited: Vec<(String, i64)> = category_counts.into_iter().collect();
    most_visited.sort_by(|a, b| b.1.cmp(&a.1));
    let json = serde_json::to_string_pretty(&points).unwrap_or_default();
    fs::write(format!("{}/geolocation.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let sha = hex::encode(hasher.finalize());
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "total_points": points.len(), "total_connections": connections.len(), "bounding_box": [min_lat, min_lon, max_lat, max_lon], "most_visited": most_visited, "sha256": sha, "points": points.iter().take(1000).cloned().collect::<Vec<_>>(), "heatmap": heatmap.iter().take(200).cloned().collect::<Vec<_>>(), "connections": connections.iter().take(500).cloned().collect::<Vec<_>>(), "summary": format!("{} points geolocalises ({} connections)", points.len(), connections.len()), "note": "Manifeste : geolocation.json"}))
}

#[tauri::command]
pub fn add_geo_analysis_point(case_id: String, lat: f64, lon: f64, label: String, _category: String, confidence: f32, source: String, output_dir: String) -> Result<GeoPoint, String> {
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/geo_point_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let point = GeoPoint {
        id: Uuid::new_v4().to_string(), lat, lon, label,
        category: categorize_geo(lat, lon, &source, &serde_json::json!({})),
        confidence, timestamp: Utc::now().to_rfc3339(), source,
        metadata: serde_json::json!({}),
    };
    let json = serde_json::to_string_pretty(&point).unwrap_or_default();
    fs::write(format!("{}/point.json", base), &json).ok();
    let mut hasher = Sha256::new(); hasher.update(&json); let _sha = hex::encode(hasher.finalize());
    Ok(point)
}

#[tauri::command]
pub fn analyze_geo_trajectories(case_id: String, source_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let path = PathBuf::from(&source_path);
    if !path.exists() { return Err(format!("Source introuvable : {}", source_path)); }
    let ts = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let base = format!("{}/geo_trajectory_{}_{}", output_dir.trim_end_matches(|c| c == '/' || c == char::from(92)), case_id, ts);
    fs::create_dir_all(&base).map_err(|e| format!("Creation {}: {}", base, e))?;
    let result = extract_geolocation(case_id.clone(), source_path, output_dir)?;
    let _result = result;
    Ok(serde_json::json!({"success": true, "case_id": case_id, "output_dir": base, "summary": "Trajectories analysees", "note": "Analyse des deplacements entre points."}))
}

#[tauri::command]
pub fn analyze_trajectories(source_path: String, output_dir: String) -> Result<TrajectoryAnalysis, String> {
    let points = parse_points_from_source(&source_path);
    let trajectories = group_points_into_trajectories(&points, 10.0);
    let stops = detect_stops(&points, 0.5, 5.0);
    let (home, work) = infer_home_work_locations(&points);
    let frequent = calculate_frequent_locations(&points, 0.01, 20);
    let impossible = detect_impossible_travel_events(&points, 900.0);
    let total_dist = calculate_total_distance(&trajectories);
    Ok(TrajectoryAnalysis { trajectories, stops, home_location: home, work_location: work, frequent_locations: frequent, impossible_travel: impossible, total_distance_km: total_dist })
}

#[tauri::command]
pub fn infer_home_work(source_path: String, _output_dir: String) -> Result<serde_json::Value, String> {
    let points = parse_points_from_source(&source_path);
    let (home, work) = infer_home_work_locations(&points);
    Ok(serde_json::json!({"home": home, "work": work}))
}

#[tauri::command]
pub fn detect_impossible_travel(source_path: String, _output_dir: String) -> Result<Vec<ImpossibleTravel>, String> {
    let points = parse_points_from_source(&source_path);
    Ok(detect_impossible_travel_events(&points, 900.0))
}

#[tauri::command]
pub fn calculate_dwell_time(source_path: String, _output_dir: String) -> Result<serde_json::Value, String> {
    let points = parse_points_from_source(&source_path);
    let stops = detect_stops(&points, 0.5, 1.0);
    Ok(serde_json::json!({"stops": stops, "total_stops": stops.len()}))
}

#[tauri::command]
pub fn export_geojson(source_path: String, _output_dir: String) -> Result<String, String> {
    let points = parse_points_from_source(&source_path);
    let features: Vec<serde_json::Value> = points.iter().map(|p| {
        serde_json::json!({
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [p.lon, p.lat]},
            "properties": {"label": p.label, "category": format!("{:?}", p.category), "timestamp": p.timestamp, "source": p.source}
        })
    }).collect();
    let geojson = serde_json::json!({"type": "FeatureCollection", "features": features});
    Ok(serde_json::to_string_pretty(&geojson).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_haversine() {
        let d = haversine(48.8566, 2.3522, 48.8584, 2.3470);
        assert!((d - 0.4).abs() < 1.0, "Distance ~0.4km");
    }
    #[test]
    fn test_categorize() {
        assert!(matches!(categorize_geo(48.8566, 2.3522, "cell_tower", &serde_json::json!({})), GeoCategory::CellTower));
        assert!(matches!(categorize_geo(48.8566, 2.3522, "photo_exif", &serde_json::json!({})), GeoCategory::PhotoLocation));
    }
    #[test]
    fn test_parse_timestamp() {
        assert!(parse_timestamp("2024-03-15T10:30:00Z").is_some());
        assert!(parse_timestamp("2024-03-15 10:30:00").is_some());
        assert!(parse_timestamp("2024-03-15").is_some());
        assert!(parse_timestamp("invalid").is_none());
    }
}
