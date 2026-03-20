use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, web};
use askama::Template;
use log::{error, info};
use std::collections::HashMap;

use crate::anki::fetch_anki_stats;
use crate::color;
use crate::config::Config;
use crate::templates::{AnkiGraphHtmlTemplate, AnkiStatsTemplate, AnkiSvgGraphTemplate, GraphCell};

fn parse_params(req: &HttpRequest) -> HashMap<String, String> {
    url::form_urlencoded::parse(req.query_string().as_bytes())
        .into_owned()
        .collect()
}

fn param_str<'a>(params: &'a HashMap<String, String>, key: &str, default: &'a str) -> String {
    params
        .get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn param_bool(params: &HashMap<String, String>, key: &str, default: bool) -> bool {
    params
        .get(key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn deck_and_days(params: &HashMap<String, String>, config: &Config) -> (Option<String>, u32) {
    let deck = params.get("deck").cloned().filter(|d| !d.is_empty());
    let days = params
        .get("days")
        .and_then(|v| v.parse().ok())
        .unwrap_or(config.days);
    (deck, days)
}

fn build_svg_template<'a>(
    stats: &'a crate::anki::AnkiStats,
    params: &HashMap<String, String>,
    config: &Config,
) -> AnkiSvgGraphTemplate<'a> {
    let primary_color = param_str(params, "primary-color", &config.fg);
    let bg_color = param_str(params, "background-color", &config.bg);
    let color_shades = color::derive_color_shades_with_bg(
        &primary_color,
        &bg_color,
        param_bool(params, "transition-hue", config.transition_hue),
    );

    const ROWS: usize = 7;
    let cells = stats
        .daily_reviews
        .iter()
        .enumerate()
        .map(|(i, (date, count, label))| {
            let color = match count {
                c if *c > 15 => color_shades[4].clone(),
                c if *c > 8 => color_shades[3].clone(),
                c if *c > 4 => color_shades[2].clone(),
                c if *c > 0 => color_shades[1].clone(),
                _ => color_shades[0].clone(),
            };
            GraphCell {
                date: date.clone(),
                count: *count,
                col: i / ROWS,
                row: i % ROWS,
                color,
                hover_text: label.clone(),
            }
        })
        .collect();

    let mut month_labels: Vec<(usize, String)> = Vec::new();
    let mut last_month = String::new();
    for (i, (date, _, _)) in stats.daily_reviews.iter().enumerate() {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            let month = d.format("%b").to_string();
            if month != last_month {
                month_labels.push((i / ROWS, month.clone()));
                last_month = month;
            }
        }
    }

    AnkiSvgGraphTemplate {
        stats,
        max_count: stats
            .daily_reviews
            .iter()
            .map(|(_, c, _)| *c)
            .max()
            .unwrap_or(0),
        cells,
        primary_color,
        color_shades,
        month_labels,
        weekday_labels: config.weekday_labels.clone(),
        svg_height: param_str(params, "svg-height", &config.svg_height),
        cell_radius: config.cell_radius,
        font_size: param_str(params, "font-size", &config.font_size),
    }
}

fn widget_title(deck: &str) -> String {
    format!("Anki – {}", deck)
}

fn quartiles_string(quartiles: &[u32; 5]) -> String {
    quartiles
        .iter()
        .map(|q| q.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

async fn load_stats(
    config: &Config,
    deck: Option<String>,
    days: u32,
) -> Result<crate::anki::AnkiStats, String> {
    let collection_path = config.collection_path.clone();
    tokio::task::spawn_blocking(move || fetch_anki_stats(&collection_path, deck.as_deref(), days))
        .await
        .map_err(|e| format!("Internal error: {}", e))?
        .map_err(|e| format!("Error: {}", e))
}

pub async fn run_api_server() -> std::io::Result<()> {
    let config = Config::from_env();
    info!(
        "Starting on 0.0.0.0:{} (collection: {})",
        config.port, config.collection_path
    );

    let port = config.port;
    HttpServer::new(|| {
        App::new()
            .route("/stats", web::get().to(stats_handler))
            .route("/graph_svg", web::get().to(svg_graph_handler))
            .route("/graph", web::get().to(graph_html_handler))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

async fn stats_handler(req: HttpRequest) -> impl Responder {
    let params = parse_params(&req);
    let config = Config::from_env();
    let (deck, days) = deck_and_days(&params, &config);
    let show_quartiles = param_bool(&params, "show_quartiles", true);

    info!("GET /stats deck={:?} days={}", deck, days);

    match load_stats(&config, deck, days).await {
        Ok(stats) => {
            let template = AnkiStatsTemplate {
                show_quartiles,
                quartiles_string: quartiles_string(&stats.quartiles),
                stats: &stats,
            };
            match template.render() {
                Ok(body) => HttpResponse::Ok()
                    .content_type("text/html")
                    .insert_header(("Widget-Title", widget_title(&stats.deck).as_str()))
                    .insert_header(("Widget-Content-Type", "html"))
                    .body(body),
                Err(e) => {
                    error!("Template error: {}", e);
                    HttpResponse::InternalServerError().body(e.to_string())
                }
            }
        }
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

async fn svg_graph_handler(req: HttpRequest) -> impl Responder {
    let params = parse_params(&req);
    let config = Config::from_env();
    let (deck, days) = deck_and_days(&params, &config);

    info!("GET /graph_svg deck={:?} days={}", deck, days);

    match load_stats(&config, deck, days).await {
        Ok(stats) => {
            let template = build_svg_template(&stats, &params, &config);
            match template.render() {
                Ok(body) => HttpResponse::Ok()
                    .content_type("image/svg+xml")
                    .insert_header(("Widget-Title", widget_title(&stats.deck).as_str()))
                    .insert_header(("Widget-Content-Type", "html"))
                    .body(body),
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}

async fn graph_html_handler(req: HttpRequest) -> impl Responder {
    let params = parse_params(&req);
    let config = Config::from_env();
    let (deck, days) = deck_and_days(&params, &config);

    info!("GET /graph deck={:?} days={}", deck, days);

    match load_stats(&config, deck, days).await {
        Ok(stats) => {
            let svg = build_svg_template(&stats, &params, &config);
            let template = AnkiGraphHtmlTemplate {
                quartiles: quartiles_string(&stats.quartiles),
                svg,
            };
            match template.render() {
                Ok(body) => HttpResponse::Ok()
                    .content_type("text/html")
                    .insert_header(("Widget-Title", widget_title(&stats.deck).as_str()))
                    .insert_header(("Widget-Content-Type", "html"))
                    .body(body),
                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
            }
        }
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().body(e)
        }
    }
}
