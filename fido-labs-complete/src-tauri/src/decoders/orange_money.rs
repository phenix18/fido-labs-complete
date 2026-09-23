// ORANGE MONEY / MOOV / WAVE / MTN MOBILE MONEY PARSER v2
// Parse mmssms.db + SMS texte brut
use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MoneyTx {
    pub id: String,
    pub date: String,
    pub provider: String,          // ORANGE, MOOV, WAVE, MTN
    pub type_tx: String,           // RECU, ENVOYE, PAIEMENT, RETRAIT, DEPOT, FRAIS
    pub montant: i64,
    pub currency: String,          // XOF, XAF, GNF
    pub from_num: String,
    pub to_num: String,
    pub balance_after: i64,
    pub reference: String,
    pub raw_sms: String,
}

#[tauri::command]
pub fn parse_mobile_money(sms_db_path: String, provider_filter: String) -> Result<Vec<MoneyTx>, String> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
    let conn = Connection::open_with_flags(&sms_db_path, flags)
        .map_err(|e| format!("Impossible d'ouvrir SMS DB: {}", e))?;

    // Schema mmssms.db Android standard
    let mut sms_list: Vec<(String, String, String)> = Vec::new(); // (date, address, body)
    if let Ok(mut stmt) = conn.prepare(
        "SELECT date, address, body FROM sms WHERE body LIKE '%vous avez%' OR body LIKE '%received%' OR body LIKE '%francs%' OR body LIKE '%FCFA%' OR body LIKE '%recu%' ORDER BY date DESC LIMIT 5000"
    ) {
        let _ = stmt.query_map([], |row| {
            Ok((
                row.get::<_,String>(0).unwrap_or_default(),
                row.get::<_,String>(1).unwrap_or_default(),
                row.get::<_,String>(2).unwrap_or_default(),
            ))
        }).map(|rows| {
            for r in rows.flatten() { sms_list.push(r); }
        });
    }

    let mut txs = Vec::new();
    for (date, address, body) in &sms_list {
        let body_lower = body.to_lowercase();
        let provider = detect_provider(&address, &body_lower);
        if !provider_filter.is_empty() && provider != provider_filter { continue; }

        if let Some(tx) = parse_sms_body(body, &provider, date) {
            txs.push(tx);
        }
    }

    Ok(txs)
}

fn detect_provider(address: &str, body: &str) -> String {
    let addr = address.to_uppercase();
    if addr.contains("ORANGE") || body.contains("orange money") { return "ORANGE".into(); }
    if addr.contains("MOOV") || body.contains("moov money") { return "MOOV".into(); }
    if addr.contains("WAVE") || body.contains("wave") { return "WAVE".into(); }
    if addr.contains("MTN") || body.contains("mtn mobile money") { return "MTN".into(); }
    if body.contains("flooz") { return "MOOV".into(); }
    "INCONNU".into()
}

