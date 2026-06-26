use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Piece {
    label: String,
    description: Option<String>,
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

        let (label, description) = if let Some((t, d)) = trimmed.split_once('|') {
            let d = d.trim().to_string();
            (t.trim().to_string(), if d.is_empty() { None } else { Some(d) })
        } else {
            (trimmed.to_string(), None)
        };
        pieces.push(Piece { label, description });
    }

    if pieces.len() < 2 {
        return Err("At least 2 pieces are required".to_string());
    }

    Ok(Puzzle { pieces })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const HEX_RADIUS: f64 = 55.0;
const PADDING: f64 = 40.0;
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_COLOR: &str = "#666";

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

/// Compute font size for a label to fit within the hexagon.
fn label_font_size(label: &str) -> f64 {
    let max_w = 3.0_f64.sqrt() * HEX_RADIUS - 16.0; // inner width minus padding
    // CJK chars ≈ 1.2 * font_size wide, ASCII ≈ 0.65 * font_size
    // (conservative estimates for sans-serif rendering)
    let char_count: f64 = label
        .chars()
        .map(|c| if c.is_ascii() { 0.65 } else { 1.2 })
        .sum();
    if char_count <= 0.0 {
        return FONT_SIZE;
    }
    (max_w / char_count).min(FONT_SIZE).max(7.0) // floor at 7px
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
    let _rows = (n + cols - 1) / cols;

    // Hexagon geometry: width = sqrt(3) * r, height = 2 * r
    let hex_w = 3.0_f64.sqrt() * HEX_RADIUS;
    let hex_h = 2.0 * HEX_RADIUS;

    // Grid spacing (no extra for descriptions — they radiate outward)
    let col_step = hex_w;
    let row_step = hex_h * 0.75;

    // Calculate hexagon centers first
    let origin_x = hex_w / 2.0;
    let origin_y = HEX_RADIUS;
    let mut centers: Vec<(f64, f64)> = Vec::new();
    for i in 0..n {
        let row = i / cols;
        let col = i % cols;
        let offset_x = if row % 2 == 1 { col_step * 0.5 } else { 0.0 };
        let cx = origin_x + col as f64 * col_step + offset_x;
        let cy = origin_y + row as f64 * row_step;
        centers.push((cx, cy));
    }

    // Grid center (average of all hexagon positions)
    let grid_cx: f64 = centers.iter().map(|(x, _)| x).sum::<f64>() / n as f64;
    let grid_cy: f64 = centers.iter().map(|(_, y)| y).sum::<f64>() / n as f64;

    // Compute description endpoints to determine bounding box
    let desc_line_len = 30.0;
    let desc_text_gap = 4.0;
    struct DescInfo {
        line_start: (f64, f64),
        line_end: (f64, f64),
        text_pos: (f64, f64),
        dir: (f64, f64),
    }
    let mut desc_infos: Vec<Option<DescInfo>> = Vec::new();
    for (i, piece) in puzzle.pieces.iter().enumerate() {
        if piece.description.is_none() {
            desc_infos.push(None);
            continue;
        }
        let (cx, cy) = centers[i];
        let dx = cx - grid_cx;
        let dy = cy - grid_cy;
        let dist = (dx * dx + dy * dy).sqrt();
        let (dir_x, dir_y) = if dist > 1.0 {
            (dx / dist, dy / dist)
        } else {
            // Center piece: extend upward
            (0.0, -1.0)
        };
        let ls_x = cx + dir_x * HEX_RADIUS;
        let ls_y = cy + dir_y * HEX_RADIUS;
        let le_x = ls_x + dir_x * desc_line_len;
        let le_y = ls_y + dir_y * desc_line_len;
        let tx = le_x + dir_x * desc_text_gap;
        let ty = le_y + dir_y * desc_text_gap;
        desc_infos.push(Some(DescInfo {
            line_start: (ls_x, ls_y),
            line_end: (le_x, le_y),
            text_pos: (tx, ty),
            dir: (dir_x, dir_y),
        }));
    }

    // Compute bounding box including hexagons and description text
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for (cx, cy) in &centers {
        min_x = min_x.min(cx - hex_w / 2.0);
        min_y = min_y.min(cy - HEX_RADIUS);
        max_x = max_x.max(cx + hex_w / 2.0);
        max_y = max_y.max(cy + HEX_RADIUS);
    }
    for (i, info) in desc_infos.iter().enumerate() {
        if let Some(di) = info {
            let (tx, ty) = di.text_pos;
            // Estimate text extent based on direction
            let desc = puzzle.pieces[i].description.as_ref().unwrap();
            let tw = desc.len() as f64 * 7.0; // rough text width estimate
            if di.dir.0 > 0.3 {
                max_x = max_x.max(tx + tw);
            } else if di.dir.0 < -0.3 {
                min_x = min_x.min(tx - tw);
            } else {
                min_x = min_x.min(tx - tw / 2.0);
                max_x = max_x.max(tx + tw / 2.0);
            }
            min_y = min_y.min(ty - DESC_FONT_SIZE);
            max_y = max_y.max(ty + DESC_FONT_SIZE);
        }
    }

    let total_w = (max_x - min_x) + PADDING * 2.0;
    let total_h = (max_y - min_y) + PADDING * 2.0;
    let off_x = PADDING - min_x;
    let off_y = PADDING - min_y;

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

    // Draw description lines and text (behind hexagons)
    for (i, piece) in puzzle.pieces.iter().enumerate() {
        if let (Some(desc), Some(di)) = (&piece.description, &desc_infos[i]) {
            let (_, stroke) = COLORS[i % COLORS.len()];
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" opacity=\"0.6\"/>",
                di.line_start.0 + off_x, di.line_start.1 + off_y,
                di.line_end.0 + off_x, di.line_end.1 + off_y,
                stroke
            ));
            let anchor = if di.dir.0 > 0.3 {
                "start"
            } else if di.dir.0 < -0.3 {
                "end"
            } else {
                "middle"
            };
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" dominant-baseline=\"central\" font-size=\"{}\" fill=\"{}\">{}</text>",
                di.text_pos.0 + off_x,
                di.text_pos.1 + off_y,
                anchor, DESC_FONT_SIZE, DESC_COLOR,
                escape_xml(desc)
            ));
        }
    }

    // Draw hexagons (on top of lines)
    for (i, piece) in puzzle.pieces.iter().enumerate() {
        let (cx, cy) = centers[i];
        let cx = cx + off_x;
        let cy = cy + off_y;

        let (fill, stroke) = COLORS[i % COLORS.len()];

        let points = hexagon_points(cx, cy, HEX_RADIUS);
        svg.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            points, fill, stroke
        ));

        // Label (shrink font if text is too wide for hexagon)
        let fs = label_font_size(&piece.label);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"central\" font-size=\"{:.1}\" fill=\"{}\">{}</text>",
            cx, cy, fs, stroke,
            escape_xml(&piece.label)
        ));
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-puzzle - Render a puzzle (hexagon grid) diagram as SVG

Usage: mdd-puzzle < input.puzzle

Each line is a piece label, rendered as a hexagon in a grid.
Use | to add a description shown outside the hexagon: Label | Description
At least 2 pieces are required.

Example:
  Strategy | Long-term vision
  People | Team and culture
  Technology
  Process
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
