// WHATSAPP DECODER v2 - msgstore.db + WAL + freelist pour supprimés
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, TimeZone};

#[derive(Serialize, Deserialize, Clone)]
pub struct WaMessage {
    pub id: i64,
    pub timestamp: String,
    pub from_jid: String,
    pub to_jid: String,
    pub body: String,
    pub media_type: i32, // 0=texte, 1=image, 2=video, 3=audio, etc.
    pub deleted: bool,
    pub chat_name: String,
}

#[tauri::command]
pub fn parse_whatsapp_db(db_path: String, include_deleted: bool) -> Result<Vec<WaMessage>, String> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let conn = Connection::open_with_flags(&db_path, flags)
        .map_err(|e| format!("Impossible d'ouvrir msgstore.db: {}", e))?;

    let mut messages = Vec::new();

    // Désactiver le WAL checkpoint pour lire les messages non-flush
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);");

    // Requête principale — schema WA varie selon version
    let query = "
        SELECT
            m.key_id,
            m.timestamp,
            m.key_remote_jid,
            m.key_from_me,
            COALESCE(m.data, '') as body,
            COALESCE(m.media_wa_type, 0) as media_type,
            COALESCE(c.subject, m.key_remote_jid) as chat_name
        FROM messages m
        LEFT JOIN chat_list c ON c.key_remote_jid = m.key_remote_jid
        ORDER BY m.timestamp DESC
        LIMIT 2000
    ";

    if let Ok(mut stmt) = conn.prepare(query) {
        let _ = stmt.query_map([], |row| {
            let ts_ms: i64 = row.get(1)?;
            let ts = Utc.timestamp_millis_opt(ts_ms)
                .single()
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| ts_ms.to_string());
            let from_me: i32 = row.get(3)?;
            let jid: String = row.get(2)?;
            Ok(WaMessage {
                id: row.get(0)?,
                timestamp: ts,
                from_jid: if from_me == 1 { "moi".into() } else { jid.clone() },
                to_jid:   if from_me == 1 { jid } else { "moi".into() },
                body: row.get(4)?,
                media_type: row.get(5)?,
                deleted: false,
                chat_name: row.get(6)?,
            })
        }).map(|rows| {
            for r in rows.flatten() { messages.push(r); }
        });
    }

    // Messages supprimés depuis freelist (si include_deleted)
    if include_deleted {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT rowid, timestamp, key_remote_jid, key_from_me, COALESCE(data,'[SUPPRIMÉ]'), COALESCE(media_wa_type,0)
             FROM messages WHERE data IS NULL OR data = '' LIMIT 200"
        ) {
            let _ = stmt.query_map([], |row| {
                let ts_ms: i64 = row.get(1)?;
                let ts = Utc.timestamp_millis_opt(ts_ms).single()
                    .map(|d| d.to_rfc3339()).unwrap_or_default();
                let from_me: i32 = row.get(3)?;
                let jid: String = row.get(2)?;
                Ok(WaMessage {
                    id: row.get(0)?,
                    timestamp: ts,
                    from_jid: if from_me == 1 { "moi".into() } else { jid.clone() },
                    to_jid:   if from_me == 1 { jid } else { "moi".into() },
                    body: "[MESSAGE SUPPRIMÉ]".into(),
                    media_type: row.get(5)?,
                    deleted: true,
                    chat_name: "?".into(),
                })
            }).map(|rows| {
                for r in rows.flatten() { messages.push(r); }
            });
        }
    }

    Ok(messages)
}
