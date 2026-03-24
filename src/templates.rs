use askama::Template;

pub struct GraphCell {
    pub date: String,
    pub count: u32,
    pub col: usize,
    pub row: usize,
    pub opacity: String,
    pub hover_text: String,
}

#[derive(Template)]
#[template(path = "svg_graph.svg")]
pub struct AnkiSvgGraphTemplate {
    pub cells: Vec<GraphCell>,
    pub viewbox_width: usize,
    pub viewbox_height: usize,
    pub month_labels: Vec<(usize, String)>,
    pub weekday_labels: Vec<(usize, &'static str)>,
    pub cell_radius: u32,
}

#[derive(Template)]
#[template(path = "graph.html")]
pub struct AnkiGraphHtmlTemplate {
    pub svg: AnkiSvgGraphTemplate,
}
