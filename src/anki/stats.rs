use std::collections::HashMap;

use super::scheduling::anki_day_offset_to_date;
use super::types::{DailyEntry, DayReviews};

pub fn build_daily_entries(
    day_count: u32,
    day_map: &HashMap<i32, DayReviews>,
    next_day_at: i64,
    rollover_hour: u32,
) -> Vec<DailyEntry> {
    (-(day_count as i32) + 1..=0)
        .map(|offset| DailyEntry {
            date: anki_day_offset_to_date(offset, next_day_at, rollover_hour),
            reviews: day_map.get(&offset).cloned().unwrap_or_default(),
        })
        .collect()
}
