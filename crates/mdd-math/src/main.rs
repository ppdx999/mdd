use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Math {
    expressions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Math, String> {
    let mut expressions: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // expr: ... format
        if trimmed.starts_with("expr:") {
            let rest = trimmed.strip_prefix("expr:").unwrap().trim();
            if !rest.is_empty() {
                expressions.push(rest.to_string());
            }
            continue;
        }

        // simple format: each non-empty line is an expression
        expressions.push(trimmed.to_string());
    }

    if expressions.is_empty() {
        return Err("At least 1 expression is required".to_string());
    }

    Ok(Math { expressions })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const EXPR_FONT_SIZE: f64 = 20.0;
const EXPR_LINE_HEIGHT: f64 = 36.0;
const FONT_FAMILY: &str = "serif";
const PADDING: f64 = 40.0;
const MIN_WIDTH: f64 = 200.0;

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

fn render_svg(math: &Math) -> String {
    // Compute width based on content
    let expr_scale = EXPR_FONT_SIZE / FONT_SIZE;
    let max_expr_w = math
        .expressions
        .iter()
        .map(|e| text_width(e) * expr_scale)
        .fold(0.0_f64, f64::max);

    let content_w = max_expr_w.max(MIN_WIDTH);
    let total_w = PADDING * 2.0 + content_w;

    let body_h = math.expressions.len() as f64 * EXPR_LINE_HEIGHT;
    let total_h = PADDING * 2.0 + body_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }} text.expr {{ font-family: \"Times New Roman\", {}; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK, FONT_FAMILY, EXPR_FONT_SIZE as u32, COLOR_DARK
    ));

    let center_x = total_w / 2.0;

    let y = PADDING;

    // Expressions
    for (i, expr) in math.expressions.iter().enumerate() {
        let expr_y = y + i as f64 * EXPR_LINE_HEIGHT + EXPR_LINE_HEIGHT / 2.0 + EXPR_FONT_SIZE / 3.0;
        svg.push_str(&format!(
            "<text class=\"expr\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            center_x, expr_y, escape_xml(expr)
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

    let math = match parse(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mdd-math: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&math));
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
E = mc²
F = ma
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.expressions.len(), 2);
        assert_eq!(m.expressions[0], "E = mc²");
        assert_eq!(m.expressions[1], "F = ma");
    }

    #[test]
    fn parse_expr_prefix() {
        let input = r#"
expr: E = mc²
expr: F = ma
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.expressions.len(), 2);
        assert_eq!(m.expressions[0], "E = mc²");
    }

    #[test]
    fn parse_empty_fails() {
        let input = "";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
E = mc²
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
