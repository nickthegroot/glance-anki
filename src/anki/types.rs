use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum ReviewType {
    Learn = 0,
    Review = 1,
    Relearn = 2,
    Filtered = 3,
    Rescheduled = 4,
    Manual = 5,
}

pub const MATURE_INTERVAL_DAYS: i32 = 21;
pub const SECS_PER_DAY: i64 = 86400;

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Clone)]
pub struct AnkiStats {
    pub deck: String,
    pub days: u32,
    pub daily_reviews: Vec<(String, u32, String)>,
}
