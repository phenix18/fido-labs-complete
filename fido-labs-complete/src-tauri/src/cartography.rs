// CARTOGRAPHY MODULE - Cartographie des données forensiques
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GeoPoint {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub category: String, // "extraction_point", "suspect_location", "victim_location", "cell_tower"
    pub confidence: f32,  // 0.0 - 1.0
    pub timestamp: String,
    pub source: String,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CartographicMap {
    pub id: String,
    pub name: String,
    pub case_id: String,
    pub points: Vec<GeoPoint>,
    pub heatmap_data: Vec<HeatPoint>,
    pub connections: Vec<MapConnection>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HeatPoint {
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f64,
    pub intensity: f32,
    pub category: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MapConnection {
    pub from_point_id: String,
    pub to_point_id: String,
    pub connection_type: String, // "called", "sms", "nearby", "data_transfer"
    pub count: usize,
}

#[tauri::command]
pub fn create_map(name: String, case_id: String) -> Result<CartographicMap, String> {
    use uuid::Uuid;
    Ok(CartographicMap {
        id: Uuid::new_v4().to_string(),
        name, case_id,
        points: Vec::new(),
        heatmap_data: Vec::new(),
        connections: Vec::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
pub fn add_geo_point(
    map_id: String, lat: f64, lon: f64, label: String, category: String,
    confidence: f32, source: String, metadata: serde_json::Value,
) -> Result<GeoPoint, String> {
    use uuid::Uuid;
    Ok(GeoPoint {
        id: Uuid::new_v4().to_string(),
        lat, lon, label, category, confidence,
        timestamp: chrono::Utc::now().to_rfc3339(),
        source, metadata,
    })
}

#[tauri::command]
pub fn get_map_summary(map_id: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "map_id": map_id,
        "analysis": "Cartographie prête — visualisation 2D/3D disponible",
        "data": "Points géolocalisés chargés"
    }))
}
