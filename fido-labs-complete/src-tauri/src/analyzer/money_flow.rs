// MONEY FLOW ANALYZER v2 - Graphe de flux financiers mobile money
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct MoneyNode {
    pub id: String,
    pub number: String,
    pub total_received: i64,
    pub total_sent: i64,
    pub tx_count: usize,
    pub provider: String,
    pub suspicious: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoneyEdge {
    pub from: String,
    pub to: String,
    pub total_amount: i64,
    pub count: usize,
    pub provider: String,
}

#[tauri::command]
pub fn analyze_money_flow(transactions: Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
    let mut nodes: HashMap<String, MoneyNode> = HashMap::new();
    let mut edges: HashMap<String, MoneyEdge> = HashMap::new();

    for tx in &transactions {
        let from = tx["from_num"].as_str().unwrap_or("?").to_string();
        let to   = tx["to_num"].as_str().unwrap_or("?").to_string();
        let amount = tx["montant"].as_i64().unwrap_or(0);
        let provider = tx["provider"].as_str().unwrap_or("?").to_string();

        // Noeud expéditeur
        let n_from = nodes.entry(from.clone()).or_insert(MoneyNode {
            id: from.clone(), number: from.clone(),
            total_received: 0, total_sent: 0, tx_count: 0,
            provider: provider.clone(), suspicious: false,
        });
        n_from.total_sent += amount;
        n_from.tx_count += 1;

        // Noeud destinataire
        let n_to = nodes.entry(to.clone()).or_insert(MoneyNode {
            id: to.clone(), number: to.clone(),
            total_received: 0, total_sent: 0, tx_count: 0,
            provider: provider.clone(), suspicious: false,
        });
        n_to.total_received += amount;
        n_to.tx_count += 1;

        // Arête
        let edge_key = format!("{}_{}", from, to);
        let e = edges.entry(edge_key).or_insert(MoneyEdge {
            from: from.clone(), to: to.clone(),
            total_amount: 0, count: 0, provider: provider.clone(),
        });
        e.total_amount += amount;
        e.count += 1;
    }

    // Détection comportements suspects
    let threshold_high = 500_000i64; // 500k FCFA
    for (_, node) in nodes.iter_mut() {
        if node.total_received > threshold_high || node.total_sent > threshold_high
            || node.tx_count > 20 {
            node.suspicious = true;
        }
    }

    let nodes_list: Vec<&MoneyNode> = nodes.values().collect();
    let edges_list: Vec<&MoneyEdge> = edges.values().collect();

    Ok(serde_json::json!({
        "nodes": nodes_list,
        "edges": edges_list,
        "total_transactions": transactions.len(),
        "total_amount": transactions.iter().map(|t| t["montant"].as_i64().unwrap_or(0)).sum::<i64>(),
        "suspicious_nodes": nodes.values().filter(|n| n.suspicious).count(),
    }))
}
