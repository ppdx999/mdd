use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GridItem {
    label: String,
    description: Option<String>,
}

#[derive(Debug)]
struct ListGrid {
    columns: usize,
    items: Vec<GridItem>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ListGrid, String> {
    let mut columns: usize = DEFAULT_COLUMNS;
    let mut items: Vec<GridItem> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // columns N
        if trimmed.starts_with("columns ") {
            let rest = trimmed.strip_prefix("columns ").unwrap().trim();
            columns = rest
                .parse::<usize>()
                .map_err(|_| format!("Invalid columns value: {}", rest))?;
            if columns == 0 {
                return Err("columns must be at least 1".to_string());
            }
            continue;
        }

        // item "Label" : "Description" or item "Label"
        if trimmed.starts_with("item ") {
            let rest = trimmed.strip_prefix("item ").unwrap().trim();
            let item = parse_item(rest)?;
            items.push(item);
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if items.is_empty() {
        return Err("At least 1 item is required".to_string());
    }

    Ok(ListGrid {
        columns,
        items,
    })
}

fn parse_item(s: &str) -> Result<GridItem, String> {
    // item "Label" : "Description" or item "Label"
    if !s.starts_with('"') {
        return Err(format!("Expected quoted label, got: {}", s));
    }

    let end_quote = s[1..]
        .find('"')
        .ok_or("Unterminated quote in item label")?;
    let label = s[1..=end_quote].to_string();
    let rest = s[end_quote + 2..].trim();

    if rest.is_empty() {
        return Ok(GridItem {
            label,
            description: None,
        });
    }

    // expect : "Description"
    if rest.starts_with(':') {
        let desc_part = rest[1..].trim();
        let desc = strip_quotes(desc_part).to_string();
        return Ok(GridItem {
            label,
            description: Some(desc),
        });
    }

    Err(format!("Unexpected content after label: {}", rest))
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;
const COLOR_DARK: &str = "#333";

const CARD_MIN_WIDTH: f64 = 160.0;
const CARD_H_PAD: f64 = 16.0;
const CARD_MIN_HEIGHT: f64 = 60.0;
const CARD_GAP: f64 = 12.0;
const LEFT_ACCENT_WIDTH: f64 = 4.0;
const DEFAULT_COLUMNS: usize = 3;
const PADDING: f64 = 40.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"),
    ("#e8f5e9", "#2e7d32"),
    ("#fff8e1", "#f57f17"),
    ("#f3e5f5", "#7b1fa2"),
    ("#e0f2f1", "#00695c"),
    ("#fce4ec", "#c62828"),
    ("#e8eaf6", "#283593"),
    ("#fff3e0", "#e65100"),
];

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(grid: &ListGrid) -> String {
    let cols = grid.columns;
    let rows = (grid.items.len() + cols - 1) / cols;

    // Compute card width based on content
    let max_label_w = grid
        .items
        .iter()
        .map(|item| text_width(&item.label))
        .fold(0.0_f64, f64::max);
    let max_desc_w = grid
        .items
        .iter()
        .filter_map(|item| item.description.as_ref())
        .map(|desc| text_width(desc) * (DESC_FONT_SIZE / FONT_SIZE))
        .fold(0.0_f64, f64::max);
    let content_w = max_label_w.max(max_desc_w);
    let card_w = (content_w + CARD_H_PAD * 2.0 + LEFT_ACCENT_WIDTH).max(CARD_MIN_WIDTH);

    // Compute card height
    let has_any_desc = grid.items.iter().any(|item| item.description.is_some());
    let card_h = if has_any_desc {
        CARD_MIN_HEIGHT + DESC_FONT_SIZE + 4.0
    } else {
        CARD_MIN_HEIGHT
    };

    let total_w = PADDING * 2.0 + cols as f64 * card_w + (cols - 1).max(0) as f64 * CARD_GAP;
    let total_h = PADDING * 2.0 + rows as f64 * card_h + (rows - 1).max(0) as f64 * CARD_GAP;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let content_y = PADDING;

    // Render cards
    for (i, item) in grid.items.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let (bg_color, accent_color) = COLORS[i % COLORS.len()];

        let x = PADDING + col as f64 * (card_w + CARD_GAP);
        let y = content_y + row as f64 * (card_h + CARD_GAP);

        // Card background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
            x, y, card_w, card_h, bg_color
        ));

        // Left accent border
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\"/>",
            x, y, LEFT_ACCENT_WIDTH, card_h, accent_color
        ));
        // Fix top-right and bottom-right corners of accent (clip to card)
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            x + LEFT_ACCENT_WIDTH / 2.0,
            y,
            LEFT_ACCENT_WIDTH / 2.0,
            card_h,
            accent_color
        ));

        // Label
        let label_x = x + LEFT_ACCENT_WIDTH + CARD_H_PAD;
        let label_y = if item.description.is_some() {
            y + card_h / 2.0 - 4.0
        } else {
            y + card_h / 2.0 + 5.0
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            label_x,
            label_y,
            accent_color,
            escape_xml(&item.label)
        ));

        // Description
        if let Some(ref desc) = item.description {
            let desc_y = label_y + DESC_FONT_SIZE + 6.0;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                label_x,
                desc_y,
                DESC_FONT_SIZE,
                COLOR_DARK,
                escape_xml(desc)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-list-grid - Render items in a grid layout as SVG

Usage: mdd-list-grid < input.list-grid

Optionally set the number of columns (default 3):
  columns <N>
Each item is: item \"<label>\" [: \"<description>\"]

Example:
  columns 2
  item \"VS Code\" : \"Code editor\"
  item \"Git\" : \"Version control\"
  item \"Docker\" : \"Containers\"
  item \"Slack\" : \"Communication\"
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

    let grid = match parse(&input) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("mdd-list-grid: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&grid));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = r#"
item "A" : "desc A"
item "B" : "desc B"
item "C"
"#;
        let g = parse(input).unwrap();
        assert_eq!(g.columns, 3);
        assert_eq!(g.items.len(), 3);
        assert_eq!(g.items[0].label, "A");
        assert_eq!(g.items[0].description.as_deref(), Some("desc A"));
        assert_eq!(g.items[2].label, "C");
        assert!(g.items[2].description.is_none());
    }

    #[test]
    fn parse_with_columns() {
        let input = r#"
columns 2
item "X" : "foo"
item "Y"
"#;
        let g = parse(input).unwrap();
        assert_eq!(g.columns, 2);
        assert_eq!(g.items.len(), 2);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
item "A" : "desc"
item "B"
"#;
        let g = parse(input).unwrap();
        let svg = render_svg(&g);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_error_no_items() {
        let input = r#"
columns 3
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_zero_columns() {
        let input = r#"
columns 0
item "A"
"#;
        assert!(parse(input).is_err());
    }
}
