use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum RowKind {
    Normal,
    Subtotal,
    Total,
}

#[derive(Debug)]
struct Row {
    cells: Vec<String>,
    kind: RowKind,
}

#[derive(Debug)]
struct Table {
    headers: Vec<String>,
    rows: Vec<Row>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Table, String> {
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detect row kind from prefix:
        //   |== ... | → Total
        //   |= ... |  → Subtotal
        //   | ... |    → Normal
        if trimmed.starts_with('|') {
            let (kind, content) = if trimmed.starts_with("|==") {
                (RowKind::Total, trimmed.replacen("|==", "|", 1))
            } else if trimmed.starts_with("|=") {
                (RowKind::Subtotal, trimmed.replacen("|=", "|", 1))
            } else {
                (RowKind::Normal, trimmed.to_string())
            };

            let cells: Vec<String> = content
                .trim_matches('|')
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            if headers.is_empty() {
                headers = cells;
            } else {
                rows.push(Row { cells, kind });
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
const MAX_COL_WIDTH: f64 = 600.0;
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
        for (i, cell) in row.cells.iter().enumerate() {
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

    // Ensure columns are wide enough for summary row text (subtotal/total),
    // since summary rows don't wrap text.
    for row in &table.rows {
        if row.kind != RowKind::Normal {
            for (i, cell) in row.cells.iter().enumerate() {
                if i < num_cols {
                    let min_w = text_width(cell) + CELL_H_PAD * 2.0;
                    if col_widths[i] < min_w {
                        col_widths[i] = min_w;
                    }
                }
            }
        }
    }

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
        for (i, cell) in row.cells.iter().enumerate() {
            if i < num_cols {
                let lines = wrap_text(cell, wrap_width(i));
                max_lines = max_lines.max(lines.len());
            }
        }
        let h = (CELL_V_PAD * 2.0 + max_lines as f64 * LINE_HEIGHT).max(MIN_ROW_HEIGHT);
        row_heights.push(h);
    }

    let table_w: f64 = col_widths.iter().sum();
    let block_gap = 16.0; // gap between table blocks and summary rows
    let summary_row_h = MIN_ROW_HEIGHT;

    // Split rows into visual blocks separated by subtotal/total rows.
    // Each block is: a run of normal rows, optionally followed by a summary row.
    struct Block {
        normal_indices: Vec<usize>,  // indices into table.rows
        summary_idx: Option<usize>,  // subtotal/total row after this block
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut current_normals: Vec<usize> = Vec::new();
    for (row_idx, row) in table.rows.iter().enumerate() {
        if row.kind == RowKind::Normal {
            current_normals.push(row_idx);
        } else {
            blocks.push(Block {
                normal_indices: std::mem::take(&mut current_normals),
                summary_idx: Some(row_idx),
            });
        }
    }
    if !current_normals.is_empty() {
        blocks.push(Block {
            normal_indices: current_normals,
            summary_idx: None,
        });
    }

    // Compute total height
    let mut total_content_h = 0.0;
    for (bi, block) in blocks.iter().enumerate() {
        // Header row for first block only
        if bi == 0 {
            total_content_h += HEADER_ROW_HEIGHT;
        }
        // Normal rows
        for &ri in &block.normal_indices {
            total_content_h += row_heights[ri];
        }
        // Summary row
        if block.summary_idx.is_some() {
            total_content_h += block_gap + summary_row_h;
        }
        // Gap before next block (if there is one)
        if bi < blocks.len() - 1 {
            total_content_h += block_gap;
        }
    }

    let total_w = PADDING * 2.0 + table_w;
    let total_h = PADDING * 2.0 + total_content_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let table_x = PADDING;
    let mut cursor_y = PADDING;

    // Helper: render a table block (header + normal rows) with border
    let render_table_block = |svg: &mut String, y: f64, show_header: bool, row_indices: &[usize]| -> f64 {
        let mut block_h = if show_header { HEADER_ROW_HEIGHT } else { 0.0 };
        for &ri in row_indices {
            block_h += row_heights[ri];
        }

        if show_header {
            // Header background
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                table_x, y, table_w, HEADER_ROW_HEIGHT, HEADER_BG
            ));
            let mut cx = table_x;
            for (i, header) in table.headers.iter().enumerate() {
                let w = col_widths[i];
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                    cx + CELL_H_PAD,
                    y + HEADER_ROW_HEIGHT / 2.0 + FONT_SIZE / 2.0 - 2.0,
                    HEADER_TEXT,
                    escape_xml(header)
                ));
                cx += w;
            }
        }

        // Data rows
        let mut row_y = y + if show_header { HEADER_ROW_HEIGHT } else { 0.0 };
        for (local_idx, &ri) in row_indices.iter().enumerate() {
            let rh = row_heights[ri];
            let bg = if local_idx % 2 == 1 { ALT_ROW_BG } else { "white" };
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                table_x, row_y, table_w, rh, bg
            ));

