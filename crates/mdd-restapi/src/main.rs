use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl Method {
    fn label(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
        }
    }

    fn colors(&self) -> (&'static str, &'static str) {
        match self {
            Method::Get => ("#e8f5e9", "#2e7d32"),
            Method::Post => ("#e3f2fd", "#1565c0"),
            Method::Put => ("#fff8e1", "#f57f17"),
            Method::Delete => ("#ffebee", "#c62828"),
            Method::Patch => ("#f3e5f5", "#7b1fa2"),
        }
    }
}

#[derive(Debug)]
struct Endpoint {
    method: Method,
    path: String,
    description: String,
}

#[derive(Debug)]
struct Group {
    label: String,
    endpoints: Vec<Endpoint>,
}

#[derive(Debug)]
enum Item {
    Endpoint(Endpoint),
    Group(Group),
}

#[derive(Debug)]
struct ApiSpec {
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ApiSpec, String> {
    let mut items: Vec<Item> = Vec::new();
    let mut current_group: Option<(String, Vec<Endpoint>)> = None;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        if t.starts_with("group ") {
            let rest = t.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let label = strip_quotes(name.trim()).to_string();
                current_group = Some((label, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", t));
        }

        if t == "}" {
            if let Some((label, endpoints)) = current_group.take() {
                items.push(Item::Group(Group { label, endpoints }));
                continue;
            }
            return Err("Unexpected }".to_string());
        }

        if let Some(endpoint) = parse_endpoint(t)? {
            if let Some((_, ref mut endpoints)) = current_group {
                endpoints.push(endpoint);
            } else {
                items.push(Item::Endpoint(endpoint));
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", t));
    }

    if current_group.is_some() {
        return Err("Unclosed group block".to_string());
    }
    if items.is_empty() {
        return Err("At least 1 endpoint required".to_string());
    }

    Ok(ApiSpec { items })
}

fn parse_endpoint(line: &str) -> Result<Option<Endpoint>, String> {
    let methods = [
        ("GET ", Method::Get),
        ("POST ", Method::Post),
        ("PUT ", Method::Put),
        ("DELETE ", Method::Delete),
        ("PATCH ", Method::Patch),
    ];

    for (prefix, method) in &methods {
        if line.starts_with(prefix) {
            let rest = line.strip_prefix(prefix).unwrap().trim();
            let (path, description) = if let Some((p, d)) = rest.split_once(" : ") {
                (p.trim().to_string(), strip_quotes(d.trim()).to_string())
            } else {
                (rest.to_string(), String::new())
            };
            return Ok(Some(Endpoint {
                method: *method,
                path,
                description,
            }));
        }
    }

    Ok(None)
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const MONO_CW: f64 = 7.8;
const FONT_SIZE: f64 = 13.0;
const MONO_FONT_SIZE: f64 = 12.5;

const ROW_H: f64 = 36.0;
const ROW_GAP: f64 = 4.0;
const BADGE_W: f64 = 60.0;
const BADGE_H: f64 = 22.0;
const BADGE_MARGIN: f64 = 12.0;
const PATH_MARGIN: f64 = 12.0;
const DESC_MARGIN: f64 = 16.0;
const PAD: f64 = 24.0;
const GROUP_HEADER_H: f64 = 32.0;
const GROUP_GAP: f64 = 16.0;
const GROUP_PAD: f64 = 12.0;
const COL_GAP: f64 = 20.0;
const NUM_COLS: usize = 2;

const COLOR_DARK: &str = "#333";
const COLOR_PATH: &str = "#333";
const COLOR_DESC: &str = "#666";
const COLOR_GROUP_BG: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#e0e0e0";
const COLOR_GROUP_TEXT: &str = "#555";
const COLOR_ROW_BG: &str = "#ffffff";
const COLOR_ROW_STROKE: &str = "#eee";

// ---------------------------------------------------------------------------
// Sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CW } else { CJK })
        .sum()
}

fn mono_width(s: &str) -> f64 {
    s.len() as f64 * MONO_CW
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Compute the content width needed for a single item (group or endpoint).
fn item_content_width(item: &Item) -> f64 {
    let endpoints = match item {
        Item::Endpoint(ep) => std::slice::from_ref(ep),
        Item::Group(g) => &g.endpoints,
    };

    let mut max_path: f64 = 0.0;
    let mut max_desc: f64 = 0.0;
    for ep in endpoints {
        max_path = max_path.max(mono_width(&ep.path));
        max_desc = max_desc.max(if ep.description.is_empty() {
            0.0
        } else {
            text_width(&ep.description)
        });
    }

    // Also account for group label width
    let label_w = match item {
        Item::Group(g) => text_width(&g.label) + 24.0,
        _ => 0.0,
    };

    let row_w = BADGE_MARGIN + BADGE_W + PATH_MARGIN + max_path
        + if max_desc > 0.0 { DESC_MARGIN + max_desc } else { 0.0 }
        + BADGE_MARGIN;

    row_w.max(label_w)
}

/// Compute the height of an item.
fn item_height(item: &Item) -> f64 {
    match item {
        Item::Endpoint(_) => ROW_H,
        Item::Group(g) => {
            let rows_h = g.endpoints.len() as f64 * ROW_H
                + (g.endpoints.len().saturating_sub(1)) as f64 * ROW_GAP;
            GROUP_HEADER_H + GROUP_PAD * 2.0 + rows_h
        }
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(spec: &ApiSpec) -> String {
    // Compute per-item sizes
    let item_widths: Vec<f64> = spec.items.iter().map(|i| item_content_width(i)).collect();
    let item_heights: Vec<f64> = spec.items.iter().map(|i| item_height(i)).collect();

    let n = spec.items.len();

    // Single column if only 1 item or all ungrouped endpoints
    let use_cols = if n <= 1 {
        1
    } else {
        NUM_COLS.min(n)
    };

    // Assign items to columns greedily (shortest column first)
    let mut col_items: Vec<Vec<usize>> = vec![vec![]; use_cols];
    let mut col_heights: Vec<f64> = vec![0.0; use_cols];

    for i in 0..n {
        // Find column with minimum height
        let min_col = col_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        if !col_items[min_col].is_empty() {
            col_heights[min_col] += GROUP_GAP;
        }
        col_items[min_col].push(i);
        col_heights[min_col] += item_heights[i];
    }

    // Compute column widths (max of items in each column)
    let col_widths: Vec<f64> = col_items
        .iter()
        .map(|items| {
            items
                .iter()
                .map(|&i| item_widths[i])
                .fold(0.0_f64, f64::max)
        })
        .collect();

    let total_w = PAD * 2.0
        + col_widths.iter().sum::<f64>()
        + (use_cols.saturating_sub(1)) as f64 * COL_GAP;
    let max_col_h = col_heights.iter().copied().fold(0.0_f64, f64::max);
    let total_h = PAD * 2.0 + max_col_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Render each column
    let mut col_x = PAD;
    for (ci, items) in col_items.iter().enumerate() {
        let cw = col_widths[ci];
        let mut cur_y = PAD;

        for (idx, &item_idx) in items.iter().enumerate() {
            if idx > 0 {
                cur_y += GROUP_GAP;
            }

            let item = &spec.items[item_idx];
            match item {
                Item::Endpoint(ep) => {
                    render_endpoint_row(&mut svg, ep, col_x, cur_y, cw);
                    cur_y += ROW_H;
                }
                Item::Group(g) => {
                    let h = item_heights[item_idx];
                    render_group(&mut svg, g, col_x, cur_y, cw, h);
                    cur_y += h;
                }
            }
        }

        col_x += cw + COL_GAP;
    }

    svg.push_str("</svg>");
    svg
}

fn render_group(svg: &mut String, g: &Group, x: f64, y: f64, width: f64, height: f64) {
    // Group background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x, y, width, height, COLOR_GROUP_BG, COLOR_GROUP_STROKE
    ));

    // Group label
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" \
         fill=\"{}\">{}</text>",
        x + 12.0,
        y + GROUP_HEADER_H * 0.7,
        COLOR_GROUP_TEXT,
        escape_xml(&g.label)
    ));

    let mut row_y = y + GROUP_HEADER_H + GROUP_PAD;
    for ep in &g.endpoints {
        render_endpoint_row(svg, ep, x, row_y, width);
        row_y += ROW_H + ROW_GAP;
    }
}

fn render_endpoint_row(svg: &mut String, ep: &Endpoint, x: f64, y: f64, width: f64) {
    let (badge_bg, badge_fg) = ep.method.colors();

    // Row background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
        x + 4.0, y, width - 8.0, ROW_H, COLOR_ROW_BG, COLOR_ROW_STROKE
    ));

    // Method badge
    let badge_x = x + BADGE_MARGIN;
    let badge_y = y + (ROW_H - BADGE_H) / 2.0;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        badge_x, badge_y, BADGE_W, BADGE_H, badge_bg, badge_fg
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" \
         font-weight=\"bold\" fill=\"{}\">{}</text>",
        badge_x + BADGE_W / 2.0,
        badge_y + BADGE_H / 2.0 + 4.0,
        badge_fg,
        ep.method.label()
    ));

    // Path (monospace)
    let path_x = x + BADGE_MARGIN + BADGE_W + PATH_MARGIN;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" \
         fill=\"{}\">{}</text>",
        path_x,
        y + ROW_H / 2.0 + MONO_FONT_SIZE * 0.35,
        MONO_FONT_SIZE,
        COLOR_PATH,
        escape_xml(&ep.path)
    ));

    // Description
    if !ep.description.is_empty() {
        let desc_x = path_x + mono_width(&ep.path) + DESC_MARGIN;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            desc_x,
            y + ROW_H / 2.0 + 4.0,
            COLOR_DESC,
            escape_xml(&ep.description)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-restapi - Render a REST API reference as SVG

Usage: mdd-restapi < input.txt

Define endpoints with HTTP method, path, and optional description.
Group endpoints by resource with \"group\".

Syntax:
  GET    /path          : \"description\"
  POST   /path
  group \"Resource\" {
    GET  /resource      : \"list\"
    POST /resource      : \"create\"
  }

