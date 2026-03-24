pub mod types;

mod db;
mod scheduling;
mod stats;

pub use types::AnkiStats;

use anyhow::Result;

use types::SECS_PER_DAY;

pub fn fetch_anki_stats(
    collection_path: &str,
    deck: Option<&str>,
    day_count: u32,
    timezone: Option<&str>,
) -> Result<AnkiStats> {
    let conn = db::open_collection_readonly(collection_path)?;

    let rollover_hour = db::read_rollover_hour(&conn);
    let next_day_at = scheduling::next_rollover_timestamp(rollover_hour, timezone);
    let window_start_ms = (next_day_at - (day_count as i64 + 1) * SECS_PER_DAY) * 1000;

    let deck = deck.filter(|d| !d.is_empty());
    let deck_ids = deck
        .map(|name| db::resolve_deck_ids(&conn, name))
        .transpose()?;

    let revlog_rows = db::query_revlog(&conn, next_day_at, window_start_ms, deck_ids.as_deref())?;
    let reviews_by_day = db::aggregate_reviews_by_day(&revlog_rows);

    let daily_entries =
        stats::build_daily_entries(day_count, &reviews_by_day, next_day_at, rollover_hour);

    let daily_reviews: Vec<(String, u32, String)> = daily_entries
        .iter()
        .map(|e| (e.date_str(), e.reviews.total(), e.hover_label()))
        .collect();

    Ok(AnkiStats {
        deck: deck
            .map(str::to_string)
            .unwrap_or_else(|| "All Decks".to_string()),
        days: day_count,
        daily_reviews,
    })
}
