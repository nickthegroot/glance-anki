use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub collection_path: String,
    pub port: u16,
    pub days: u32,
    pub fg: String,
    pub bg: String,
    pub svg_height: String,
    pub cell_radius: u32,
    pub weekday_labels: Vec<(usize, &'static str)>,
    pub transition_hue: bool,
    pub font_size: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            collection_path: env::var("ANKI_COLLECTION_PATH")
                .unwrap_or_else(|_| "collection.anki2".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            days: env::var("DEFAULT_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            fg: "#40c463".to_string(),
            bg: "#ebedf0".to_string(),
            svg_height: "110".to_string(),
            cell_radius: 2,
            weekday_labels: vec![(1, "Mon"), (3, "Wed"), (5, "Fri")],
            transition_hue: false,
            font_size: "12".to_string(),
        }
    }
}