fn parse_sms_body(body: &str, provider: &str, date: &str) -> Option<MoneyTx> {
    use uuid::Uuid;

    // Patterns Orange Money (Afrique de l'Ouest - FCFA/XOF)
    let patterns: Vec<(&str, &str, &str)> = vec![
        // (regex, type_tx, devise)
        (r"(?i)vous avez re[cç]u\s+([0-9\s]+)\s*(?:F|FCFA|XOF).*de\s+(\+?[0-9]+)", "RECU", "XOF"),
        (r"(?i)vous avez envoye\s+([0-9\s]+)\s*(?:F|FCFA|XOF).*[àa]\s+(\+?[0-9]+)", "ENVOYE", "XOF"),
        (r"(?i)paiement de\s+([0-9\s]+)\s*(?:F|FCFA|XOF)", "PAIEMENT", "XOF"),
        (r"(?i)retrait de\s+([0-9\s]+)\s*(?:F|FCFA|XOF)", "RETRAIT", "XOF"),
        (r"(?i)depot de\s+([0-9\s]+)\s*(?:F|FCFA|XOF)", "DEPOT", "XOF"),
        // Wave
        (r"(?i)([0-9\s]+)\s*FCFA.*reçu de\s+(\+?[0-9]+)", "RECU", "XOF"),
        (r"(?i)envoye\s+([0-9\s]+)\s*FCFA.*[àa]\s+(\+?[0-9]+)", "ENVOYE", "XOF"),
        // MTN (XAF Cameroun)
        (r"(?i)received\s+([0-9\s]+)\s*(?:XAF|FCFA).*from\s+(\+?[0-9]+)", "RECU", "XAF"),
        (r"(?i)sent\s+([0-9\s]+)\s*(?:XAF|FCFA).*to\s+(\+?[0-9]+)", "ENVOYE", "XAF"),
    ];

    // Solde restant
    let balance_re = Regex::new(r"(?i)solde[:\s]+([0-9\s]+)\s*(?:F|FCFA|XOF|XAF)").ok();
    let ref_re = Regex::new(r"(?i)(?:ref|id|transaction)[:\s#]+([A-Z0-9]+)").ok();

    let balance_after = balance_re.as_ref()
        .and_then(|r| r.captures(body))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().replace(' ', "").parse::<i64>().ok())
        .unwrap_or(-1);

    let reference = ref_re.as_ref()
        .and_then(|r| r.captures(body))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string()[..8].to_uppercase());

    for (pattern, type_tx, currency) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(body) {
                let montant_str = caps.get(1).map(|m| m.as_str().replace(' ', "")).unwrap_or_default();
                let montant = montant_str.parse::<i64>().unwrap_or(0);
                if montant == 0 { continue; }

                let num = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                let (from_num, to_num) = match *type_tx {
                    "RECU" => (num, "moi".into()),
                    "ENVOYE" => ("moi".into(), num),
                    _ => ("moi".into(), "commerçant".into()),
                };

                return Some(MoneyTx {
                    id: Uuid::new_v4().to_string(),
                    date: date.to_string(),
                    provider: provider.to_string(),
                    type_tx: type_tx.to_string(),
                    montant,
                    currency: currency.to_string(),
                    from_num,
                    to_num,
                    balance_after,
                    reference,
                    raw_sms: body.chars().take(200).collect(),
                });
            }
        }
    }

    // ── Repli générique par mots-clés (XOF/FCFA + type déduit du texte) ──
    // Capte les formulations non couvertes par les patterns ci-dessus.
    let bl = body.to_lowercase();
    let amount_re = Regex::new(r"(?i)([0-9][0-9\s.]{1,})\s*(?:f\b|fcfa|xof|xaf|francs?)").ok();
    if let Some(m) = amount_re.as_ref().and_then(|r| r.captures(body)).and_then(|c| c.get(1)) {
        let montant = m.as_str().replace(' ', "").replace('.', "").parse::<i64>().unwrap_or(0);
        if montant > 0 {
            let type_tx = if bl.contains("retrait") { "RETRAIT" }
                else if bl.contains("depot") || bl.contains("dépôt") { "DEPOT" }
                else if bl.contains("paiement") || bl.contains("achat") { "PAIEMENT" }
                else if bl.contains("envoy") || bl.contains("transf") { "ENVOYE" }
                else if bl.contains("recu") || bl.contains("reçu") || bl.contains("credit") || bl.contains("received") { "RECU" }
                else { "AUTRE" };
            let num = Regex::new(r"\+?\d[\d\s]{6,}\d").ok()
                .and_then(|r| r.find(body))
                .map(|mm| mm.as_str().replace(' ', ""))
                .unwrap_or_default();
            let (from_num, to_num) = match type_tx {
                "RECU" | "DEPOT" => (if num.is_empty() { "?".into() } else { num }, "moi".to_string()),
                _ => ("moi".to_string(), if num.is_empty() { "commerçant".into() } else { num }),
            };
            let currency = if bl.contains("xaf") { "XAF" } else { "XOF" };
            return Some(MoneyTx {
                id: Uuid::new_v4().to_string(), date: date.to_string(),
                provider: provider.to_string(), type_tx: type_tx.to_string(),
                montant, currency: currency.into(), from_num, to_num,
                balance_after, reference, raw_sms: body.chars().take(200).collect(),
            });
        }
    }
    None
}

