use chrono::{DateTime, Duration, Local, NaiveDate, Timelike};
use chrono_tz::Tz;

use super::types::SECS_PER_DAY;

pub fn next_rollover_timestamp(rollover_hour: u32, timezone: Option<&str>) -> i64 {
    if let Some(tz_str) = timezone.filter(|s| !s.is_empty()) {
        if let Ok(tz) = tz_str.parse::<Tz>() {
            return next_rollover_in_tz(rollover_hour, tz);
        }
    }
    next_rollover_in_tz(rollover_hour, Local)
}

fn next_rollover_in_tz<Z>(rollover_hour: u32, tz: Z) -> i64
where
    Z: chrono::TimeZone,
    Z::Offset: std::fmt::Display,
{
    let now = chrono::Utc::now().with_timezone(&tz);
    let rollover_today = now
        .with_hour(rollover_hour)
        .and_then(|t| t.with_minute(0))
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or_else(|| now.clone());

    if now < rollover_today {
        rollover_today.timestamp()
    } else {
        (rollover_today + Duration::days(1)).timestamp()
    }
}

pub fn anki_day_offset_to_date(day_offset: i32, next_day_at: i64, rollover_hour: u32) -> NaiveDate {
    let last_rollover = DateTime::from_timestamp(next_day_at - SECS_PER_DAY, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    (last_rollover + Duration::days(day_offset as i64) - Duration::hours(rollover_hour as i64))
        .date_naive()
}
