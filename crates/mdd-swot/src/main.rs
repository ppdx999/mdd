use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Swot {
    strengths: Vec<String>,
    weaknesses: Vec<String>,
    opportunities: Vec<String>,
    threats: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Swot, String> {
    let mut strengths: Vec<String> = Vec::new();
    let mut weaknesses: Vec<String> = Vec::new();
    let mut opportunities: Vec<String> = Vec::new();
    let mut threats: Vec<String> = Vec::new();

    let mut current_section: Option<String> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Section openers: strengths {, weaknesses {, opportunities {, threats {
        if current_section.is_none() {
            let section_name = if trimmed.starts_with("strengths") {
                Some("strengths")
            } else if trimmed.starts_with("weaknesses") {
                Some("weaknesses")
            } else if trimmed.starts_with("opportunities") {
                Some("opportunities")
            } else if trimmed.starts_with("threats") {
                Some("threats")
            } else {
                None
            };

            if let Some(name) = section_name {
                let rest = trimmed.strip_prefix(name).unwrap().trim();
                if rest == "{" || rest.is_empty() {
                    current_section = Some(name.to_string());
                    continue;
                } else if trimmed.ends_with('{') {
                    current_section = Some(name.to_string());
                    continue;
                } else {
                    return Err(format!("Expected '{{' after {}", name));
                }
            }
        }

        // closing brace
        if trimmed == "}" {
            if current_section.is_none() {
                return Err("Unexpected '}'".to_string());
            }
            current_section = None;
            continue;
        }

        // inside a section
        if let Some(ref section) = current_section {
            match section.as_str() {
                "strengths" => strengths.push(trimmed.to_string()),
                "weaknesses" => weaknesses.push(trimmed.to_string()),
                "opportunities" => opportunities.push(trimmed.to_string()),
                "threats" => threats.push(trimmed.to_string()),
                _ => {}
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_section.is_some() {
        return Err("Unclosed section (missing '}')".to_string());
    }

    let total_items = strengths.len() + weaknesses.len() + opportunities.len() + threats.len();
    if total_items == 0 {
        return Err("At least one section must have items".to_string());
    }

    Ok(Swot {
        strengths,
        weaknesses,
        opportunities,
        threats,
    })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const HEADER_FONT_SIZE: f64 = 14.0;
const COLOR_DARK: &str = "#333";

const QUADRANT_MIN_WIDTH: f64 = 220.0;
const QUADRANT_H_PAD: f64 = 16.0;
const HEADER_HEIGHT: f64 = 36.0;
const ITEM_HEIGHT: f64 = 24.0;
const QUADRANT_GAP: f64 = 4.0;
const PADDING: f64 = 40.0;
const BULLET_RADIUS: f64 = 3.0;

// Quadrant colors: (desc_bg, title_bg, accent)
const S_BG: &str = "#f0f7ff";
const S_TITLE: &str = "#90caf9";
const S_ACCENT: &str = "#1565c0";
const W_BG: &str = "#fdf2f4";
const W_TITLE: &str = "#ef9a9a";
const W_ACCENT: &str = "#c62828";
const O_BG: &str = "#f0f9f1";
const O_TITLE: &str = "#a5d6a7";
const O_ACCENT: &str = "#2e7d32";
const T_BG: &str = "#fffef2";
const T_TITLE: &str = "#ffe082";
const T_ACCENT: &str = "#f57f17";

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

fn render_svg(swot: &Swot) -> String {
    // Compute quadrant width based on content
    let all_items: Vec<(&[String], &str)> = vec![
        (&swot.strengths, "S - Strengths"),
        (&swot.weaknesses, "W - Weaknesses"),
        (&swot.opportunities, "O - Opportunities"),
        (&swot.threats, "T - Threats"),
    ];

    let mut max_content_w: f64 = 0.0;
    for (items, header) in &all_items {
        let header_w = text_width(header) + QUADRANT_H_PAD * 2.0;
        max_content_w = max_content_w.max(header_w);
        for item in *items {
            let item_w = text_width(item) + QUADRANT_H_PAD * 3.0; // extra pad for bullet
            max_content_w = max_content_w.max(item_w);
        }
    }

    let quad_w = max_content_w.max(QUADRANT_MIN_WIDTH);

    // Compute quadrant heights: top row = max(strengths, weaknesses), bottom row = max(opportunities, threats)
    let top_items = swot.strengths.len().max(swot.weaknesses.len()).max(1);
    let bottom_items = swot.opportunities.len().max(swot.threats.len()).max(1);

    let top_h = HEADER_HEIGHT + top_items as f64 * ITEM_HEIGHT + QUADRANT_H_PAD;
    let bottom_h = HEADER_HEIGHT + bottom_items as f64 * ITEM_HEIGHT + QUADRANT_H_PAD;

    let total_w = PADDING + quad_w + QUADRANT_GAP + quad_w + PADDING;
    let total_h = PADDING + top_h + QUADRANT_GAP + bottom_h + PADDING;

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

    // Top-left: Strengths
    render_quadrant(
        &mut svg,
        PADDING,
        content_y,
        quad_w,
        top_h,
        "S - Strengths",
        &swot.strengths,
        S_BG,
        S_TITLE,
        S_ACCENT,
    );

    // Top-right: Weaknesses
    render_quadrant(
        &mut svg,
        PADDING + quad_w + QUADRANT_GAP,
        content_y,
        quad_w,
        top_h,
        "W - Weaknesses",
        &swot.weaknesses,
        W_BG,
        W_TITLE,
        W_ACCENT,
    );

    // Bottom-left: Opportunities
    render_quadrant(
        &mut svg,
        PADDING,
        content_y + top_h + QUADRANT_GAP,
        quad_w,
        bottom_h,
        "O - Opportunities",
        &swot.opportunities,
        O_BG,
        O_TITLE,
        O_ACCENT,
    );

    // Bottom-right: Threats
    render_quadrant(
        &mut svg,
        PADDING + quad_w + QUADRANT_GAP,
        content_y + top_h + QUADRANT_GAP,
        quad_w,
        bottom_h,
        "T - Threats",
        &swot.threats,
        T_BG,
        T_TITLE,
        T_ACCENT,
    );

    svg.push_str("</svg>");
    svg
}

fn render_quadrant(
    svg: &mut String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    header: &str,
    items: &[String],
    bg_color: &str,
    title_color: &str,
    accent_color: &str,
) {
    // Quadrant background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
        x, y, w, h, bg_color
    ));

    // Header background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
        x, y, w, HEADER_HEIGHT, title_color
    ));
    // Fill bottom corners of header
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        x,
        y + HEADER_HEIGHT / 2.0,
        w,
        HEADER_HEIGHT / 2.0,
        title_color
    ));

    // Header text
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        x + w / 2.0,
        y + HEADER_HEIGHT / 2.0 + 5.0,
        HEADER_FONT_SIZE,
        accent_color,
        escape_xml(header)
    ));

    // Items with bullet points
    for (i, item) in items.iter().enumerate() {
        let item_y = y + HEADER_HEIGHT + (i as f64 + 0.5) * ITEM_HEIGHT + 4.0;
        let bullet_x = x + QUADRANT_H_PAD;
        let text_x = bullet_x + QUADRANT_H_PAD;

        // Bullet
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
            bullet_x,
            item_y,
            BULLET_RADIUS,
            accent_color
        ));

        // Item text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\">{}</text>",
            text_x,
            item_y + 4.0,
            escape_xml(item)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-swot - Render a SWOT analysis matrix as SVG

