// LINK GRAPH v2 - Graphe de relations entre numéros, personnes, comptes
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,  // PHONE, EMAIL, ACCOUNT, PERSON, LOCATION
    pub weight: usize,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relation: String,   // CALLED, MESSAGED, TRANSFERRED, KNOWN_CONTACT
    pub count: usize,
    pub last_seen: String,
}

#[tauri::command]
pub fn build_link_graph(contacts: Vec<serde_json::Value>, messages: Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: HashMap<String, GraphEdge> = HashMap::new();

    // Ajouter les contacts comme noeuds
    for contact in &contacts {
        if let Some(num) = contact["number"].as_str() {
            let id = normalize_number(num);
            nodes.entry(id.clone()).or_insert(GraphNode {
                id: id.clone(),
                label: contact["name"].as_str().unwrap_or(num).to_string(),
                node_type: "PHONE".into(),
                weight: 1,
                metadata: contact.clone(),
            });
        }
    }

    // Construire les arêtes depuis les messages
    for msg in &messages {
        let from = msg["from_jid"].as_str()
            .or(msg["from_num"].as_str())
            .unwrap_or("?");
        let to = msg["to_jid"].as_str()
            .or(msg["to_num"].as_str())
            .unwrap_or("?");
        let ts = msg["timestamp"].as_str().unwrap_or("").to_string();
        let relation = if msg["type_tx"].is_null() { "MESSAGED" } else { "TRANSFERRED" };

        if from == "?" || to == "?" { continue; }

        let fid = normalize_number(from);
        let tid = normalize_number(to);

        // Noeuds
        nodes.entry(fid.clone()).or_insert(GraphNode {
            id: fid.clone(), label: fid.clone(),
            node_type: "PHONE".into(), weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        nodes.entry(tid.clone()).or_insert(GraphNode {
            id: tid.clone(), label: tid.clone(),
            node_type: "PHONE".into(), weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        // Arête
        let key = format!("{}->{}", fid, tid);
        let e = edges.entry(key).or_insert(GraphEdge {
            source: fid, target: tid,
            relation: relation.into(),
            count: 0, last_seen: ts.clone(),
        });
        e.count += 1;
        if ts > e.last_seen { e.last_seen = ts; }
    }

    let nodes_list: Vec<GraphNode> = nodes.into_values().collect();
    let edges_list: Vec<GraphEdge> = edges.into_values().collect();

    Ok(serde_json::json!({
        "nodes": nodes_list,
        "edges": edges_list,
    }))
}

fn normalize_number(n: &str) -> String {
    let digits: String = n.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    if digits.starts_with('+') { digits } else { format!("+{}", digits) }
}
