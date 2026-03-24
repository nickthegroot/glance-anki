use anyhow::{Result, anyhow};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;

use super::types::{DayReviews, MATURE_INTERVAL_DAYS, ReviewType, SECS_PER_DAY};

pub type RevlogRow = (i32, i32, i32);

pub fn open_collection_readonly(collection_path: &str) -> Result<Connection> {
    // `immutable=1` tells SQLite to skip locking and WAL-index writes.
    // Safe here because we never modify the database.
    let uri = format!("file:{}?immutable=1", collection_path);
    Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        anyhow!(
            "Failed to open Anki database at '{}': {}",
            collection_path,
            e
        )
    })
}

pub fn read_rollover_hour(conn: &Connection) -> u32 {
    const DEFAULT_ROLLOVER_HOUR: u32 = 4;

    read_rollover_from_config_table(conn)
        .or_else(|| read_rollover_from_legacy_col_conf(conn))
        .unwrap_or(DEFAULT_ROLLOVER_HOUR)
}

fn read_rollover_from_config_table(conn: &Connection) -> Option<u32> {
    conn.query_row(
        "SELECT val FROM config WHERE key = 'rollover' LIMIT 1",
        [],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .ok()
    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    .and_then(|v| v.as_u64())
    .map(|v| v as u32)
}

fn read_rollover_from_legacy_col_conf(conn: &Connection) -> Option<u32> {
    conn.query_row("SELECT conf FROM col LIMIT 1", [], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .filter(|s| !s.is_empty())
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .and_then(|v| v.get("rollover").and_then(|r| r.as_u64()))
    .map(|v| v as u32)
}

pub fn resolve_deck_ids(conn: &Connection, deck_name: &str) -> Result<Vec<i64>> {
    let decks_json: String = conn
        .query_row("SELECT decks FROM col LIMIT 1", [], |row| row.get(0))
        .map_err(|e| anyhow!("Failed to read decks: {}", e))?;
    let decks: serde_json::Value = serde_json::from_str(&decks_json)
        .map_err(|e| anyhow!("Failed to parse decks JSON: {}", e))?;

    let ids: Vec<i64> = decks
        .as_object()
        .into_iter()
        .flat_map(|map| map.values())
        .filter(|d| {
            let name = d.get("name").and_then(|n| n.as_str()).unwrap_or("");
            name == deck_name || name.starts_with(&format!("{}::", deck_name))
        })
        .filter_map(|d| d.get("id").and_then(|i| i.as_i64()))
        .collect();

    if ids.is_empty() {
        return Err(anyhow!("Deck '{}' not found in collection", deck_name));
    }
    Ok(ids)
}

pub fn query_revlog(
    conn: &Connection,
    next_day_at: i64,
    cutoff_ms: i64,
    deck_ids: Option<&[i64]>,
) -> Result<Vec<RevlogRow>> {
    let last_day_at = next_day_at - SECS_PER_DAY;

    let excluded_types = format!(
        "{}, {}",
        ReviewType::Rescheduled as i8,
        ReviewType::Manual as i8
    );
    let base_where = format!("type NOT IN ({excluded_types}) AND id >= {cutoff_ms}");

    let sql = match deck_ids {
        Some(ids) => {
            let id_list = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "SELECT r.id, r.type, r.lastIvl FROM revlog r
                 JOIN cards c ON c.id = r.cid
                 WHERE r.{base_where} AND c.did IN ({id_list})"
            )
        }
        None => format!("SELECT id, type, lastIvl FROM revlog WHERE {base_where}"),
    };

    conn.prepare(&sql)
        .map_err(|e| anyhow!("SQL prepare error: {}", e))?
        .query_map([], |row| {
            let id_ms: i64 = row.get(0)?;
            let rev_type: i32 = row.get(1)?;
            let last_ivl: i32 = row.get(2)?;
            let day_offset = (id_ms / 1000 - last_day_at).div_euclid(SECS_PER_DAY) as i32;
            Ok((day_offset, rev_type, last_ivl))
        })
        .map_err(|e| anyhow!("SQL query error: {}", e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| anyhow!("SQL row error: {}", e))
}

pub fn aggregate_reviews_by_day(rows: &[RevlogRow]) -> HashMap<i32, DayReviews> {
    let mut map: HashMap<i32, DayReviews> = HashMap::new();
    for &(anki_day, rev_type, last_ivl) in rows {
        let entry = map.entry(anki_day).or_default();
        match rev_type {
            t if t == ReviewType::Learn as i32 => entry.learn += 1,
            t if t == ReviewType::Review as i32 && last_ivl < MATURE_INTERVAL_DAYS => {
                entry.young += 1
            }
            t if t == ReviewType::Review as i32 => entry.mature += 1,
            t if t == ReviewType::Relearn as i32 => entry.relearn += 1,
            t if t == ReviewType::Filtered as i32 => entry.filtered += 1,
            _ => {}
        }
    }
    map
}
