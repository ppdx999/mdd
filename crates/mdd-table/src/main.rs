use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Table, String> {
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
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
        headers,
        rows,
    })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const CELL_H_PAD: f64 = 12.0;
const CELL_V_PAD: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const MIN_ROW_HEIGHT: f64 = 32.0;
const HEADER_ROW_HEIGHT: f64 = 36.0;
const MIN_COL_WIDTH: f64 = 80.0;
const MAX_COL_WIDTH: f64 = 400.0;
const PADDING: f64 = 40.0;

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

/// Wrap text into lines that fit within `max_width` pixels.
fn wrap_text(s: &str, max_width: f64) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0.0;

    for c in s.chars() {
        let cw = if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH };
        if current_width + cw > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0.0;
        }
        current_line.push(c);
        current_width += cw;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(table: &Table) -> String {
    let num_cols = table.headers.len();

    // Compute column widths proportional to total text content.
    // Sum all text widths per column (header + all cells), then distribute
    // total table width proportionally.
    let mut col_text_totals: Vec<f64> = vec![0.0; num_cols];
    for (i, h) in table.headers.iter().enumerate() {
        col_text_totals[i] += text_width(h);
    }
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_text_totals[i] += text_width(cell);
            }
        }
    }

    let grand_total: f64 = col_text_totals.iter().sum();
    let mut col_widths: Vec<f64> = if grand_total > 0.0 {
        // Each column gets a share proportional to its total text
        let target_table_w = grand_total / (table.rows.len() + 1) as f64 + CELL_H_PAD * 2.0 * num_cols as f64;
        col_text_totals
            .iter()
            .map(|&t| {
                let share = t / grand_total;
                (share * target_table_w).max(MIN_COL_WIDTH)
            })
            .collect()
    } else {
        vec![MIN_COL_WIDTH; num_cols]
    };

    // Clamp columns to MAX_COL_WIDTH — text will wrap within this limit
    for w in col_widths.iter_mut() {
        if *w > MAX_COL_WIDTH {
            *w = MAX_COL_WIDTH;
        }
    }

    // Compute row heights based on wrapped text
    let wrap_width = |col: usize| -> f64 { col_widths[col] - CELL_H_PAD * 2.0 };

    let mut row_heights: Vec<f64> = Vec::new();
    for row in &table.rows {
        let mut max_lines = 1usize;
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let lines = wrap_text(cell, wrap_width(i));
                max_lines = max_lines.max(lines.len());
            }
        }
        let h = (CELL_V_PAD * 2.0 + max_lines as f64 * LINE_HEIGHT).max(MIN_ROW_HEIGHT);
        row_heights.push(h);
    }

    let table_w: f64 = col_widths.iter().sum();
    let table_h = HEADER_ROW_HEIGHT + row_heights.iter().sum::<f64>();

    let total_w = PADDING * 2.0 + table_w;
    let total_h = PADDING * 2.0 + table_h;

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
    let mut row_y = content_y + HEADER_ROW_HEIGHT;
    for (row_idx, row) in table.rows.iter().enumerate() {
        let rh = row_heights[row_idx];

        // Alternating background
        let bg = if row_idx % 2 == 1 { ALT_ROW_BG } else { "white" };
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            table_x, row_y, table_w, rh, bg
        ));

        // Cell text (with wrapping)
        let mut cx = table_x;
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                let w = col_widths[i];
                let lines = wrap_text(cell, wrap_width(i));
                let text_block_h = lines.len() as f64 * LINE_HEIGHT;
                let text_start_y = row_y + (rh - text_block_h) / 2.0 + FONT_SIZE;
                for (li, line) in lines.iter().enumerate() {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\">{}</text>",
                        cx + CELL_H_PAD,
                        text_start_y + li as f64 * LINE_HEIGHT,
                        escape_xml(line)
                    ));
                }
                cx += w;
            }
        }

        // Row bottom border
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            table_x,
            row_y + rh,
            table_x + table_w,
            row_y + rh,
            BORDER_COLOR
        ));

        row_y += rh;
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

const HELP: &str = "\
mdd-table - Render a table as SVG

Usage: mdd-table < input.table

Rows are pipe-delimited. The first row becomes the header.
At least one data row is required after the header.

Example:
  | Name  | Role   | Status |
  | Alice | Dev    | Active |
  | Bob   | QA     | Active |
  | Carol | PM     | Away   |
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
        assert_eq!(t.headers.len(), 3);
        assert_eq!(t.headers[0], "Name");
        assert_eq!(t.headers[1], "Age");
        assert_eq!(t.headers[2], "City");
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0][0], "Alice");
        assert_eq!(t.rows[1][2], "Osaka");
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
