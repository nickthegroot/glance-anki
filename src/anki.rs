use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Local, NaiveDate, Timelike};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DayReviews {
    pub learn: u32,
    pub young: u32,
    pub mature: u32,
    pub relearn: u32,
    pub filtered: u32,
}

impl DayReviews {
    pub fn total(&self) -> u32 {
        self.learn + self.young + self.mature + self.relearn + self.filtered
    }
}

#[derive(Debug, Clone)]
pub struct DailyEntry {
    pub date: NaiveDate,
    pub reviews: DayReviews,
}

impl DailyEntry {
    pub fn date_str(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }

    pub fn hover_label(&self) -> String {
        let total = self.reviews.total();
        if total == 0 {
            return format!("{}: No reviews", self.date_str());
        }
        let r = &self.reviews;
        format!(
            "{}: {} reviews (Learn {}, Young {}, Mature {}, Relearn {}, Filtered {})",
            self.date_str(),
            total,
            r.learn,
            r.young,
            r.mature,
            r.relearn,
            r.filtered
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiStats {
    pub deck: String,
    pub total: u32,
    pub today: u32,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub high_score: u32,
    pub high_score_date: String,
    pub days: u32,
    pub daily_reviews: Vec<(String, u32, String)>,
    pub quartiles: [u32; 5],
}

fn next_rollover_timestamp(rollover_hour: u32) -> i64 {
    let now = Local::now();
    let rollover_today = now
        .with_hour(rollover_hour)
        .and_then(|t| t.with_minute(0))
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(now);

    if now < rollover_today {
        rollover_today.timestamp()
    } else {
        (rollover_today + Duration::days(1)).timestamp()
    }
}

fn anki_day_to_date(anki_day: i32, next_day_at: i64, rollover_hour: u32) -> NaiveDate {
    let rollover = DateTime::from_timestamp(next_day_at, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    (rollover + Duration::days(anki_day as i64) - Duration::hours(rollover_hour as i64))
        .date_naive()
}

fn resolve_deck_ids(conn: &Connection, deck_name: &str) -> Result<Vec<i64>> {
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

fn query_revlog(
    conn: &Connection,
    next_day_at: i64,
    cutoff_ms: i64,
    deck_ids: Option<&[i64]>,
) -> Result<Vec<(i32, i32, i32)>> {
    let day_expr = format!("CAST(({{table}}id / 1000 - {next_day_at}) / 86400 AS INTEGER)");
    let base_where = format!("type NOT IN (4, 5) AND id >= {cutoff_ms}");

    let sql = match deck_ids {
        Some(ids) => {
            let id_list = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "SELECT {day}, r.type, r.lastIvl FROM revlog r
                 JOIN cards c ON c.id = r.cid
                 WHERE r.{where} AND c.did IN ({id_list})",
                day = day_expr.replace("{table}", "r."),
                where = base_where,
            )
        }
        None => format!(
            "SELECT {day}, type, lastIvl FROM revlog WHERE {where}",
            day = day_expr.replace("{table}", ""),
            where = base_where,
        ),
    };

    conn.prepare(&sql)
        .map_err(|e| anyhow!("SQL prepare error: {}", e))?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| anyhow!("SQL query error: {}", e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| anyhow!("SQL row error: {}", e))
}

fn accumulate_day_map(rows: &[(i32, i32, i32)]) -> HashMap<i32, DayReviews> {
    let mut map: HashMap<i32, DayReviews> = HashMap::new();
    for &(anki_day, rev_type, last_ivl) in rows {
        let entry = map.entry(anki_day).or_default();
        match rev_type {
            0 => entry.learn += 1,
            1 if last_ivl < 21 => entry.young += 1,
            1 => entry.mature += 1,
            2 => entry.relearn += 1,
            3 => entry.filtered += 1,
            _ => {}
        }
    }
    map
}

fn compute_streaks(counts: &[u32]) -> (u32, u32) {
    let mut current_streak = 0u32;
    let mut longest_streak = 0u32;
    let mut streak = 0u32;
    let mut past_first_zero = false;

    for &count in counts.iter().rev() {
        if count > 0 {
            streak += 1;
            longest_streak = longest_streak.max(streak);
        } else {
            if !past_first_zero {
                current_streak = streak;
                past_first_zero = true;
            }
            streak = 0;
        }
    }
    if !past_first_zero {
        current_streak = streak;
    }
    (current_streak, longest_streak)
}

fn compute_quartiles(counts: &[u32]) -> [u32; 5] {
    let mut sorted = counts.to_vec();
    sorted.sort();
    let n = sorted.len();
    [
        *sorted.first().unwrap_or(&0),
        *sorted.get(n / 4).unwrap_or(&0),
        *sorted.get(n / 2).unwrap_or(&0),
        *sorted.get(3 * n / 4).unwrap_or(&0),
        *sorted.last().unwrap_or(&0),
    ]
}

pub fn fetch_anki_stats(collection_path: &str, deck: Option<&str>, days: u32) -> Result<AnkiStats> {
    // Open via URI with immutable=1 so SQLite skips all locking and WAL-index
    // writes — safe because we never modify the database.
    let uri = format!("file:{}?immutable=1", collection_path);
    let conn = Connection::open_with_flags(
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
    })?;

    let rollover_hour: u32 = conn
        .query_row(
            "SELECT val FROM config WHERE key = 'rollover' LIMIT 1",
            [],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| v.as_u64())
        .or_else(|| {
            // Legacy path: rollover stored inside the col.conf JSON blob.
            conn.query_row("SELECT conf FROM col LIMIT 1", [], |row| {
                row.get::<_, String>(0)
            })
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("rollover").and_then(|r| r.as_u64()))
        })
        .unwrap_or(4) as u32;

    let next_day_at = next_rollover_timestamp(rollover_hour);
    let cutoff_ms = (next_day_at - (days as i64 + 1) * 86_400) * 1000;
    let deck = deck.filter(|d| !d.is_empty());

    let deck_ids = deck.map(|name| resolve_deck_ids(&conn, name)).transpose()?;
    let rows = query_revlog(&conn, next_day_at, cutoff_ms, deck_ids.as_deref())?;
    let day_map = accumulate_day_map(&rows);

    let daily_reviews: Vec<(String, u32, String)> = (-(days as i32) + 1..=0)
        .map(|offset| {
            let entry = DailyEntry {
                date: anki_day_to_date(offset, next_day_at, rollover_hour),
                reviews: day_map.get(&offset).cloned().unwrap_or_default(),
            };
            (entry.date_str(), entry.reviews.total(), entry.hover_label())
        })
        .collect();

    let counts: Vec<u32> = daily_reviews.iter().map(|(_, c, _)| *c).collect();
    let total = counts.iter().sum();
    let today = counts.last().copied().unwrap_or(0);

    let (high_score_date, high_score) = daily_reviews
        .iter()
        .max_by_key(|(_, c, _)| c)
        .map(|(d, c, _)| (d.clone(), *c))
        .unwrap_or_default();

    let (current_streak, longest_streak) = compute_streaks(&counts);
    let quartiles = compute_quartiles(&counts);

    Ok(AnkiStats {
        deck: deck
            .map(str::to_string)
            .unwrap_or_else(|| "All Decks".to_string()),
        total,
        today,
        current_streak,
        longest_streak,
        high_score,
        high_score_date,
        days,
        daily_reviews,
        quartiles,
    })
}
