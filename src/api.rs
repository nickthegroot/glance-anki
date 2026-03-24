use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use askama::Template;
use chrono::Datelike;
use log::{error, info};
use std::collections::HashMap;

use crate::anki::AnkiStats;
use crate::anki::fetch_anki_stats;
use crate::config::Config;
use crate::templates::{AnkiGraphHtmlTemplate, AnkiSvgGraphTemplate, GraphCell};

type QueryParams = web::Query<HashMap<String, String>>;

// ---------------------------------------------------------------------------
// Request parameters
// ---------------------------------------------------------------------------

struct WidgetParams {
    deck: Option<String>,
    days: u32,
    timezone: Option<String>,
}

impl WidgetParams {
    fn from_query(params: &HashMap<String, String>, config: &Config) -> Self {
        Self {
            deck: params.get("deck").cloned().filter(|d| !d.is_empty()),
            days: params
                .get("days")
                .and_then(|v| v.parse().ok())
                .unwrap_or(config.default_days),
            timezone: params.get("timezone").cloned().filter(|s| !s.is_empty()),
        }
    }
}

// ---------------------------------------------------------------------------
// Stats loading
// ---------------------------------------------------------------------------

async fn load_stats(config: &Config, params: &WidgetParams) -> Result<AnkiStats, String> {
    let collection_path = config.collection_path.clone();
    let deck = params.deck.clone();
    let days = params.days;
    let timezone = params.timezone.clone();

    tokio::task::spawn_blocking(move || {
        fetch_anki_stats(&collection_path, deck.as_deref(), days, timezone.as_deref())
    })
    .await
    .map_err(|e| format!("Internal error: {}", e))?
    .map_err(|e| format!("Error: {}", e))
}

// ---------------------------------------------------------------------------
// Graph building
// ---------------------------------------------------------------------------

const GRID_ROWS: usize = 7;
const CELL_SIZE: usize = 12;
const CELL_GAP: usize = 2;
const LABEL_OFFSET: usize = 30;
const TOP_OFFSET: usize = 20;
const MIN_CELL_OPACITY: f32 = 0.15;
const DEFAULT_CELL_RADIUS: u32 = 2;
const WEEKDAY_LABELS: &[(usize, &str)] = &[(1, "Mon"), (3, "Wed"), (5, "Fri")];

fn first_date_weekday(stats: &AnkiStats) -> usize {
    stats
        .daily_reviews
        .first()
        .and_then(|(date, _, _)| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .map(|d| d.weekday().num_days_from_sunday() as usize)
        .unwrap_or(0)
}

fn build_graph_cells(stats: &AnkiStats) -> Vec<GraphCell> {
    let max_count = stats
        .daily_reviews
        .iter()
        .map(|(_, c, _)| *c)
        .max()
        .unwrap_or(0);

    let first_weekday = first_date_weekday(stats);

    stats
        .daily_reviews
        .iter()
        .enumerate()
        .map(|(i, (date, count, label))| {
            let opacity = cell_opacity(*count, max_count);
            let row = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|d| d.weekday().num_days_from_sunday() as usize)
                .unwrap_or(i % GRID_ROWS);
            GraphCell {
                date: date.clone(),
                count: *count,
                col: (first_weekday + i) / GRID_ROWS,
                row,
                opacity,
                hover_text: label.clone(),
            }
        })
        .collect()
}

fn cell_opacity(count: u32, max_count: u32) -> String {
    if count == 0 || max_count == 0 {
        return String::new();
    }
    let opacity = MIN_CELL_OPACITY + (1.0 - MIN_CELL_OPACITY) * (count as f32 / max_count as f32);
    format!("{:.3}", opacity)
}

fn build_month_labels(stats: &AnkiStats) -> Vec<(usize, String)> {
    let first_weekday = first_date_weekday(stats);
    let mut labels: Vec<(usize, String)> = Vec::new();
    let mut last_month = String::new();
    for (i, (date, _, _)) in stats.daily_reviews.iter().enumerate() {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            let month = d.format("%b").to_string();
            if month != last_month {
                labels.push(((first_weekday + i) / GRID_ROWS, month.clone()));
                last_month = month;
            }
        }
    }
    labels
}

fn build_svg_template(stats: &AnkiStats) -> AnkiSvgGraphTemplate {
    let cells = build_graph_cells(stats);
    let num_cols = cells.iter().map(|c| c.col).max().unwrap_or(0) + 1;
    let viewbox_width = num_cols * (CELL_SIZE + CELL_GAP) + LABEL_OFFSET;
    let viewbox_height = TOP_OFFSET + GRID_ROWS * (CELL_SIZE + CELL_GAP);

    AnkiSvgGraphTemplate {
        cells,
        viewbox_width,
        viewbox_height,
        month_labels: build_month_labels(stats),
        weekday_labels: WEEKDAY_LABELS.to_vec(),
        cell_radius: DEFAULT_CELL_RADIUS,
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn widget_title(deck: &str) -> String {
    format!("Anki – {}", deck)
}

fn html_response(body: String, deck: &str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .insert_header(("Widget-Title", widget_title(deck).as_str()))
        .insert_header(("Widget-Content-Type", "html"))
        .body(body)
}

fn render_or_500<T: Template>(template: T) -> Result<String, HttpResponse> {
    template
        .render()
        .map_err(|e| HttpResponse::InternalServerError().body(e.to_string()))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn graph_html_handler(params: QueryParams) -> impl Responder {
    let config = Config::from_env();
    let widget_params = WidgetParams::from_query(&params, &config);
    info!(
        "GET /graph deck={:?} days={}",
        widget_params.deck, widget_params.days
    );

    match load_stats(&config, &widget_params).await {
        Ok(stats) => {
            let svg = build_svg_template(&stats);
            match render_or_500(AnkiGraphHtmlTemplate { svg }) {
                Ok(body) => html_response(body, &stats.deck),
                Err(e) => e,
            }
        }
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

pub async fn run_api_server() -> std::io::Result<()> {
    let config = Config::from_env();
    info!(
        "Starting on 0.0.0.0:{} (collection: {})",
        config.port, config.collection_path
    );

    let port = config.port;
    HttpServer::new(|| App::new().route("/graph", web::get().to(graph_html_handler)))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
