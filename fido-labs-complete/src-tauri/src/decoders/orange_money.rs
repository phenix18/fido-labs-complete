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
                    date: date.clone(),
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
    None
}