/// Mots-clés de pré-filtrage mobile money (déclenchent l'analyse d'un message).
const MONEY_KEYWORDS: &[&str] = &[
    "xof", "fcfa", "xaf", "orange money", "wave", "moov", "mtn", "flooz",
    "argent", "depot", "dépôt", "recu", "reçu", "paiement", "retrait",
    "envoy", "transf", "solde", "franc", "received", "sent",
];

/// Analyse les SMS/messages DÉJÀ EXTRAITS (en mémoire) par mots-clés et construit
/// le graphe de flux financiers — sans base SQLite séparée. Chaque message porte
/// `body`/`text`, `contact`/`address`/`from_jid` et `date`/`timestamp`.
#[tauri::command]
pub fn scan_money_from_messages(messages: Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
    let s = |m: &serde_json::Value, keys: &[&str]| -> String {
        for k in keys { if let Some(v) = m.get(*k).and_then(|v| v.as_str()) { if !v.is_empty() { return v.to_string(); } } }
        String::new()
    };
    let mut txs: Vec<MoneyTx> = Vec::new();
    for m in &messages {
        let body = s(m, &["body", "text"]);
        if body.is_empty() { continue; }
        let address = s(m, &["contact", "address", "from_jid", "from"]);
        let date = s(m, &["date", "timestamp"]);
        let bl = body.to_lowercase();
        if !MONEY_KEYWORDS.iter().any(|k| bl.contains(k)) { continue; }
        let provider = detect_provider(&address, &bl);
        if let Some(mut tx) = parse_sms_body(&body, &provider, &date) {
            // Contrepartie : compléter avec l'expéditeur du SMS si numéro et placeholder.
            let addr_is_num = address.chars().filter(|c| c.is_ascii_digit()).count() >= 6;
            if addr_is_num && (tx.from_num == "?" || (tx.type_tx == "RECU" && tx.from_num == "moi")) {
                tx.from_num = address.clone();
            }
            txs.push(tx);
        }
    }
    let tx_values: Vec<serde_json::Value> = txs.iter().filter_map(|t| serde_json::to_value(t).ok()).collect();
    let graph = crate::analyzer::money_flow::analyze_money_flow(tx_values)?;
    Ok(serde_json::json!({ "transactions": txs, "graph": graph, "count": txs.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecte_les_transactions_par_mots_cles() {
        let messages = vec![
            serde_json::json!({"date":"2024-03-15 09:10:44","contact":"ORANGEMONEY",
                "body":"Vous avez recu 150000 FCFA de +22607654321. Solde: 182500 FCFA. Ref: OM240315"}),
            serde_json::json!({"date":"2024-03-15 11:20:00","contact":"WAVE",
                "body":"Paiement de 25 000 FCFA effectue chez BOUTIQUE. Solde: 157500 F"}),
            serde_json::json!({"date":"2024-03-16 08:00:00","contact":"+22670000000",
                "body":"Retrait de 50000 F au point cash. Solde 107500 FCFA"}),
            serde_json::json!({"date":"2024-03-16 09:00:00","contact":"Maman",
                "body":"Bonjour, tu passes ce soir ?"}), // non financier -> ignoré
        ];
        let r = scan_money_from_messages(messages).expect("scan");
        assert_eq!(r["count"], 3, "3 transactions détectées");
        let txs = r["transactions"].as_array().unwrap();
        let recu = txs.iter().find(|t| t["type_tx"] == "RECU").unwrap();
        assert_eq!(recu["montant"], 150000);
        assert_eq!(recu["from_num"], "+22607654321");
        assert!(txs.iter().any(|t| t["type_tx"] == "RETRAIT" && t["montant"] == 50000));
        assert!(txs.iter().any(|t| t["type_tx"] == "PAIEMENT" && t["montant"] == 25000));
        // Le graphe est construit
        assert!(r["graph"]["total_amount"].as_i64().unwrap() >= 225000);
    }
}