Usage: mdd-swot < input.swot

Define four sections (all optional, but at least one must have items):
  strengths { ... }
  weaknesses { ... }
  opportunities { ... }
  threats { ... }

Each line inside a section is one bullet item.

Example:
  strengths {
    Strong brand
    Skilled team
  }
  weaknesses {
    Limited budget
  }
  opportunities {
    Growing market
  }
  threats {
    New competitors
  }
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

    let swot = match parse(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd-swot: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&swot));
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
strengths {
  Item A
  Item B
}
weaknesses {
  Item C
}
opportunities {
  Item D
  Item E
}
threats {
  Item F
}
"#;
        let s = parse(input).unwrap();
        assert_eq!(s.strengths.len(), 2);
        assert_eq!(s.strengths[0], "Item A");
        assert_eq!(s.weaknesses.len(), 1);
        assert_eq!(s.opportunities.len(), 2);
        assert_eq!(s.threats.len(), 1);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
strengths {
  A
}
weaknesses {
  B
}
opportunities {
  C
}
threats {
  D
}
"#;
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_partial() {
        let input = r#"
strengths {
  Only strengths here
}
"#;
        let s = parse(input).unwrap();
        assert_eq!(s.strengths.len(), 1);
        assert_eq!(s.weaknesses.len(), 0);
        assert_eq!(s.opportunities.len(), 0);
        assert_eq!(s.threats.len(), 0);
    }
}
