use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Piece {
    label: String,
}

#[derive(Debug)]
struct Puzzle {
    pieces: Vec<Piece>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Puzzle, String> {
    let mut pieces: Vec<Piece> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        pieces.push(Piece {
            label: trimmed.to_string(),
        });
    }

    if pieces.len() < 2 {
        return Err("At least 2 pieces are required".to_string());
    }

    Ok(Puzzle { pieces })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const HEX_RADIUS: f64 = 55.0;
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

fn hexagon_points(cx: f64, cy: f64, r: f64) -> String {
    (0..6)
        .map(|i| {
            let angle = std::f64::consts::FRAC_PI_3 * i as f64 - std::f64::consts::FRAC_PI_6;
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            format!("{:.1},{:.1}", x, y)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_svg(puzzle: &Puzzle) -> String {
    let n = puzzle.pieces.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = (n + cols - 1) / cols;

    // Hexagon geometry: width = sqrt(3) * r, height = 2 * r
    let hex_w = 3.0_f64.sqrt() * HEX_RADIUS;
    let hex_h = 2.0 * HEX_RADIUS;

    // Grid spacing
    let col_step = hex_w;
    let row_step = hex_h * 0.75;

    // Calculate total dimensions
    let grid_w = if rows > 1 {
        cols as f64 * col_step + col_step * 0.5
    } else {
        cols as f64 * col_step
    };
    let grid_h = row_step * (rows - 1) as f64 + hex_h;

    let total_w = PADDING * 2.0 + grid_w;
    let total_h = PADDING * 2.0 + grid_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w.ceil(),
        total_h.ceil(),
        total_w.ceil(),
        total_h.ceil()
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Draw hexagons
    let origin_x = PADDING + hex_w / 2.0;
    let origin_y = PADDING + HEX_RADIUS;

    for (i, piece) in puzzle.pieces.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;

        let offset_x = if row % 2 == 1 { col_step * 0.5 } else { 0.0 };
        let cx = origin_x + col as f64 * col_step + offset_x;
        let cy = origin_y + row as f64 * row_step;

        let (fill, stroke) = COLORS[i % COLORS.len()];

        let points = hexagon_points(cx, cy, HEX_RADIUS);
        svg.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            points, fill, stroke
        ));

        // Label
        let label = &piece.label;
        let tw = text_width(label);
        let _ = tw; // used implicitly by text-anchor middle
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"central\" fill=\"{}\">{}</text>",
            cx,
            cy,
            stroke,
            escape_xml(label)
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

    let puzzle = match parse(&input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("mdd-puzzle: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&puzzle));
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
A
B
C
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.pieces.len(), 3);
        assert_eq!(p.pieces[0].label, "A");
        assert_eq!(p.pieces[1].label, "B");
        assert_eq!(p.pieces[2].label, "C");
    }

    #[test]
    fn parse_too_few_pieces() {
        let input = "A\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_empty() {
        let input = "";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_japanese() {
        let input = "リーダー\nデザイナー\n";
        let p = parse(input).unwrap();
        assert_eq!(p.pieces[0].label, "リーダー");
    }

    #[test]
    fn render_produces_svg() {
        let input = "A\nB\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn render_contains_pieces() {
        let input = "Hello\nWorld\nTest\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.contains("Hello"));
        assert!(svg.contains("World"));
        assert!(svg.contains("Test"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn hexagon_has_six_points() {
        let points = hexagon_points(100.0, 100.0, 50.0);
        let count = points.split(' ').count();
        assert_eq!(count, 6);
    }
}
