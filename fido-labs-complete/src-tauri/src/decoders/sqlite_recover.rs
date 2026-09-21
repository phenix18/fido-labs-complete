// RÉCUPÉRATION DE DONNÉES EFFACÉES — bases SQLite (SMS, WhatsApp, apps…)
// Technique forensique : une ligne supprimée n'est pas effacée physiquement ;
// son contenu subsiste dans les zones NON ALLOUÉES des pages (freeblocks) et
// dans les pages de la freelist, jusqu'à un VACUUM.
//
// On NE reconstruit PAS le record complet : quand une cellule est supprimée, ses
// premiers octets sont écrasés par l'en-tête du freeblock, donc l'en-tête du
// record est souvent corrompu. En revanche le CONTENU texte (corps du message,
// numéros, noms) reste intact. On le récupère par "string carving" ciblé sur les
// seules zones libérées — ce que font les outils forensiques (undark, bring2lite…).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone)]
pub struct RecoveredRecord {
    pub page: u32,
    pub offset: u32,
    pub source: String,       // "freeblock" | "non alloué" | "page libérée"
    pub values: Vec<String>,
    pub text: String,
}

fn printable(b: u8) -> bool {
    (0x20..0x7f).contains(&b) || b >= 0x80 // ASCII imprimable + octets UTF-8 (accents)
}

/// Extrait les chaînes lisibles d'une zone [start, end) et les ajoute au résultat.
fn carve_strings(
    page: &[u8], start: usize, end: usize, page_i: usize, source: &str,
    out: &mut Vec<RecoveredRecord>, seen: &mut HashSet<String>,
) {
    let end = end.min(page.len());
    if start >= end { return; }
    let mut i = start;
    while i < end {
        let s = i;
        while i < end && printable(page[i]) { i += 1; }
        if i - s >= 4 {
            let text = String::from_utf8_lossy(&page[s..i]).trim().to_string();
            let letters = text.chars().filter(|c| c.is_alphabetic()).count();
            if letters >= 3 && seen.insert(text.clone()) {
                out.push(RecoveredRecord {
                    page: page_i as u32,
                    offset: s as u32,
                    source: source.into(),
                    values: vec![text.clone()],
                    text,
                });
            }
        }
        i += 1;
    }
}

/// Récupère les fragments effacés d'une base SQLite.
pub fn recover_deleted(db_path: &str) -> Result<Vec<RecoveredRecord>, String> {
    let data = std::fs::read(db_path).map_err(|e| format!("Lecture {}: {}", db_path, e))?;
    if data.len() < 100 || &data[0..15] != b"SQLite format 3" {
        return Err("Fichier SQLite invalide (en-tête absent)".into());
    }
    let page_size = {
        let ps = u16::from_be_bytes([data[16], data[17]]) as usize;
        if ps == 1 { 65536 } else { ps }
    };
    if page_size < 512 || page_size > 65536 {
        return Err(format!("Taille de page invalide: {}", page_size));
    }
    let n_pages = data.len() / page_size;
    let mut out: Vec<RecoveredRecord> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1) Pages "table leaf" (0x0D) : zone non allouée + freeblocks (cellules supprimées)
    for pi in 0..n_pages {
        let page_start = pi * page_size;
        let page = &data[page_start..(page_start + page_size).min(data.len())];
        let h = if pi == 0 { 100 } else { 0 };
        if h + 8 > page.len() || page[h] != 0x0D { continue; }

        let ncells = u16::from_be_bytes([page[h + 3], page[h + 4]]) as usize;
        let content_start = {
            let v = u16::from_be_bytes([page[h + 5], page[h + 6]]) as usize;
            if v == 0 { 65536 } else { v }
        };
        let first_fb = u16::from_be_bytes([page[h + 1], page[h + 2]]) as usize;
        let ptr_end = h + 8 + ncells * 2;

        carve_strings(page, ptr_end, content_start, pi, "non alloué", &mut out, &mut seen);

        let mut fb = first_fb;
        let mut guard = 0;
        while fb >= 4 && fb + 4 <= page.len() && guard < 4096 {
            let next = u16::from_be_bytes([page[fb], page[fb + 1]]) as usize;
            let size = u16::from_be_bytes([page[fb + 2], page[fb + 3]]) as usize;
            carve_strings(page, fb + 4, fb + size.max(4), pi, "freeblock", &mut out, &mut seen);
            if next == 0 || next <= fb { break; }
            fb = next;
            guard += 1;
        }
    }

    // 2) Pages de la freelist (pages entièrement libérées après suppression massive)
    let free_trunk = u32::from_be_bytes([data[32], data[33], data[34], data[35]]) as usize;
    let mut trunk = free_trunk;
    let mut tguard = 0;
    while trunk >= 1 && trunk <= n_pages && tguard <= n_pages {
        let ts = (trunk - 1) * page_size;
        if ts + 8 > data.len() { break; }
        let tp = &data[ts..(ts + page_size).min(data.len())];
        let next_trunk = u32::from_be_bytes([tp[0], tp[1], tp[2], tp[3]]) as usize;
        let nleaf = u32::from_be_bytes([tp[4], tp[5], tp[6], tp[7]]) as usize;
        for k in 0..nleaf.min(page_size / 4) {
            let off = 8 + k * 4;
            if off + 4 > tp.len() { break; }
            let leaf = u32::from_be_bytes([tp[off], tp[off + 1], tp[off + 2], tp[off + 3]]) as usize;
            if leaf >= 1 && leaf <= n_pages {
                let ls = (leaf - 1) * page_size;
                let lp = &data[ls..(ls + page_size).min(data.len())];
                carve_strings(lp, 0, lp.len(), leaf - 1, "page libérée", &mut out, &mut seen);
            }
        }
        if next_trunk == 0 || next_trunk == trunk { break; }
        trunk = next_trunk;
        tguard += 1;
    }

    Ok(out)
}

