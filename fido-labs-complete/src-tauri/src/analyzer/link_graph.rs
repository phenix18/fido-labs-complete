use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum NodeType {
    Phone,
    Email,
    Account,
    Person,
    Location,
    Device,
    Financial,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Phone => write!(f, "Phone"),
            NodeType::Email => write!(f, "Email"),
            NodeType::Account => write!(f, "Account"),
            NodeType::Person => write!(f, "Person"),
            NodeType::Location => write!(f, "Location"),
            NodeType::Device => write!(f, "Device"),
            NodeType::Financial => write!(f, "Financial"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum EdgeType {
    Called,
    Messaged,
    Transferred,
    Emailed,
    ContactOf,
    LocatedAt,
    Nearby,
    KnownContact,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeType::Called => write!(f, "Called"),
            EdgeType::Messaged => write!(f, "Messaged"),
            EdgeType::Transferred => write!(f, "Transferred"),
            EdgeType::Emailed => write!(f, "Emailed"),
            EdgeType::ContactOf => write!(f, "ContactOf"),
            EdgeType::LocatedAt => write!(f, "LocatedAt"),
            EdgeType::Nearby => write!(f, "Nearby"),
            EdgeType::KnownContact => write!(f, "KnownContact"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub weight: usize,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relation: EdgeType,
    pub count: usize,
    pub last_seen: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphAnalysis {
    pub degree_centrality: HashMap<String, f64>,
    pub betweenness: HashMap<String, f64>,
    pub communities: HashMap<String, usize>,
    pub risk_scores: HashMap<String, f64>,
    pub key_bridges: Vec<String>,
    pub isolated_nodes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn normalize_number(n: &str) -> String {
    let digits: String = n.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    if digits.starts_with('+') { digits } else { format!("+{}", digits) }
}

fn normalize_email(e: &str) -> String {
    e.trim().to_lowercase()
}

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn json_string(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn build_graph_internal(
    contacts: &[serde_json::Value],
    messages: &[serde_json::Value],
    call_history: &[serde_json::Value],
    emails: &[serde_json::Value],
    geo_points: &[serde_json::Value],
    money_transactions: &[serde_json::Value],
) -> GraphData {
    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: HashMap<String, GraphEdge> = HashMap::new();

    for contact in contacts {
        if let Some(num) = contact["number"].as_str() {
            let id = normalize_number(num);
            let name = contact["name"].as_str().unwrap_or(num).to_string();
            nodes.entry(id.clone()).or_insert(GraphNode {
                id: id.clone(),
                label: name,
                node_type: NodeType::Phone,
                weight: 1,
                metadata: contact.clone(),
            });
        }
        if let Some(email) = contact["email"].as_str() {
            let id = normalize_email(email);
            if id.is_empty() { continue; }
            let name = contact["name"].as_str().unwrap_or(email).to_string();
            nodes.entry(id.clone()).or_insert(GraphNode {
                id: id.clone(),
                label: name,
                node_type: NodeType::Email,
                weight: 1,
                metadata: contact.clone(),
            });
        }
        if let Some(name) = contact["name"].as_str() {
            let id = format!("person:{}", name.to_lowercase().replace(' ', "_"));
            nodes.entry(id.clone()).or_insert(GraphNode {
                id: id.clone(),
                label: name.to_string(),
                node_type: NodeType::Person,
                weight: 1,
                metadata: contact.clone(),
            });
        }
    }

    for msg in messages {
        let from = json_string(msg, "from_jid");
        let to = json_string(msg, "to_jid");
        let from = if from.is_empty() { json_string(msg, "from_num") } else { from };
        let to = if to.is_empty() { json_string(msg, "to_num") } else { to };
        let ts = json_string(msg, "timestamp");
        let relation = if msg["type_tx"].is_null() { EdgeType::Messaged } else { EdgeType::Transferred };

        if from.is_empty() || to.is_empty() || from == "?" || to == "?" { continue; }

        let fid = normalize_number(&from);
        let tid = normalize_number(&to);

        nodes.entry(fid.clone()).or_insert_with(|| GraphNode {
            id: fid.clone(), label: fid.clone(),
            node_type: NodeType::Phone, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        nodes.entry(tid.clone()).or_insert_with(|| GraphNode {
            id: tid.clone(), label: tid.clone(),
            node_type: NodeType::Phone, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        let key = format!("{}->{}:{:?}", fid, tid, relation);
        let e = edges.entry(key).or_insert(GraphEdge {
            source: fid.clone(), target: tid.clone(),
            relation: relation.clone(),
            count: 0, last_seen: ts.clone(),
        });
        e.count += 1;
        if ts > e.last_seen { e.last_seen = ts.clone(); }
    }

    for call in call_history {
        let from = if json_string(call, "from_num").is_empty() { json_string(call, "caller") } else { json_string(call, "from_num") };
        let to = if json_string(call, "to_num").is_empty() { json_string(call, "callee") } else { json_string(call, "to_num") };
        let ts = if json_string(call, "timestamp").is_empty() { json_string(call, "date") } else { json_string(call, "timestamp") };

        if from.is_empty() || to.is_empty() || from == "?" || to == "?" { continue; }

        let fid = normalize_number(&from);
        let tid = normalize_number(&to);

        nodes.entry(fid.clone()).or_insert_with(|| GraphNode {
            id: fid.clone(), label: fid.clone(),
            node_type: NodeType::Phone, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        nodes.entry(tid.clone()).or_insert_with(|| GraphNode {
            id: tid.clone(), label: tid.clone(),
            node_type: NodeType::Phone, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        let key = format!("{}->{}:{:?}", fid, tid, EdgeType::Called);
        let e = edges.entry(key).or_insert(GraphEdge {
            source: fid.clone(), target: tid.clone(),
            relation: EdgeType::Called,
            count: 0, last_seen: ts.clone(),
        });
        e.count += 1;
        if ts > e.last_seen { e.last_seen = ts.clone(); }
    }

    for email in emails {
        let from = json_string(email, "from");
        let to = json_string(email, "to");
        let ts = if json_string(email, "timestamp").is_empty() { json_string(email, "date") } else { json_string(email, "timestamp") };

        if from.is_empty() || to.is_empty() || from == "?" || to == "?" { continue; }

        let fid = normalize_email(&from);
        let tid = normalize_email(&to);

        nodes.entry(fid.clone()).or_insert_with(|| GraphNode {
            id: fid.clone(), label: fid.clone(),
            node_type: NodeType::Email, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        nodes.entry(tid.clone()).or_insert_with(|| GraphNode {
            id: tid.clone(), label: tid.clone(),
            node_type: NodeType::Email, weight: 0,
            metadata: serde_json::json!({}),
        }).weight += 1;

        let key = format!("{}->{}:{:?}", fid, tid, EdgeType::Emailed);
        let e = edges.entry(key).or_insert(GraphEdge {
            source: fid.clone(), target: tid.clone(),
            relation: EdgeType::Emailed,
            count: 0, last_seen: ts.clone(),
        });
        e.count += 1;
        if ts > e.last_seen { e.last_seen = ts.clone(); }
    }

    for point in geo_points {
        let lat = point["lat"].as_f64().or_else(|| point["latitude"].as_f64());
        let lon = point["lon"].as_f64().or_else(|| point["longitude"].as_f64());
        if let (Some(lat), Some(lon)) = (lat, lon) {
            let id = format!("loc:{:.6},{:.6}", lat, lon);
            let label = point["label"].as_str().unwrap_or(&id).to_string();
            nodes.entry(id.clone()).or_insert(GraphNode {
                id: id.clone(),
                label,
                node_type: NodeType::Location,
                weight: 1,
                metadata: point.clone(),
            });
        }
    }

    for point in geo_points {
        let binding = json_string(point, "number");
        let num = if binding.is_empty() { json_string(point, "device") } else { binding };
        if num.is_empty() { continue; }
        let lat = point["lat"].as_f64().or_else(|| point["latitude"].as_f64());
        let lon = point["lon"].as_f64().or_else(|| point["longitude"].as_f64());
        if let (Some(lat), Some(lon)) = (lat, lon) {
            let nid = normalize_number(&num);
            let lid = format!("loc:{:.6},{:.6}", lat, lon);
            if nodes.contains_key(&nid) && nodes.contains_key(&lid) {
                let key = format!("{}->{}:{:?}", nid, lid, EdgeType::LocatedAt);
                let ts = json_string(point, "timestamp");
                let e = edges.entry(key).or_insert(GraphEdge {
                    source: nid.clone(), target: lid.clone(),
                    relation: EdgeType::LocatedAt,
                    count: 0, last_seen: ts.clone(),
                });
                e.count += 1;
                if ts > e.last_seen { e.last_seen = ts.clone(); }
            }
        }
    }

    for tx in money_transactions {
        let from = json_string(tx, "from_num");
        let to = json_string(tx, "to_num");
        let ts = if json_string(tx, "timestamp").is_empty() { json_string(tx, "date") } else { json_string(tx, "timestamp") };
        let amount = tx["montant"].as_i64().or(tx["amount"].as_i64()).unwrap_or(0);

        if from.is_empty() || to.is_empty() || from == "?" || to == "?" { continue; }

        let fid = normalize_number(&from);
        let tid = normalize_number(&to);

        nodes.entry(fid.clone()).or_insert_with(|| GraphNode {
            id: fid.clone(), label: fid.clone(),
            node_type: NodeType::Financial, weight: 0,
            metadata: serde_json::json!({"total_sent": amount}),
        }).weight += 1;

        nodes.entry(tid.clone()).or_insert_with(|| GraphNode {
            id: tid.clone(), label: tid.clone(),
            node_type: NodeType::Financial, weight: 0,
            metadata: serde_json::json!({"total_received": amount}),
        }).weight += 1;

        let key = format!("{}->{}:{:?}", fid, tid, EdgeType::Transferred);
        let e = edges.entry(key).or_insert(GraphEdge {
            source: fid.clone(), target: tid.clone(),
            relation: EdgeType::Transferred,
            count: 0, last_seen: ts.clone(),
        });
        e.count += 1;
        if ts > e.last_seen { e.last_seen = ts.clone(); }
    }

    for i in 0..geo_points.len() {
        for j in (i + 1)..geo_points.len() {
            let lat1 = geo_points[i]["lat"].as_f64().or_else(|| geo_points[i]["latitude"].as_f64());
            let lon1 = geo_points[i]["lon"].as_f64().or_else(|| geo_points[i]["longitude"].as_f64());
            let lat2 = geo_points[j]["lat"].as_f64().or_else(|| geo_points[j]["latitude"].as_f64());
            let lon2 = geo_points[j]["lon"].as_f64().or_else(|| geo_points[j]["longitude"].as_f64());
            if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) = (lat1, lon1, lat2, lon2) {
                let dist = haversine(lat1, lon1, lat2, lon2);
                if dist <= 1.0 {
                    let id1 = format!("loc:{:.6},{:.6}", lat1, lon1);
                    let id2 = format!("loc:{:.6},{:.6}", lat2, lon2);
                    if nodes.contains_key(&id1) && nodes.contains_key(&id2) {
                        let key = format!("{}->{}:{:?}", id1, id2, EdgeType::Nearby);
                        edges.entry(key).or_insert(GraphEdge {
                            source: id1.clone(), target: id2.clone(),
                            relation: EdgeType::Nearby,
                            count: 1, last_seen: String::new(),
                        });
                    }
                }
            }
        }
    }

    let nodes_list: Vec<GraphNode> = nodes.into_values().collect();
    let edges_list: Vec<GraphEdge> = edges.into_values().collect();

    GraphData { nodes: nodes_list, edges: edges_list }
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

#[tauri::command]
pub fn build_link_graph(
    contacts: Vec<serde_json::Value>,
    messages: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let graph = build_graph_internal(&contacts, &messages, &[], &[], &[], &[]);
    Ok(serde_json::to_value(&graph).unwrap_or(serde_json::json!({})))
}

#[tauri::command]
pub fn build_comprehensive_graph(
    contacts: Vec<serde_json::Value>,
    messages: Vec<serde_json::Value>,
    call_history: Vec<serde_json::Value>,
    emails: Vec<serde_json::Value>,
    geo_points: Vec<serde_json::Value>,
    money_transactions: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let graph = build_graph_internal(&contacts, &messages, &call_history, &emails, &geo_points, &money_transactions);
    Ok(serde_json::to_value(&graph).unwrap_or(serde_json::json!({})))
}

fn compute_degree_centrality(graph: &GraphData) -> HashMap<String, f64> {
    let n = graph.nodes.len() as f64;
    if n <= 1.0 { return HashMap::new(); }
    let mut degrees: HashMap<String, usize> = HashMap::new();
    for node in &graph.nodes {
        degrees.insert(node.id.clone(), 0);
    }
    for edge in &graph.edges {
        *degrees.entry(edge.source.clone()).or_insert(0) += 1;
        *degrees.entry(edge.target.clone()).or_insert(0) += 1;
    }
    let max_degree = *degrees.values().max().unwrap_or(&1) as f64;
    degrees.into_iter()
        .map(|(k, v)| (k, v as f64 / max_degree.max(1.0)))
        .collect()
}

fn compute_betweenness(graph: &GraphData) -> HashMap<String, f64> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        adj.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        adj.entry(edge.source.clone()).or_default().push(edge.target.clone());
        adj.entry(edge.target.clone()).or_default().push(edge.source.clone());
    }

    let mut betweenness: HashMap<String, f64> = HashMap::new();
    for node in &graph.nodes {
        betweenness.insert(node.id.clone(), 0.0);
    }

    let node_ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();

    for source in &node_ids {
        let mut stack: Vec<String> = Vec::new();
        let mut preds: HashMap<String, Vec<String>> = HashMap::new();
        let mut sigma: HashMap<String, f64> = HashMap::new();
        let mut dist: HashMap<String, i64> = HashMap::new();

        for nid in &node_ids {
            preds.insert(nid.clone(), Vec::new());
            sigma.insert(nid.clone(), 0.0);
            dist.insert(nid.clone(), -1);
        }
        sigma.insert(source.clone(), 1.0);
        dist.insert(source.clone(), 0);

        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(source.clone());

        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            if let Some(neighbors) = adj.get(&v) {
                for w in neighbors {
                    if *dist.get(w).unwrap_or(&-1) < 0 {
                        dist.insert(w.clone(), dist.get(&v).unwrap_or(&0) + 1);
                        queue.push_back(w.clone());
                    }
                    if *dist.get(w).unwrap_or(&-1) == dist.get(&v).unwrap_or(&0) + 1 {
                        let sigma_v = *sigma.get(&v).unwrap_or(&0.0);
                        *sigma.entry(w.clone()).or_insert(0.0) += sigma_v;
                        preds.entry(w.clone()).or_default().push(v.clone());
                    }
                }
            }
        }

        let mut delta: HashMap<String, f64> = HashMap::new();
        for nid in &node_ids {
            delta.insert(nid.clone(), 0.0);
        }

        while let Some(w) = stack.pop() {
            for v in preds.get(&w).unwrap_or(&Vec::new()) {
                let frac = sigma.get(v).unwrap_or(&0.0) / sigma.get(&w).unwrap_or(&1.0);
                let d = delta.get(&w).unwrap_or(&0.0);
                *delta.entry(v.clone()).or_insert(0.0) += frac * (1.0 + d);
            }
            if w != *source {
                *betweenness.entry(w.clone()).or_insert(0.0) += delta.get(&w).unwrap_or(&0.0);
            }
        }
    }

    let n = graph.nodes.len() as f64;
    let normalizer = if n > 2.0 { 1.0 / ((n - 1.0) * (n - 2.0)) } else { 1.0 };
    for v in betweenness.values_mut() {
        *v *= normalizer;
    }

    betweenness
}

fn detect_communities(graph: &GraphData) -> HashMap<String, usize> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        adj.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        adj.entry(edge.source.clone()).or_default().push(edge.target.clone());
        adj.entry(edge.target.clone()).or_default().push(edge.source.clone());
    }

    let mut community: HashMap<String, usize> = HashMap::new();
    let mut next_id = 0usize;

    for node in &graph.nodes {
        if community.contains_key(&node.id) { continue; }
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(node.id.clone());
        community.insert(node.id.clone(), next_id);

        while let Some(current) = queue.pop_front() {
            let cid = *community.get(&current).unwrap_or(&next_id);
            for neighbor in adj.get(&current).unwrap_or(&Vec::new()) {
                if !community.contains_key(neighbor) {
                    community.insert(neighbor.clone(), cid);
                    queue.push_back(neighbor.clone());
                }
            }
        }
        next_id += 1;
    }

    community
}

fn compute_risk_scores(graph: &GraphData) -> HashMap<String, f64> {
    let degree = compute_degree_centrality(graph);
    let mut edge_counts: HashMap<String, usize> = HashMap::new();
    let mut risk: HashMap<String, f64> = HashMap::new();

    for node in &graph.nodes {
        edge_counts.insert(node.id.clone(), 0);
    }
    for edge in &graph.edges {
        *edge_counts.entry(edge.source.clone()).or_insert(0) += 1;
        *edge_counts.entry(edge.target.clone()).or_insert(0) += 1;
    }

    for node in &graph.nodes {
        let deg = *degree.get(&node.id).unwrap_or(&0.0);
        let edge_count = *edge_counts.get(&node.id).unwrap_or(&0) as f64;
        let type_risk = match node.node_type {
            NodeType::Financial => 0.4,
            NodeType::Account => 0.3,
            NodeType::Device => 0.2,
            NodeType::Person => 0.15,
            NodeType::Email => 0.1,
            NodeType::Phone => 0.05,
            NodeType::Location => 0.05,
        };
        let score = (deg * 0.4 + type_risk + edge_count.min(20.0) * 0.02).min(1.0);
        risk.insert(node.id.clone(), score);
    }

    risk
}

#[tauri::command]
pub fn analyze_graph(graph_data: serde_json::Value) -> Result<GraphAnalysis, String> {
    let graph: GraphData = serde_json::from_value(graph_data)
        .map_err(|e| format!("Invalid graph data: {}", e))?;

    let degree_centrality = compute_degree_centrality(&graph);
    let betweenness = compute_betweenness(&graph);
    let communities = detect_communities(&graph);
    let risk_scores = compute_risk_scores(&graph);

    let mut edge_set: HashSet<String> = HashSet::new();
    for edge in &graph.edges {
        edge_set.insert(edge.source.clone());
        edge_set.insert(edge.target.clone());
    }

    let isolated_nodes: Vec<String> = graph.nodes.iter()
        .filter(|n| !edge_set.contains(&n.id))
        .map(|n| n.id.clone())
        .collect();

    let mut betweenness_vec: Vec<(String, f64)> = betweenness.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    betweenness_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let key_bridges: Vec<String> = betweenness_vec.iter()
        .take(10)
        .filter(|(_, v)| *v > 0.0)
        .map(|(k, _)| k.clone())
        .collect();

    Ok(GraphAnalysis {
        degree_centrality,
        betweenness,
        communities,
        risk_scores,
        key_bridges,
        isolated_nodes,
    })
}

#[tauri::command]
pub fn find_connections(
    graph_data: serde_json::Value,
    from_id: String,
    to_id: String,
    max_hops: usize,
) -> Result<Vec<String>, String> {
    let graph: GraphData = serde_json::from_value(graph_data)
        .map_err(|e| format!("Invalid graph data: {}", e))?;

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        adj.insert(node.id.clone(), Vec::new());
    }
    for edge in &graph.edges {
        adj.entry(edge.source.clone()).or_default().push(edge.target.clone());
        adj.entry(edge.target.clone()).or_default().push(edge.source.clone());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
    queue.push_back((from_id.clone(), vec![from_id.clone()]));
    visited.insert(from_id.clone());

    while let Some((current, path)) = queue.pop_front() {
        if current == to_id {
            return Ok(path);
        }
        if path.len() - 1 >= max_hops {
            continue;
        }
        for neighbor in adj.get(&current).unwrap_or(&Vec::new()) {
            if !visited.contains(neighbor) {
                visited.insert(neighbor.clone());
                let mut new_path = path.clone();
                new_path.push(neighbor.clone());
                queue.push_back((neighbor.clone(), new_path));
            }
        }
    }

    Ok(Vec::new())
}

#[tauri::command]
pub fn get_node_details(
    graph_data: serde_json::Value,
    node_id: String,
) -> Result<serde_json::Value, String> {
    let graph: GraphData = serde_json::from_value(graph_data)
        .map_err(|e| format!("Invalid graph data: {}", e))?;

    let node = graph.nodes.iter()
        .find(|n| n.id == node_id)
        .ok_or_else(|| format!("Node not found: {}", node_id))?;

    let connected_edges: Vec<&GraphEdge> = graph.edges.iter()
        .filter(|e| e.source == node_id || e.target == node_id)
        .collect();

    let mut neighbor_ids: HashSet<String> = HashSet::new();
    for edge in &connected_edges {
        if edge.source == node_id {
            neighbor_ids.insert(edge.target.clone());
        } else {
            neighbor_ids.insert(edge.source.clone());
        }
    }

    let neighbors: Vec<&GraphNode> = graph.nodes.iter()
        .filter(|n| neighbor_ids.contains(&n.id))
        .collect();

    Ok(serde_json::json!({
        "node": node,
        "edges": connected_edges,
        "neighbors": neighbors,
        "neighbor_count": neighbors.len(),
        "edge_count": connected_edges.len(),
    }))
}

#[tauri::command]
pub fn export_graph(
    graph_data: serde_json::Value,
    format: String,
) -> Result<String, String> {
    let graph: GraphData = serde_json::from_value(graph_data)
        .map_err(|e| format!("Invalid graph data: {}", e))?;

    match format.to_lowercase().as_str() {
        "gexf" => {
            let mut output = String::new();
            output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
            output.push('\n');
            output.push_str(r#"<gexf xmlns="http://www.gexf.net/1.2draft" version="1.2">"#);
            output.push('\n');
            output.push_str(r#"  <graph mode="static" defaultedgetype="directed">"#);
            output.push('\n');
            output.push_str("    <nodes>\n");
            for node in &graph.nodes {
                output.push_str(&format!(
                    r#"      <node id="{}" label="{}">"#,
                    xml_escape(&node.id),
                    xml_escape(&node.label)
                ));
                output.push('\n');
                output.push_str(&format!(
                    r#"        <attvalues><attvalue for="type" value="{}"/><attvalue for="weight" value="{}"/></attvalues>"#,
                    node.node_type, node.weight
                ));
                output.push('\n');
                output.push_str("      </node>\n");
            }
            output.push_str("    </nodes>\n");
            output.push_str("    <edges>\n");
            for (i, edge) in graph.edges.iter().enumerate() {
                output.push_str(&format!(
                    r#"      <edge id="{}" source="{}" target="{}">"#,
                    i,
                    xml_escape(&edge.source),
                    xml_escape(&edge.target)
                ));
                output.push('\n');
                output.push_str(&format!(
                    r#"        <attvalues><attvalue for="relation" value="{}"/><attvalue for="count" value="{}"/></attvalues>"#,
                    edge.relation, edge.count
                ));
                output.push('\n');
                output.push_str("      </edge>\n");
            }
            output.push_str("    </edges>\n");
            output.push_str("  </graph>\n");
            output.push_str("</gexf>");
            Ok(output)
        }
        "json" | _ => {
            serde_json::to_string_pretty(&graph).map_err(|e| e.to_string())
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}


