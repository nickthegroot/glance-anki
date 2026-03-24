use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub collection_path: String,
    pub port: u16,
    pub default_days: u32,
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
            default_days: env::var("DEFAULT_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}
