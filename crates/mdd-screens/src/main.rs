use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Screen {
    name: String,
    path: String,
    description: String,
}

#[derive(Debug)]
struct Group {
    label: String,
    screens: Vec<Screen>,
}

#[derive(Debug)]
enum Item {
    Screen(Screen),
    Group(Group),
}

#[derive(Debug)]
struct ScreenSpec {
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ScreenSpec, String> {
    let mut items: Vec<Item> = Vec::new();
    let mut current_group: Option<(String, Vec<Screen>)> = None;

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
            if let Some((label, screens)) = current_group.take() {
                items.push(Item::Group(Group { label, screens }));
                continue;
            }
            return Err("Unexpected }".to_string());
        }

        if let Some(screen) = parse_screen(t)? {
            if let Some((_, ref mut screens)) = current_group {
                screens.push(screen);
            } else {
                items.push(Item::Screen(screen));
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", t));
    }

    if current_group.is_some() {
        return Err("Unclosed group block".to_string());
    }
    if items.is_empty() {
        return Err("At least 1 screen required".to_string());
    }

    Ok(ScreenSpec { items })
}

/// Parse: /path 画面名 : "説明"
fn parse_screen(line: &str) -> Result<Option<Screen>, String> {
    if !line.starts_with('/') {
        return Ok(None);
    }

    let (path, rest) = match line.split_once(' ') {
        Some((p, r)) => (p.trim().to_string(), r.trim()),
        None => return Ok(Some(Screen {
            path: line.to_string(),
            name: String::new(),
            description: String::new(),
        })),
    };

    let (name, description) = if let Some((n, d)) = rest.split_once(" : ") {
        (n.trim().to_string(), strip_quotes(d.trim()).to_string())
    } else {
        (rest.to_string(), String::new())
    };

    Ok(Some(Screen { name, path, description }))
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
const MARGIN: f64 = 12.0;
const PATH_MARGIN: f64 = 12.0;
const NAME_MARGIN: f64 = 14.0;
const DESC_MARGIN: f64 = 16.0;
const PAD: f64 = 24.0;
const GROUP_HEADER_H: f64 = 32.0;
const GROUP_GAP: f64 = 16.0;
const GROUP_PAD: f64 = 12.0;
const COL_GAP: f64 = 20.0;
const NUM_COLS: usize = 2;

const COLOR_DARK: &str = "#333";
const COLOR_PATH: &str = "#1565c0";
const COLOR_NAME: &str = "#333";
const COLOR_DESC: &str = "#666";
const COLOR_GROUP_BG: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#e0e0e0";
const COLOR_GROUP_TEXT: &str = "#555";
const COLOR_ROW_BG: &str = "#ffffff";
const COLOR_ROW_STROKE: &str = "#eee";
const COLOR_ICON: &str = "#90caf9";

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

const ICON_W: f64 = 20.0;

fn item_content_width(item: &Item) -> f64 {
    let screens = match item {
        Item::Screen(s) => std::slice::from_ref(s),
        Item::Group(g) => &g.screens,
    };

    let mut max_path: f64 = 0.0;
    let mut max_name: f64 = 0.0;
    let mut max_desc: f64 = 0.0;
    for s in screens {
        max_path = max_path.max(mono_width(&s.path));
        max_name = max_name.max(text_width(&s.name));
        max_desc = max_desc.max(if s.description.is_empty() {
            0.0
        } else {
            text_width(&s.description)
        });
    }

    let label_w = match item {
        Item::Group(g) => text_width(&g.label) + 24.0,
        _ => 0.0,
    };

    let row_w = MARGIN + ICON_W + PATH_MARGIN + max_path
        + if max_name > 0.0 { NAME_MARGIN + max_name } else { 0.0 }
        + if max_desc > 0.0 { DESC_MARGIN + max_desc } else { 0.0 }
        + MARGIN;

    row_w.max(label_w)
}

fn item_height(item: &Item) -> f64 {
    match item {
        Item::Screen(_) => ROW_H,
        Item::Group(g) => {
            let rows_h = g.screens.len() as f64 * ROW_H
                + (g.screens.len().saturating_sub(1)) as f64 * ROW_GAP;
            GROUP_HEADER_H + GROUP_PAD * 2.0 + rows_h
        }
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(spec: &ScreenSpec) -> String {
    let item_widths: Vec<f64> = spec.items.iter().map(|i| item_content_width(i)).collect();
    let item_heights: Vec<f64> = spec.items.iter().map(|i| item_height(i)).collect();

    let n = spec.items.len();
    let use_cols = if n <= 1 { 1 } else { NUM_COLS.min(n) };

    let mut col_items: Vec<Vec<usize>> = vec![vec![]; use_cols];
    let mut col_heights: Vec<f64> = vec![0.0; use_cols];

    for i in 0..n {
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
                Item::Screen(s) => {
                    render_screen_row(&mut svg, s, col_x, cur_y, cw);
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
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x, y, width, height, COLOR_GROUP_BG, COLOR_GROUP_STROKE
    ));

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" \
         fill=\"{}\">{}</text>",
        x + 12.0,
        y + GROUP_HEADER_H * 0.7,
        COLOR_GROUP_TEXT,
        escape_xml(&g.label)
    ));

    let mut row_y = y + GROUP_HEADER_H + GROUP_PAD;
    for s in &g.screens {
        render_screen_row(svg, s, x, row_y, width);
        row_y += ROW_H + ROW_GAP;
    }
}

fn render_screen_row(svg: &mut String, screen: &Screen, x: f64, y: f64, width: f64) {
    // Row background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
        x + 4.0, y, width - 8.0, ROW_H, COLOR_ROW_BG, COLOR_ROW_STROKE
    ));

