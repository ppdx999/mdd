use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Matrix {
    x_axis: Option<(String, String)>,
    y_axis: Option<(String, String)>,
    quadrants: [Vec<String>; 4],
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Matrix, String> {
    let mut x_axis: Option<(String, String)> = None;
    let mut y_axis: Option<(String, String)> = None;
    let mut quadrants: [Vec<String>; 4] = [vec![], vec![], vec![], vec![]];

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // x-axis "Low" "High"
        if trimmed.starts_with("x-axis ") {
            let rest = trimmed.strip_prefix("x-axis ").unwrap().trim();
            let labels = parse_quoted_strings(rest)?;
            if labels.len() != 2 {
                return Err("x-axis requires exactly 2 labels".to_string());
            }
            x_axis = Some((labels[0].clone(), labels[1].clone()));
            continue;
        }

        // y-axis "Low" "High"
        if trimmed.starts_with("y-axis ") {
            let rest = trimmed.strip_prefix("y-axis ").unwrap().trim();
            let labels = parse_quoted_strings(rest)?;
            if labels.len() != 2 {
                return Err("y-axis requires exactly 2 labels".to_string());
            }
            y_axis = Some((labels[0].clone(), labels[1].clone()));
            continue;
        }

        // quadrant N : "Item A" "Item B"
        if trimmed.starts_with("quadrant ") {
            let rest = trimmed.strip_prefix("quadrant ").unwrap().trim();
            let (num_str, after_num) = rest
                .split_once(|c: char| c.is_whitespace() || c == ':')
                .ok_or_else(|| format!("Invalid quadrant syntax: {}", trimmed))?;
            let num: usize = num_str
                .parse()
                .map_err(|_| format!("Invalid quadrant number: {}", num_str))?;
            if num < 1 || num > 4 {
                return Err(format!("Quadrant number must be 1-4, got: {}", num));
            }
            let after_colon = if after_num.contains(':') {
                after_num.split_once(':').unwrap().1.trim()
            } else {
                after_num.trim()
            };
            let items = parse_quoted_strings(after_colon)?;
            quadrants[num - 1] = items;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    Ok(Matrix {
        x_axis,
        y_axis,
        quadrants,
    })
}

fn parse_quoted_strings(s: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();

    loop {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        if chars.peek() != Some(&'"') {
            return Err(format!("Expected '\"' in: {}", s));
        }
        chars.next(); // consume opening quote
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some(c) => value.push(c),
                None => return Err("Unterminated quoted string".to_string()),
            }
        }
        result.push(value);
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

#[allow(dead_code)]
const CHAR_WIDTH: f64 = 8.0;
#[allow(dead_code)]
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const QUADRANT_COLORS: [&str; 4] = ["#e3f2fd", "#e8f5e9", "#fff8e1", "#fce4ec"];

const QUADRANT_SIZE: f64 = 200.0;
const PADDING: f64 = 60.0;
const AXIS_LABEL_GAP: f64 = 30.0;
const ITEM_LINE_HEIGHT: f64 = 20.0;

const GRID_COLOR: &str = "#bdbdbd";

#[allow(dead_code)]
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

fn render_svg(matrix: &Matrix) -> String {
    let has_x_axis = matrix.x_axis.is_some();
    let has_y_axis = matrix.y_axis.is_some();

    let y_axis_space = if has_y_axis { AXIS_LABEL_GAP } else { 0.0 };
    let x_axis_space = if has_x_axis { AXIS_LABEL_GAP } else { 0.0 };

    let grid_x = PADDING + y_axis_space;
    let grid_y = PADDING;
    let grid_w = QUADRANT_SIZE * 2.0;
    let grid_h = QUADRANT_SIZE * 2.0;

    let total_w = grid_x + grid_w + PADDING;
    let total_h = grid_y + grid_h + x_axis_space + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Quadrant backgrounds: 1=top-left, 2=top-right, 3=bottom-left, 4=bottom-right
    let quad_positions = [
        (grid_x, grid_y),                           // Q1: top-left
        (grid_x + QUADRANT_SIZE, grid_y),            // Q2: top-right
        (grid_x, grid_y + QUADRANT_SIZE),            // Q3: bottom-left
        (grid_x + QUADRANT_SIZE, grid_y + QUADRANT_SIZE), // Q4: bottom-right
    ];

    for (i, &(qx, qy)) in quad_positions.iter().enumerate() {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            qx, qy, QUADRANT_SIZE, QUADRANT_SIZE, QUADRANT_COLORS[i]
        ));
    }

    // Grid border
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        grid_x, grid_y, grid_w, grid_h, GRID_COLOR
    ));

    // Vertical grid line (center)
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        grid_x + QUADRANT_SIZE, grid_y,
        grid_x + QUADRANT_SIZE, grid_y + grid_h,
        GRID_COLOR
    ));

    // Horizontal grid line (center)
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        grid_x, grid_y + QUADRANT_SIZE,
        grid_x + grid_w, grid_y + QUADRANT_SIZE,
        GRID_COLOR
    ));

    // Quadrant items
    for (i, &(qx, qy)) in quad_positions.iter().enumerate() {
        let items = &matrix.quadrants[i];
        let start_y = qy + 24.0;
        for (j, item) in items.iter().enumerate() {
            let item_y = start_y + j as f64 * ITEM_LINE_HEIGHT;
            let label = format!("\u{2022} {}", item);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>",
                qx + 12.0,
                item_y,
                escape_xml(&label)
            ));
        }
    }

    // X-axis labels (bottom)
    if let Some(ref x) = matrix.x_axis {
        let label_y = grid_y + grid_h + 20.0;
        // Left label (under Q3)
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            grid_x + QUADRANT_SIZE / 2.0,
            label_y,
            escape_xml(&x.0)
        ));
        // Right label (under Q4)
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            grid_x + QUADRANT_SIZE + QUADRANT_SIZE / 2.0,
            label_y,
            escape_xml(&x.1)
        ));
    }

    // Y-axis labels (left side)
    if let Some(ref y) = matrix.y_axis {
        let label_x = grid_x - 10.0;
        // Top label (beside Q1) — rotated
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, {}, {})\">{}</text>",
            label_x,
            grid_y + QUADRANT_SIZE / 2.0,
            label_x,
            grid_y + QUADRANT_SIZE / 2.0,
            escape_xml(&y.1)
        ));
        // Bottom label (beside Q3) — rotated
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" transform=\"rotate(-90, {}, {})\">{}</text>",
            label_x,
            grid_y + QUADRANT_SIZE + QUADRANT_SIZE / 2.0,
            label_x,
            grid_y + QUADRANT_SIZE + QUADRANT_SIZE / 2.0,
            escape_xml(&y.0)
        ));
    }

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
    fn parse_basic() {
        let input = r#"
quadrant 1 : "A" "B"
quadrant 2 : "C"
quadrant 3 : "D" "E"
quadrant 4 : "F"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.quadrants[0], vec!["A", "B"]);
        assert_eq!(m.quadrants[1], vec!["C"]);
        assert_eq!(m.quadrants[2], vec!["D", "E"]);
        assert_eq!(m.quadrants[3], vec!["F"]);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