Supported methods: GET, POST, PUT, DELETE, PATCH
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let spec = match parse(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd-restapi: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&spec));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let input = "GET /api/users : \"ユーザー一覧\"\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            Item::Endpoint(ep) => {
                assert_eq!(ep.method, Method::Get);
                assert_eq!(ep.path, "/api/users");
                assert_eq!(ep.description, "ユーザー一覧");
            }
            _ => panic!("Expected Endpoint"),
        }
    }

    #[test]
    fn parse_no_description() {
        let input = "DELETE /api/users/:id\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Endpoint(ep) => {
                assert_eq!(ep.method, Method::Delete);
                assert!(ep.description.is_empty());
            }
            _ => panic!("Expected Endpoint"),
        }
    }

    #[test]
    fn parse_all_methods() {
        let input = "GET /a\nPOST /b\nPUT /c\nDELETE /d\nPATCH /e\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 5);
    }

    #[test]
    fn parse_group() {
        let input = "group \"Users\" {\n  GET /users\n  POST /users\n}\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            Item::Group(g) => {
                assert_eq!(g.label, "Users");
                assert_eq!(g.endpoints.len(), 2);
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn parse_mixed() {
        let input = "GET /health\ngroup \"Users\" {\n  GET /users\n}\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 2);
    }

    #[test]
    fn parse_unclosed_group() {
        assert!(parse("group \"X\" {\n  GET /x\n").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "GET /api/test : \"test\"\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("GET"));
    }

    #[test]
    fn render_grouped() {
        let input = "group \"Auth\" {\n  POST /login : \"ログイン\"\n  POST /logout\n}\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.contains("Auth"));
        assert!(svg.contains("POST"));
        assert!(svg.contains("/login"));
    }
}