    // Screen icon (small window)
    let icon_x = x + MARGIN;
    let icon_y = y + (ROW_H - 14.0) / 2.0;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"14\" rx=\"2\" \
         fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        icon_x, icon_y, COLOR_ICON
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        icon_x, icon_y + 4.5, icon_x + 16.0, icon_y + 4.5, COLOR_ICON
    ));

    // Path (monospace, blue)
    let path_x = x + MARGIN + ICON_W + PATH_MARGIN;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" \
         font-weight=\"bold\" fill=\"{}\">{}</text>",
        path_x,
        y + ROW_H / 2.0 + MONO_FONT_SIZE * 0.35,
        MONO_FONT_SIZE,
        COLOR_PATH,
        escape_xml(&screen.path)
    ));

    // Screen name
    let mut next_x = path_x + mono_width(&screen.path);
    if !screen.name.is_empty() {
        next_x += NAME_MARGIN;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            next_x,
            y + ROW_H / 2.0 + FONT_SIZE * 0.35,
            COLOR_NAME,
            escape_xml(&screen.name)
        ));
        next_x += text_width(&screen.name);
    }

    // Description
    if !screen.description.is_empty() {
        next_x += DESC_MARGIN;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            next_x,
            y + ROW_H / 2.0 + 4.0,
            COLOR_DESC,
            escape_xml(&screen.description)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-screens - Render a screen list as SVG

Usage: mdd-screens < input.txt

Define screens with URL path, name, and optional description.
Group screens by section with \"group\".

Syntax:
  /path  Screen Name  : \"description\"
  /path  Screen Name
  /path

  group \"Section\" {
    /path  Screen Name : \"description\"
  }

Example:
  group \"認証\" {
    /login  ログイン : \"メール・パスワード認証\"
  }
  group \"管理\" {
    /dashboard  ダッシュボード : \"トップページ\"
    /users      ユーザー管理
  }
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
            eprintln!("mdd-screens: {}", e);
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
        let input = "/login ログイン : \"メール認証\"\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            Item::Screen(sc) => {
                assert_eq!(sc.path, "/login");
                assert_eq!(sc.name, "ログイン");
                assert_eq!(sc.description, "メール認証");
            }
            _ => panic!("Expected Screen"),
        }
    }

    #[test]
    fn parse_no_description() {
        let input = "/users ユーザー管理\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Screen(sc) => {
                assert_eq!(sc.name, "ユーザー管理");
                assert!(sc.description.is_empty());
            }
            _ => panic!("Expected Screen"),
        }
    }

    #[test]
    fn parse_path_only() {
        let input = "/health\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Screen(sc) => {
                assert_eq!(sc.path, "/health");
                assert!(sc.name.is_empty());
            }
            _ => panic!("Expected Screen"),
        }
    }

    #[test]
    fn parse_group() {
        let input = "group \"Auth\" {\n  /login ログイン\n  /register 新規登録\n}\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Group(g) => {
                assert_eq!(g.label, "Auth");
                assert_eq!(g.screens.len(), 2);
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn parse_unclosed_group() {
        assert!(parse("group \"X\" {\n  /x Xページ\n").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "/test テスト : \"テスト画面\"\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("/test"));
    }

    #[test]
    fn render_grouped() {
        let input = "group \"管理\" {\n  /dashboard ダッシュボード\n  /users ユーザー\n}\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.contains("管理"));
        assert!(svg.contains("/dashboard"));
    }
}