/// Commande Tauri : récupère les lignes effacées et enregistre le résultat.
#[tauri::command]
pub fn recover_deleted_records(db_path: String, output_dir: String) -> Result<serde_json::Value, String> {
    let records = recover_deleted(&db_path)?;
    let total = records.len();

    if !output_dir.is_empty() {
        let _ = std::fs::create_dir_all(&output_dir);
        let path = format!("{}/recovered_deleted.json", output_dir.trim_end_matches(['/', '\\']));
        std::fs::write(&path, serde_json::to_string_pretty(&records).unwrap_or_default()).ok();
    }

    let view: Vec<_> = records.iter().take(500).collect();
    Ok(serde_json::json!({
        "summary": format!("{} fragment(s) effacé(s) récupéré(s)", total),
        "total": total,
        "records": view,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn recupere_lignes_effacees_reelles() {
        // Crée une VRAIE base SQLite, insère des messages, en supprime, puis récupère.
        let path = std::env::temp_dir().join("fido_recover_test.db");
        let p = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&p);
        {
            let conn = Connection::open(&p).unwrap();
            conn.execute_batch(
                "CREATE TABLE messages(id INTEGER PRIMARY KEY, contact TEXT, body TEXT);
                 INSERT INTO messages(contact, body) VALUES('+22670000001','Message conserve un');
                 INSERT INTO messages(contact, body) VALUES('+22670000002','SECRET rendez-vous minuit au port');
                 INSERT INTO messages(contact, body) VALUES('+22670000003','Autre message a effacer confidentiel');
                 INSERT INTO messages(contact, body) VALUES('+22670000004','Message conserve deux');
                 DELETE FROM messages WHERE id IN (2,3);"
            ).unwrap();
        }
        let recovered = recover_deleted(&p).unwrap();
        let all_text: String = recovered.iter().map(|r| r.text.clone()).collect::<Vec<_>>().join("\n");
        assert!(all_text.contains("SECRET rendez-vous minuit"), "doit récupérer le message effacé 1. Trouvé: {}", all_text);
        assert!(all_text.contains("confidentiel"), "doit récupérer le message effacé 2. Trouvé: {}", all_text);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn rejette_fichier_non_sqlite() {
        let path = std::env::temp_dir().join("fido_not_sqlite.bin");
        let p = path.to_string_lossy().to_string();
        std::fs::write(&p, b"ceci n'est pas une base SQLite").unwrap();
        assert!(recover_deleted(&p).is_err());
        let _ = std::fs::remove_file(&p);
    }
}
