use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Radial {
    center: String,
    spokes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Radial, String> {
    let mut center: Option<String> = None;
    let mut spokes: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("center ") {
            if center.is_some() {
                return Err("Duplicate 'center' directive".to_string());
            }
            let rest = trimmed.strip_prefix("center ").unwrap().trim();
            center = Some(strip_quotes(rest).to_string());
            continue;
        }

        if trimmed.starts_with("spoke ") {
            let rest = trimmed.strip_prefix("spoke ").unwrap().trim();
            spokes.push(rest.to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    let center = center.ok_or("Missing 'center' directive")?;

    if spokes.len() < 2 {
        return Err("At least 2 spokes are required".to_string());
    }

    Ok(Radial {
        center,
        spokes,
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
const CENTER_FONT_SIZE: f64 = 15.0;
const COLOR_DARK: &str = "#333";

const ORBIT_RADIUS: f64 = 160.0;
const CENTER_NODE_W: f64 = 120.0;
const CENTER_NODE_H: f64 = 44.0;
const SPOKE_NODE_H: f64 = 36.0;
const SPOKE_NODE_H_PAD: f64 = 16.0;
const MIN_SPOKE_W: f64 = 80.0;
const PADDING: f64 = 60.0;

const CENTER_BG: &str = "#e8eaf6";
const CENTER_TEXT_COLOR: &str = "#283593";

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

fn render_svg(radial: &Radial) -> String {
    let n = radial.spokes.len();

    // Compute spoke node widths
    let spoke_widths: Vec<f64> = radial
        .spokes
        .iter()
        .map(|s| (text_width(s) + SPOKE_NODE_H_PAD * 2.0).max(MIN_SPOKE_W))
        .collect();

    let max_spoke_w = spoke_widths.iter().cloned().fold(MIN_SPOKE_W, f64::max);

    // Center node width based on text
    let center_w = (text_width(&radial.center) + SPOKE_NODE_H_PAD * 2.0).max(CENTER_NODE_W);

    // Canvas center
    let canvas_size = (ORBIT_RADIUS + max_spoke_w / 2.0 + PADDING) * 2.0;
    let cx = canvas_size / 2.0;
    let cy = PADDING + ORBIT_RADIUS + max_spoke_w / 2.0;

    let total_w = canvas_size;
    let total_h = canvas_size;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Lines from center to each spoke (drawn first, behind nodes)
    for i in 0..n {
        let angle = std::f64::consts::TAU * i as f64 / n as f64 - std::f64::consts::FRAC_PI_2;
        let x = cx + ORBIT_RADIUS * angle.cos();
        let y = cy + ORBIT_RADIUS * angle.sin();
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#bdbdbd\" stroke-width=\"2\"/>",
            cx, cy, x, y
        ));
    }

    // Center node
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx - center_w / 2.0,
        cy - CENTER_NODE_H / 2.0,
        center_w,
        CENTER_NODE_H,
        CENTER_BG,
        CENTER_TEXT_COLOR
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        cx,
        cy + CENTER_FONT_SIZE / 2.0 - 2.0,
        CENTER_FONT_SIZE,
        CENTER_TEXT_COLOR,
        escape_xml(&radial.center)
    ));

    // Spoke nodes
    for (i, spoke) in radial.spokes.iter().enumerate() {
        let angle = std::f64::consts::TAU * i as f64 / n as f64 - std::f64::consts::FRAC_PI_2;
        let x = cx + ORBIT_RADIUS * angle.cos();
        let y = cy + ORBIT_RADIUS * angle.sin();
        let w = spoke_widths[i];
        let (bg, fg) = COLORS[i % COLORS.len()];

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x - w / 2.0,
            y - SPOKE_NODE_H / 2.0,
            w,
            SPOKE_NODE_H,
            bg,
            fg
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
            x,
            y + FONT_SIZE / 2.0 - 2.0,
            fg,
            escape_xml(spoke)
        ));
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-radial - Render a radial (hub-and-spoke) diagram as SVG

Usage: mdd-radial < input.radial

Use \"center\" to define the central node and \"spoke\" for each
surrounding node. At least 2 spokes are required.
Quoted strings are supported for the center label.

Example:
  center \"Marketing\"
  spoke Product
  spoke Price
  spoke Place
  spoke Promotion
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

    let radial = match parse(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mdd-radial: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&radial));
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
center "Hub"
spoke A
spoke B
"#;
        let r = parse(input).unwrap();
        assert_eq!(r.center, "Hub");
        assert_eq!(r.spokes.len(), 2);
        assert_eq!(r.spokes[0], "A");
        assert_eq!(r.spokes[1], "B");
    }

    #[test]
    fn parse_requires_two_spokes() {
        let input = r#"
center "Hub"
spoke A
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_requires_center() {
        let input = r#"
spoke A
spoke B
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_rejects_duplicate_center() {
        let input = r#"
center "A"
center "B"
spoke X
spoke Y
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
center "Hub"
spoke A
spoke B
"#;
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
