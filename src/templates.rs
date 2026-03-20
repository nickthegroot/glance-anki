use askama::Template;

pub struct GraphCell {
    pub date: String,
    pub count: u32,
    pub col: usize,
    pub row: usize,
    pub color: String,
    pub hover_text: String,
}

#[derive(Template)]
#[template(path = "stats.html")]
pub struct AnkiStatsTemplate<'a> {
    pub stats: &'a crate::anki::AnkiStats,
    pub show_quartiles: bool,
    pub quartiles_string: String,
}

#[derive(Template)]
#[template(path = "svg_graph.svg")]
pub struct AnkiSvgGraphTemplate<'a> {
    pub stats: &'a crate::anki::AnkiStats,
    pub max_count: u32,
    pub cells: Vec<GraphCell>,
    pub svg_height: String,
    pub font_size: String,
    pub primary_color: String,
    pub color_shades: Vec<String>,
    pub month_labels: Vec<(usize, String)>,
    pub weekday_labels: Vec<(usize, &'static str)>,
    pub cell_radius: u32,
}

#[derive(Template)]
#[template(path = "graph.html")]
pub struct AnkiGraphHtmlTemplate<'a> {
    pub svg: AnkiSvgGraphTemplate<'a>,
    pub quartiles: String,
}
