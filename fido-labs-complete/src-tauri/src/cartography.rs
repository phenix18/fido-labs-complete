#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct GeoPoint {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub category: String,
    pub confidence: f32,
    pub timestamp: String,
    pub source: String,
    pub metadata: serde_json::Value,
    pub entity_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CartographicMap {
    pub id: String,
    pub name: String,
    pub case_id: String,
    pub points: Vec<GeoPoint>,
    pub heatmap_data: Vec<HeatPoint>,
    pub connections: Vec<MapConnection>,
    pub trajectories: Vec<Trajectory>,
    pub geofences: Vec<Geofence>,
    pub layers: Vec<MapLayer>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeatPoint {
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f64,
    pub intensity: f32,
    pub category: String,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MapConnection {
    pub from_point_id: String,
    pub to_point_id: String,
    pub connection_type: String,
    pub count: usize,
    pub distance_km: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trajectory {
    pub id: String,
    pub entity_id: String,
    pub points: Vec<GeoPoint>,
    pub start_time: String,
    pub end_time: String,
    pub total_distance_km: f64,
    pub purpose: String,
    pub stops: Vec<GeoPoint>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Geofence {
    pub id: String,
    pub name: String,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_m: f64,
    pub category: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MapLayer {
    pub id: String,
    pub name: String,
    pub layer_type: String,
    pub visible: bool,
    pub data: serde_json::Value,
}

static MAPS: Mutex<Option<HashMap<String, CartographicMap>>> = Mutex::new(None);

fn maps_store() -> std::sync::MutexGuard<'static, Option<HashMap<String, CartographicMap>>> {
    let mut guard = MAPS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

fn parse_timestamp(ts: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .ok()
}

fn grid_cell(lat: f64, lon: f64, cell_size: f64) -> (i64, i64) {
    let lat_cell = (lat / cell_size).floor() as i64;
    let lon_cell = (lon / cell_size).floor() as i64;
    (lat_cell, lon_cell)
}

fn cell_center(cell: (i64, i64), cell_size: f64) -> (f64, f64) {
    let lat = (cell.0 as f64 + 0.5) * cell_size;
    let lon = (cell.1 as f64 + 0.5) * cell_size;
    (lat, lon)
}

fn detect_stops(points: &[GeoPoint], stop_radius_m: f64, stop_duration_secs: i64) -> Vec<GeoPoint> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut stops = Vec::new();
    let mut cluster_start = 0;
    for i in 1..points.len() {
        let dist = haversine(
            points[cluster_start].lat,
            points[cluster_start].lon,
            points[i].lat,
            points[i].lon,
        );
        if dist * 1000.0 > stop_radius_m {
            if cluster_start < i - 1 {
                let duration = match (
                    parse_timestamp(&points[cluster_start].timestamp),
                    parse_timestamp(&points[i - 1].timestamp),
                ) {
                    (Some(start), Some(end)) => (end - start).num_seconds(),
                    _ => 0,
                };
                if duration >= stop_duration_secs {
                    let avg_lat: f64 = points[cluster_start..i].iter().map(|p| p.lat).sum::<f64>()
                        / (i - cluster_start) as f64;
                    let avg_lon: f64 = points[cluster_start..i].iter().map(|p| p.lon).sum::<f64>()
                        / (i - cluster_start) as f64;
                    stops.push(GeoPoint {
                        id: Uuid::new_v4().to_string(),
                        lat: avg_lat,
                        lon: avg_lon,
                        label: format!("Stop: {}", points[cluster_start].label),
                        category: "stop".into(),
                        confidence: 0.8,
                        timestamp: points[cluster_start].timestamp.clone(),
                        source: "trajectory_analysis".into(),
                        metadata: serde_json::json!({
                            "duration_secs": duration,
                            "point_count": i - cluster_start,
                        }),
                        entity_id: points[cluster_start].entity_id.clone(),
                    });
                }
            }
            cluster_start = i;
        }
    }
    stops
}

fn infer_purpose(trajectory: &Trajectory) -> String {
    if trajectory.stops.is_empty() {
        return "transit".into();
    }
    let stop_count = trajectory.stops.len();
    let total_duration = match (
        parse_timestamp(&trajectory.start_time),
        parse_timestamp(&trajectory.end_time),
    ) {
        (Some(start), Some(end)) => (end - start).num_seconds(),
        _ => 0,
    };
    if stop_count >= 3 && total_duration > 3600 {
        "patrol".into()
    } else if stop_count >= 2 {
        "delivery".into()
    } else {
        "visit".into()
    }
}

#[tauri::command]
pub fn create_map(name: String, case_id: String) -> Result<CartographicMap, String> {
    let map = CartographicMap {
        id: Uuid::new_v4().to_string(),
        name,
        case_id,
        points: Vec::new(),
        heatmap_data: Vec::new(),
        connections: Vec::new(),
        trajectories: Vec::new(),
        geofences: Vec::new(),
        layers: vec![
            MapLayer {
                id: Uuid::new_v4().to_string(),
                name: "Base".into(),
                layer_type: "base".into(),
                visible: true,
                data: serde_json::json!({}),
            },
            MapLayer {
                id: Uuid::new_v4().to_string(),
                name: "Heatmap".into(),
                layer_type: "heatmap".into(),
                visible: true,
                data: serde_json::json!({}),
            },
            MapLayer {
                id: Uuid::new_v4().to_string(),
                name: "Connections".into(),
                layer_type: "connections".into(),
                visible: true,
                data: serde_json::json!({}),
            },
            MapLayer {
                id: Uuid::new_v4().to_string(),
                name: "Trajectories".into(),
                layer_type: "trajectories".into(),
                visible: true,
                data: serde_json::json!({}),
            },
            MapLayer {
                id: Uuid::new_v4().to_string(),
                name: "Geofences".into(),
                layer_type: "geofences".into(),
                visible: true,
                data: serde_json::json!({}),
            },
        ],
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    maps_store()
        .as_mut()
        .unwrap()
        .insert(map.id.clone(), map.clone());
    Ok(map)
}

#[tauri::command]
pub fn add_geo_point(
    map_id: String,
    lat: f64,
    lon: f64,
    label: String,
    category: String,
    confidence: f32,
    source: String,
    metadata: serde_json::Value,
) -> Result<GeoPoint, String> {
    let entity_id = metadata
        .get("entity_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let point = GeoPoint {
        id: Uuid::new_v4().to_string(),
        lat,
        lon,
        label,
        category,
        confidence,
        timestamp: chrono::Utc::now().to_rfc3339(),
        source,
        metadata,
        entity_id,
    };
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    map.points.push(point.clone());
    Ok(point)
}

#[tauri::command]
pub fn add_geo_points_batch(
    map_id: String,
    points: Vec<serde_json::Value>,
) -> Result<usize, String> {
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let mut added = 0;
    for pt in points {
        let lat = pt.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lon = pt.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if lat.abs() > 90.0 || lon.abs() > 180.0 {
            continue;
        }
        let entity_id = pt
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let point = GeoPoint {
            id: Uuid::new_v4().to_string(),
            lat,
            lon,
            label: pt
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            category: pt
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            confidence: pt.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
            timestamp: pt
                .get("timestamp")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            source: pt
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("batch")
                .to_string(),
            metadata: pt.get("metadata").cloned().unwrap_or(serde_json::json!({})),
            entity_id,
        };
        map.points.push(point);
        added += 1;
    }
    Ok(added)
}

#[tauri::command]
pub fn generate_heatmap(map_id: String) -> Result<Vec<HeatPoint>, String> {
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let cell_size = 0.01;
    let mut grid: HashMap<(i64, i64), Vec<&GeoPoint>> = HashMap::new();
    for point in &map.points {
        let cell = grid_cell(point.lat, point.lon, cell_size);
        grid.entry(cell).or_default().push(point);
    }
    let total = map.points.len().max(1);
    let mut heatmap: Vec<HeatPoint> = grid
        .iter()
        .map(|(cell, pts)| {
            let (lat, lon) = cell_center(*cell, cell_size);
            let avg_confidence: f32 = pts.iter().map(|p| p.confidence).sum::<f32>() / pts.len() as f32;
            let intensity = (pts.len() as f32 / total as f32 * avg_confidence).min(1.0);
            let dominant_category = pts
                .iter()
                .fold(HashMap::<String, usize>::new(), |mut acc, p| {
                    *acc.entry(p.category.clone()).or_insert(0) += 1;
                    acc
                })
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(cat, _)| cat)
                .unwrap_or_else(|| "unknown".into());
            HeatPoint {
                lat,
                lon,
                radius_km: cell_size * 111.0,
                intensity,
                category: dominant_category,
                count: pts.len(),
            }
        })
        .collect();
    heatmap.sort_by(|a, b| b.intensity.partial_cmp(&a.intensity).unwrap());
    map.heatmap_data = heatmap.clone();
    Ok(heatmap)
}

#[tauri::command]
pub fn build_connections(map_id: String) -> Result<Vec<MapConnection>, String> {
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let max_distance_km = 5.0;
    let max_time_secs = 3600.0;
    let mut connections: Vec<MapConnection> = Vec::new();
    for i in 0..map.points.len() {
        for j in (i + 1)..map.points.len() {
            let dist = haversine(
                map.points[i].lat,
                map.points[i].lon,
                map.points[j].lat,
                map.points[j].lon,
            );
            if dist > max_distance_km {
                continue;
            }
            let time_diff = match (
                parse_timestamp(&map.points[i].timestamp),
                parse_timestamp(&map.points[j].timestamp),
            ) {
                (Some(t1), Some(t2)) => (t2 - t1).num_seconds().abs() as f64,
                _ => f64::MAX,
            };
            if time_diff > max_time_secs {
                continue;
            }
            let conn_type = if map.points[i].category == map.points[j].category {
                "same_category"
            } else if dist < 0.5 {
                "proximity"
            } else {
                "temporal"
            };
            connections.push(MapConnection {
                from_point_id: map.points[i].id.clone(),
                to_point_id: map.points[j].id.clone(),
                connection_type: conn_type.into(),
                count: 1,
                distance_km: dist,
            });
        }
    }
    connections.sort_by(|a, b| a.distance_km.partial_cmp(&b.distance_km).unwrap());
    map.connections = connections.clone();
    Ok(connections)
}

#[tauri::command]
pub fn reconstruct_trajectories(map_id: String) -> Result<Vec<Trajectory>, String> {
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let mut entity_points: HashMap<String, Vec<GeoPoint>> = HashMap::new();
    for point in &map.points {
        entity_points
            .entry(point.entity_id.clone())
            .or_default()
            .push(point.clone());
    }
    let mut trajectories = Vec::new();
    for (entity_id, mut pts) in entity_points {
        if pts.len() < 2 {
            continue;
        }
        pts.sort_by(|a, b| {
            let ta = parse_timestamp(&a.timestamp);
            let tb = parse_timestamp(&b.timestamp);
            ta.cmp(&tb)
        });
        let total_distance: f64 = pts
            .windows(2)
            .map(|w| haversine(w[0].lat, w[0].lon, w[1].lat, w[1].lon))
            .sum();
        let stops = detect_stops(&pts, 100.0, 300);
        let start_time = pts.first().map(|p| p.timestamp.clone()).unwrap_or_default();
        let end_time = pts.last().map(|p| p.timestamp.clone()).unwrap_or_default();
        let mut trajectory = Trajectory {
            id: Uuid::new_v4().to_string(),
            entity_id,
            points: pts,
            start_time,
            end_time,
            total_distance_km: total_distance,
            purpose: String::new(),
            stops,
        };
        trajectory.purpose = infer_purpose(&trajectory);
        trajectories.push(trajectory);
    }
    trajectories.sort_by(|a, b| b.total_distance_km.partial_cmp(&a.total_distance_km).unwrap());
    map.trajectories = trajectories.clone();
    Ok(trajectories)
}

#[tauri::command]
pub fn add_geofence(
    map_id: String,
    name: String,
    center_lat: f64,
    center_lon: f64,
    radius_m: f64,
    category: String,
) -> Result<Geofence, String> {
    let geofence = Geofence {
        id: Uuid::new_v4().to_string(),
        name,
        center_lat,
        center_lon,
        radius_m,
        category,
    };
    let mut store = maps_store();
    let map = store
        .as_mut()
        .unwrap()
        .get_mut(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    map.geofences.push(geofence.clone());
    Ok(geofence)
}

#[tauri::command]
pub fn detect_geofence_breaches(map_id: String) -> Result<Vec<serde_json::Value>, String> {
    let store = maps_store();
    let map = store
        .as_ref()
        .unwrap()
        .get(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let mut breaches = Vec::new();
    for point in &map.points {
        for fence in &map.geofences {
            let dist = haversine(point.lat, point.lon, fence.center_lat, fence.center_lon) * 1000.0;
            if dist <= fence.radius_m {
                breaches.push(serde_json::json!({
                    "point_id": point.id,
                    "point_label": point.label,
                    "point_category": point.category,
                    "geofence_id": fence.id,
                    "geofence_name": fence.name,
                    "geofence_category": fence.category,
                    "distance_m": dist,
                    "timestamp": point.timestamp,
                    "event_type": "entry",
                    "lat": point.lat,
                    "lon": point.lon,
                }));
            }
        }
    }
    breaches.sort_by(|a, b| {
        let ta = a.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        let tb = b.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        ta.cmp(tb)
    });
    Ok(breaches)
}

#[tauri::command]
pub fn get_map_data(map_id: String) -> Result<serde_json::Value, String> {
    let store = maps_store();
    let map = store
        .as_ref()
        .unwrap()
        .get(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    Ok(serde_json::to_value(map).unwrap_or(serde_json::json!({})))
}

#[tauri::command]
pub fn export_map_geojson(map_id: String) -> Result<String, String> {
    let store = maps_store();
    let map = store
        .as_ref()
        .unwrap()
        .get(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let features: Vec<serde_json::Value> = map
        .points
        .iter()
        .map(|p| {
            serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [p.lon, p.lat],
                },
                "properties": {
                    "id": p.id,
                    "label": p.label,
                    "category": p.category,
                    "confidence": p.confidence,
                    "timestamp": p.timestamp,
                    "source": p.source,
                    "entity_id": p.entity_id,
                    "metadata": p.metadata,
                },
            })
        })
        .collect();
    let connection_features: Vec<serde_json::Value> = map
        .connections
        .iter()
        .filter_map(|c| {
            let from = map.points.iter().find(|p| p.id == c.from_point_id)?;
            let to = map.points.iter().find(|p| p.id == c.to_point_id)?;
            Some(serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[from.lon, from.lat], [to.lon, to.lat]],
                },
                "properties": {
                    "connection_type": c.connection_type,
                    "distance_km": c.distance_km,
                    "count": c.count,
                },
            }))
        })
        .collect();
    let geofence_features: Vec<serde_json::Value> = map
        .geofences
        .iter()
        .map(|g| {
            let mut coords = Vec::new();
            let segments = 32;
            for i in 0..=segments {
                let angle = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
                let d_lat = (g.radius_m / 111000.0) * angle.cos();
                let d_lon = (g.radius_m / (111000.0 * g.center_lat.to_radians().cos())) * angle.sin();
                coords.push([g.center_lon + d_lon, g.center_lat + d_lat]);
            }
            serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [coords],
                },
                "properties": {
                    "id": g.id,
                    "name": g.name,
                    "category": g.category,
                    "radius_m": g.radius_m,
                },
            })
        })
        .collect();
    let trajectory_features: Vec<serde_json::Value> = map
        .trajectories
        .iter()
        .map(|t| {
            let coords: Vec<Vec<f64>> = t
                .points
                .iter()
                .map(|p| vec![p.lon, p.lat])
                .collect();
            serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": coords,
                },
                "properties": {
                    "id": t.id,
                    "entity_id": t.entity_id,
                    "purpose": t.purpose,
                    "total_distance_km": t.total_distance_km,
                    "start_time": t.start_time,
                    "end_time": t.end_time,
                },
            })
        })
        .collect();
    let mut all_features = features;
    all_features.extend(connection_features);
    all_features.extend(geofence_features);
    all_features.extend(trajectory_features);
    let geojson = serde_json::json!({
        "type": "FeatureCollection",
        "features": all_features,
    });
    serde_json::to_string_pretty(&geojson).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_point_details(map_id: String, point_id: String) -> Result<serde_json::Value, String> {
    let store = maps_store();
    let map = store
        .as_ref()
        .unwrap()
        .get(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    let point = map
        .points
        .iter()
        .find(|p| p.id == point_id)
        .ok_or_else(|| format!("Point not found: {}", point_id))?;
    let related_connections: Vec<MapConnection> = map
        .connections
        .iter()
        .filter(|c| c.from_point_id == point_id || c.to_point_id == point_id)
        .cloned()
        .collect();
    let related_geofences: Vec<Geofence> = map
        .geofences
        .iter()
        .filter(|g| {
            let dist = haversine(point.lat, point.lon, g.center_lat, g.center_lon) * 1000.0;
            dist <= g.radius_m
        })
        .cloned()
        .collect();
    let related_trajectories: Vec<Trajectory> = map
        .trajectories
        .iter()
        .filter(|t| t.points.iter().any(|p| p.id == point_id))
        .cloned()
        .collect();
    Ok(serde_json::json!({
        "point": point,
        "connections": related_connections,
        "geofences": related_geofences,
        "trajectories": related_trajectories,
    }))
}

#[tauri::command]
pub fn delete_map(map_id: String) -> Result<String, String> {
    let mut store = maps_store();
    store
        .as_mut()
        .unwrap()
        .remove(&map_id)
        .ok_or_else(|| format!("Map not found: {}", map_id))?;
    Ok(format!("Map {} deleted", map_id))
}

#[tauri::command]
pub fn list_maps(case_id: String) -> Result<Vec<CartographicMap>, String> {
    let store = maps_store();
    let maps: Vec<CartographicMap> = store
        .as_ref()
        .unwrap()
        .values()
        .filter(|m| m.case_id == case_id)
        .cloned()
        .collect();
    Ok(maps)
}
