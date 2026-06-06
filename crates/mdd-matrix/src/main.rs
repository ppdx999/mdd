use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ColorDef {
    text: String,
    bg: String,
}

#[derive(Debug)]
struct Row {
    label: String,
    values: Vec<String>,
}

#[derive(Debug)]
struct Matrix {
    columns: Vec<String>,
    rows: Vec<Row>,
    colors: HashMap<String, ColorDef>,
}

// ---------------------------------------------------------------------------
// Named colors
// ---------------------------------------------------------------------------

fn resolve_color(name: &str) -> String {
    match name.trim() {
        "red" => "#c62828".to_string(),
        "blue" => "#1565c0".to_string(),
        "green" => "#2e7d32".to_string(),
        "amber" | "yellow" => "#f57f17".to_string(),
        "orange" => "#e65100".to_string(),
        "teal" => "#00695c".to_string(),
        "purple" => "#6a1b9a".to_string(),
        "pink" => "#ad1457".to_string(),
        "grey" | "gray" => "#9e9e9e".to_string(),
        "lightgrey" | "lightgray" => "#bdbdbd".to_string(),
        "black" => "#333333".to_string(),
        other => other.to_string(), // pass through hex codes like #ff0000
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Matrix, String> {
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut colors: HashMap<String, ColorDef> = HashMap::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("columns ") {
            let rest = line.strip_prefix("columns ").unwrap();
            columns = rest.split(',').map(|s| s.trim().to_string()).collect();
            continue;
        }

        // color <value> : <text_color>
        // color <value> : <text_color>, <bg_color>
        if line.starts_with("color ") {
            let rest = line.strip_prefix("color ").unwrap();
            if let Some((value, colors_str)) = rest.split_once(" : ") {
                let parts: Vec<&str> = colors_str.split(',').collect();
                let text_color = resolve_color(parts[0].trim());
                let bg_color = if parts.len() > 1 {
                    resolve_color(parts[1].trim())
                } else {
                    "#fff".to_string()
                };
                colors.insert(
                    value.trim().to_string(),
                    ColorDef { text: text_color, bg: bg_color },
                );
                continue;
            }
            return Err(format!("Invalid color syntax: {}", line));
        }

        if let Some((label, values_str)) = line.split_once(" : ") {
            let values: Vec<String> = values_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            rows.push(Row {
                label: label.trim().to_string(),
                values,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if columns.is_empty() {
        return Err("Missing 'columns' definition".to_string());
    }

    Ok(Matrix { columns, rows, colors })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const PADDING: f64 = 20.0;
const CELL_H_PAD: f64 = 16.0;
const ROW_HEIGHT: f64 = 36.0;
const MIN_COL_W: f64 = 80.0;
const ROW_LABEL_MIN_W: f64 = 100.0;

const COLOR_DARK: &str = "#333";
const COLOR_GRID: &str = "#ddd";
const COLOR_COL_HEADER_BG: &str = "#e8eaf6";
const COLOR_COL_HEADER_TEXT: &str = "#333";
const COLOR_ROW_HEADER_BG: &str = "#f5f5f5";

// ---------------------------------------------------------------------------
// Cell color lookup
// ---------------------------------------------------------------------------

const DEFAULT_TEXT_COLOR: &str = "#333";
const DEFAULT_BG_COLOR: &str = "#fff";

fn cell_color<'a>(value: &str, colors: &'a HashMap<String, ColorDef>) -> (String, String) {
    if let Some(def) = colors.get(value.trim()) {
        (def.bg.clone(), def.text.clone())
    } else {
        (DEFAULT_BG_COLOR.to_string(), DEFAULT_TEXT_COLOR.to_string())
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(matrix: &Matrix) -> String {
    // Compute column widths
    let row_label_w = matrix
        .rows
        .iter()
        .map(|r| text_width(&r.label) + CELL_H_PAD * 2.0)
        .fold(ROW_LABEL_MIN_W, f64::max);

    let col_widths: Vec<f64> = matrix
        .columns
        .iter()
        .enumerate()
        .map(|(ci, col)| {
            let header_w = text_width(col) + CELL_H_PAD * 2.0;
            let max_val_w = matrix
                .rows
                .iter()
                .map(|r| {
                    let val = r.values.get(ci).map(|s| s.as_str()).unwrap_or("");
                    text_width(val) + CELL_H_PAD * 2.0
                })
                .fold(0.0_f64, f64::max);
            header_w.max(max_val_w).max(MIN_COL_W)
        })
        .collect();

    let total_w = PADDING * 2.0 + row_label_w + col_widths.iter().sum::<f64>();
    let total_h = PADDING * 2.0 + ROW_HEIGHT * (1 + matrix.rows.len()) as f64;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    let table_x = PADDING;
    let table_y = PADDING;

    // Row label header (top-left corner)
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        table_x, table_y, row_label_w, ROW_HEIGHT, COLOR_COL_HEADER_BG
    ));

    // Column headers
    let mut cx = table_x + row_label_w;
    for (ci, col) in matrix.columns.iter().enumerate() {
        let cw = col_widths[ci];
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            cx, table_y, cw, ROW_HEIGHT, COLOR_COL_HEADER_BG
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            cx + cw / 2.0,
            table_y + ROW_HEIGHT / 2.0 + 5.0,
            COLOR_COL_HEADER_TEXT,
            escape_xml(col)
        ));
        cx += cw;
    }

    // Data rows
    for (ri, row) in matrix.rows.iter().enumerate() {
        let ry = table_y + ROW_HEIGHT * (ri + 1) as f64;

        // Row label
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            table_x, ry, row_label_w, ROW_HEIGHT, COLOR_ROW_HEADER_BG
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
            table_x + CELL_H_PAD,
            ry + ROW_HEIGHT / 2.0 + 5.0,
            escape_xml(&row.label)
        ));

        // Cell values
        let mut cx = table_x + row_label_w;
        for (ci, col_w) in col_widths.iter().enumerate() {
            let val = row.values.get(ci).map(|s| s.as_str()).unwrap_or("");
            let (bg, text_color) = cell_color(val, &matrix.colors);

            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                cx, ry, col_w, ROW_HEIGHT, bg
            ));

            if !val.is_empty() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                    cx + col_w / 2.0,
                    ry + ROW_HEIGHT / 2.0 + 5.0,
                    text_color,
                    escape_xml(val)
                ));
            }

            cx += col_w;
        }
    }

    // Grid lines
    let table_w = row_label_w + col_widths.iter().sum::<f64>();
    let table_h = ROW_HEIGHT * (1 + matrix.rows.len()) as f64;

    // Horizontal lines
    for i in 0..=(matrix.rows.len() + 1) {
        let y = table_y + ROW_HEIGHT * i as f64;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            table_x, y, table_x + table_w, y, COLOR_GRID
        ));
    }

    // Vertical lines
    let mut vx = table_x;
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        vx, table_y, vx, table_y + table_h, COLOR_GRID
    ));
    vx += row_label_w;
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        vx, table_y, vx, table_y + table_h, COLOR_GRID
    ));
    for cw in &col_widths {
        vx += cw;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            vx, table_y, vx, table_y + table_h, COLOR_GRID
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let matrix = match parse(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mdd-matrix: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&matrix));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_columns() {
        let input = "columns A, B, C\n";
        let m = parse(input).unwrap();
        assert_eq!(m.columns, vec!["A", "B", "C"]);
    }

    #[test]
    fn parse_row() {
        let input = "columns X, Y\nAlice : R, A\n";
        let m = parse(input).unwrap();
        assert_eq!(m.rows.len(), 1);
        assert_eq!(m.rows[0].label, "Alice");
        assert_eq!(m.rows[0].values, vec!["R", "A"]);
    }

    #[test]
    fn parse_multiple_rows() {
        let input = "columns A, B\nX : R, A\nY : C, I\n";
        let m = parse(input).unwrap();
        assert_eq!(m.rows.len(), 2);
    }

    #[test]
    fn parse_japanese() {
        let input = "columns 設計, 実装\n田中 : R, A\n";
        let m = parse(input).unwrap();
        assert_eq!(m.columns[0], "設計");
        assert_eq!(m.rows[0].label, "田中");
    }

    #[test]
    fn cell_color_from_dsl() {
        let input = "columns A\ncolor R : blue\ncolor - : lightgrey\nX : R\n";
        let m = parse(input).unwrap();
        assert_eq!(cell_color("R", &m.colors).1, "#1565c0");
        assert_eq!(cell_color("-", &m.colors).1, "#bdbdbd");
    }

    #[test]
    fn cell_color_with_bg() {
        let input = "columns A\ncolor OK : green, #e8f5e9\nX : OK\n";
        let m = parse(input).unwrap();
        let (bg, text) = cell_color("OK", &m.colors);
        assert_eq!(text, "#2e7d32");
        assert_eq!(bg, "#e8f5e9");
    }

    #[test]
    fn cell_color_default_for_unknown() {
        let colors = HashMap::new();
        let (bg, text) = cell_color("???", &colors);
        assert_eq!(bg, "#fff");
        assert_eq!(text, "#333");
    }

    #[test]
    fn render_produces_svg() {
        let input = "columns A, B\nX : R, A\n";
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn missing_columns_error() {
        let result = parse("X : R, A\n");
        assert!(result.is_err());
    }
}