            let mut cx = table_x;
            for (i, cell) in table.rows[ri].cells.iter().enumerate() {
                if i < num_cols {
                    let w = col_widths[i];
                    let ww = w - CELL_H_PAD * 2.0;
                    let lines = wrap_text(cell, ww);
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

            // Row bottom border (except last)
            if local_idx < row_indices.len() - 1 {
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                    table_x, row_y + rh, table_x + table_w, row_y + rh, BORDER_COLOR
                ));
            }
            row_y += rh;
        }

        // Vertical column borders
        let mut cx = table_x;
        for i in 0..=num_cols {
            let stroke_w = if i == 0 || i == num_cols { "1" } else { "0.5" };
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/>",
                cx, y, cx, y + block_h, BORDER_COLOR, stroke_w
            ));
            if i < num_cols { cx += col_widths[i]; }
        }

        // Outer border
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
            table_x, y, table_w, block_h, BORDER_COLOR
        ));

        block_h
    };

    // Helper: render a summary row (subtotal/total) outside the table
    let render_summary_row = |svg: &mut String, y: f64, row: &Row| -> f64 {
        // Top line(s)
        if row.kind == RowKind::Total {
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\" stroke-width=\"1\"/>",
                table_x, y, table_x + table_w, y
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\" stroke-width=\"1\"/>",
                table_x, y + 3.0, table_x + table_w, y + 3.0
            ));
        }

        // Text
        let text_y = y + summary_row_h / 2.0 + FONT_SIZE / 2.0 - 2.0;
        let mut cx = table_x;
        for (i, cell) in row.cells.iter().enumerate() {
            if i < num_cols {
                let w = col_widths[i];
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
                    cx + CELL_H_PAD, text_y, escape_xml(cell)
                ));
                cx += w;
            }
        }

        summary_row_h
    };

    // Render blocks
    for (bi, block) in blocks.iter().enumerate() {
        // Table block (with header only for first block)
        if !block.normal_indices.is_empty() {
            let show_header = bi == 0;
            let h = render_table_block(&mut svg, cursor_y, show_header, &block.normal_indices);
            cursor_y += h;
        }

        // Summary row after this block
        if let Some(si) = block.summary_idx {
            cursor_y += block_gap;
            let h = render_summary_row(&mut svg, cursor_y, &table.rows[si]);
            cursor_y += h;
        }

        // Gap before next block
        if bi < blocks.len() - 1 {
            cursor_y += block_gap;
        }
    }

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

Row prefixes:
  | ... |     Normal data row
  |= ... |    Subtotal row (splits the table, shown outside with a line)
  |== ... |   Total row (shown outside with a double line)

Column widths are proportional to text content and capped at 400px.
Long text wraps automatically within cells.

Example:
  | Item       | Amount    |
  | Labor      | 1,000,000 |
  | Transport  | 50,000    |
  |= Subtotal  | 1,050,000 |
  | Supplies   | 200,000   |
  |= Subtotal  | 200,000   |
  |== Total    | 1,250,000 |
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
        assert_eq!(t.rows[0].cells[0], "Alice");
        assert_eq!(t.rows[1].cells[2], "Osaka");
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
