use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Table {
    title: Option<String>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Table, String> {
    let mut title: Option<String> = None;
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // title "..."
        if trimmed.starts_with("title ") {
            let rest = trimmed.strip_prefix("title ").unwrap().trim();
            title = Some(strip_quotes(rest).to_string());
            continue;
        }

        // pipe-delimited row
        if trimmed.starts_with('|') {
            let cells: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            if headers.is_empty() {
                headers = cells;
            } else {
                rows.push(cells);
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if headers.is_empty() {
        return Err("Missing header row".to_string());
    }
    if rows.is_empty() {
        return Err("Need at least 1 data row".to_string());
    }

    Ok(Table {
        title,
        headers,
        rows,
    })
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
const TITLE_FONT_SIZE: f64 = 16.0;
const COLOR_DARK: &str = "#333";

const CELL_H_PAD: f64 = 12.0;
const _CELL_V_PAD: f64 = 8.0;
const ROW_HEIGHT: f64 = 32.0;
const HEADER_ROW_HEIGHT: f64 = 36.0;
const MIN_COL_WIDTH: f64 = 80.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;

const HEADER_BG: &str = "#e8eaf6";
const HEADER_TEXT: &str = "#283593";
const ALT_ROW_BG: &str = "#f5f5f5";
const BORDER_COLOR: &str = "#e0e0e0";

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                CHAR_WIDTH
            } else {
                CJK_CHAR_WIDTH
            }
        })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(table: &Table) -> String {
    let num_cols = table.headers.len();

    // Compute column widths from content
    let mut col_widths: Vec<f64> = table
        .headers
        .iter()
        .map(|h| text_width(h) + CELL_H_PAD * 2.0)
        .collect();

    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let w = text_width(cell) + CELL_H_PAD * 2.0;
                if w > col_widths[i] {
                    col_widths[i] = w;
                }
            }
        }
    }

    // Enforce minimum column width
    for w in col_widths.iter_mut() {
        if *w < MIN_COL_WIDTH {
            *w = MIN_COL_WIDTH;
        }
    }

    let table_w: f64 = col_widths.iter().sum();
    let table_h = HEADER_ROW_HEIGHT + table.rows.len() as f64 * ROW_HEIGHT;

    let title_space = if table.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + table_w;
    let total_h = PADDING * 2.0 + title_space + table_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Title
    let content_y = if let Some(ref title) = table.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            TITLE_FONT_SIZE,
            escape_xml(title)
        ));
        PADDING + TITLE_HEIGHT + TITLE_GAP
    } else {
        PADDING
    };

    let table_x = PADDING;

    // Header row background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        table_x, content_y, table_w, HEADER_ROW_HEIGHT, HEADER_BG
    ));

    // Header cells
    let mut cx = table_x;
    for (i, header) in table.headers.iter().enumerate() {
        let w = col_widths[i];
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            cx + CELL_H_PAD,
            content_y + HEADER_ROW_HEIGHT / 2.0 + FONT_SIZE / 2.0 - 2.0,
            HEADER_TEXT,
            escape_xml(header)
        ));
        cx += w;
    }

    // Header bottom border
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        table_x,
        content_y + HEADER_ROW_HEIGHT,
        table_x + table_w,
        content_y + HEADER_ROW_HEIGHT,
        BORDER_COLOR
    ));

    // Data rows
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_y = content_y + HEADER_ROW_HEIGHT + row_idx as f64 * ROW_HEIGHT;

        // Alternating background
        let bg = if row_idx % 2 == 1 { ALT_ROW_BG } else { "white" };
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            table_x, row_y, table_w, ROW_HEIGHT, bg
        ));

        // Cell text
        let mut cx = table_x;
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let w = col_widths[i];
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\">{}</text>",
                    cx + CELL_H_PAD,
                    row_y + ROW_HEIGHT / 2.0 + FONT_SIZE / 2.0 - 2.0,
                    escape_xml(cell)
                ));
                cx += w;
            }
        }

        // Row bottom border
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            table_x,
            row_y + ROW_HEIGHT,
            table_x + table_w,
            row_y + ROW_HEIGHT,
            BORDER_COLOR
        ));
    }

    // Vertical column borders
    let mut cx = table_x;
    for i in 0..=num_cols {
        let stroke_w = if i == 0 || i == num_cols { "1" } else { "0.5" };
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
            cx,
            content_y,
            cx,
            content_y + table_h,
            BORDER_COLOR,
            stroke_w
        ));
        if i < num_cols {
            cx += col_widths[i];
        }
    }

    // Outer border
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        table_x, content_y, table_w, table_h, BORDER_COLOR
    ));

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let table = match parse(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("mdd-table: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&table));
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
| Name | Age | City |
| Alice | 30 | Tokyo |
| Bob | 25 | Osaka |
"#;
        let t = parse(input).unwrap();
        assert!(t.title.is_none());
        assert_eq!(t.headers.len(), 3);
        assert_eq!(t.headers[0], "Name");
        assert_eq!(t.headers[1], "Age");
        assert_eq!(t.headers[2], "City");
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0][0], "Alice");
        assert_eq!(t.rows[1][2], "Osaka");
    }

    #[test]
    fn parse_with_title() {
        let input = r#"
title "My Table"
| A | B |
| 1 | 2 |
"#;
        let t = parse(input).unwrap();
        assert_eq!(t.title.as_deref(), Some("My Table"));
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
| H1 | H2 |
| C1 | C2 |
"#;
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