quadrant 1 : "X"
quadrant 2 : "Y"
quadrant 3 : "Z"
quadrant 4 : "W"
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_with_axis_labels() {
        let input = r#"
x-axis "Low" "High"
y-axis "Small" "Large"
quadrant 1 : "A"
quadrant 2 : "B"
quadrant 3 : "C"
quadrant 4 : "D"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.x_axis, Some(("Low".to_string(), "High".to_string())));
        assert_eq!(m.y_axis, Some(("Small".to_string(), "Large".to_string())));
    }

    #[test]
    fn parse_japanese() {
        let input = r#"
x-axis "緊急でない" "緊急"
y-axis "重要でない" "重要"
quadrant 1 : "計画" "戦略立案"
quadrant 2 : "即実行" "危機対応"
quadrant 3 : "削除" "時間の浪費"
quadrant 4 : "委任" "割り込み作業"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.quadrants[0].len(), 2);
        assert_eq!(m.quadrants[0][0], "計画");
    }

    #[test]
    fn render_contains_items() {
        let input = r#"
quadrant 1 : "Alpha"
quadrant 2 : "Beta"
quadrant 3 : "Gamma"
quadrant 4 : "Delta"
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
        assert!(svg.contains("Gamma"));
        assert!(svg.contains("Delta"));
    }

    #[test]
    fn render_with_axes() {
        let input = r#"
x-axis "Left" "Right"
y-axis "Bottom" "Top"
quadrant 1 : "A"
quadrant 2 : "B"
quadrant 3 : "C"
quadrant 4 : "D"
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.contains("Left"));
        assert!(svg.contains("Right"));
        assert!(svg.contains("Bottom"));
        assert!(svg.contains("Top"));
    }

    #[test]
    fn error_invalid_quadrant_number() {
        let input = r#"quadrant 5 : "X""#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_invalid_syntax() {
        let input = "foobar";
        assert!(parse(input).is_err());
    }

    #[test]
    fn text_width_ascii() {
        assert!((text_width("abc") - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn text_width_cjk() {
        assert!((text_width("あ") - 14.0).abs() < f64::EPSILON);
    }

    #[test]
    fn escape_xml_special() {
        assert_eq!(escape_xml("<a&b>"), "&lt;a&amp;b&gt;");
    }
}
