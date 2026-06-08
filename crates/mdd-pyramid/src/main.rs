use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Level {
    label: String,
    description: Vec<String>,
}

#[derive(Debug)]
struct Pyramid {
    levels: Vec<Level>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Pyramid, String> {
    let mut levels: Vec<Level> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Name : "Description"
        // Name
        if let Some(colon_pos) = trimmed.find(" : ") {
            let label = trimmed[..colon_pos].trim().to_string();
            let desc_part = trimmed[colon_pos + 3..].trim();
            let (desc, consumed) = parse_multiline_desc(desc_part, &lines, i)?;
            i += consumed;
            levels.push(Level { label, description: desc });
        } else {
            levels.push(Level {
                label: trimmed.to_string(),
                description: Vec::new(),
            });
        }
        i += 1;
    }

    if levels.len() < 2 {
        return Err("At least 2 levels are required".to_string());
    }

    Ok(Pyramid { levels })
}

fn parse_multiline_desc(start: &str, lines: &[&str], current: usize) -> Result<(Vec<String>, usize), String> {
    let content = start.strip_prefix('"').unwrap_or(start);
    if let Some(end) = content.find('"') {
        return Ok((vec![content[..end].to_string()], 0));
    }
    let mut desc_lines = vec![content.to_string()];
    let mut extra = 0;
    for j in (current + 1)..lines.len() {
        extra += 1;
        let line = lines[j].trim();
        if line.ends_with('"') {
            desc_lines.push(line[..line.len() - 1].to_string());
            return Ok((desc_lines, extra));
        }
        desc_lines.push(line.to_string());
    }
    Err("Unterminated description (missing closing \")".to_string())
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 12.0;
const COLOR_DARK: &str = "#333";
const COLOR_DESC: &str = "#666";
const DESC_LINE_COLOR: &str = "#ccc";

const LEVEL_HEIGHT: f64 = 50.0;
const MAX_WIDTH: f64 = 500.0;
const PADDING: f64 = 40.0;
const DESC_GAP: f64 = 30.0;

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

fn render_svg(pyramid: &Pyramid) -> String {
    let n = pyramid.levels.len();

    let has_desc = pyramid.levels.iter().any(|l| !l.description.is_empty());
    let desc_area_w = if has_desc {
        let max_desc_w = pyramid
            .levels
            .iter()
            .flat_map(|l| l.description.iter())
            .map(|d| text_width(d))
            .fold(0.0_f64, f64::max);
        DESC_GAP + max_desc_w + 16.0
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + MAX_WIDTH + desc_area_w;
    let total_h = PADDING * 2.0 + n as f64 * LEVEL_HEIGHT;
    let center_x = PADDING + MAX_WIDTH / 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let pyramid_top_y = PADDING;

    // Draw each level as a polygon
    for i in 0..n {
        let (bg, fg) = COLORS[i % COLORS.len()];

        // Top edge of this level
        let top_y = pyramid_top_y + i as f64 * LEVEL_HEIGHT;
        // Bottom edge of this level
        let bot_y = top_y + LEVEL_HEIGHT;

        // Width at top and bottom of this band
        let top_half_w = MAX_WIDTH * i as f64 / n as f64 / 2.0;
        let bot_half_w = MAX_WIDTH * (i + 1) as f64 / n as f64 / 2.0;

        // Polygon points: top-left, top-right, bottom-right, bottom-left
        let tl_x = center_x - top_half_w;
        let tr_x = center_x + top_half_w;
        let bl_x = center_x - bot_half_w;
        let br_x = center_x + bot_half_w;

        let points = if i == 0 {
            // Top level is a triangle (peak)
            format!(
                "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                center_x, top_y, br_x, bot_y, bl_x, bot_y
            )
        } else {
            format!(
                "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                tl_x, top_y, tr_x, top_y, br_x, bot_y, bl_x, bot_y
            )
        };

        svg.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"white\" stroke-width=\"2\"/>",
            points, bg
        ));

        // Label text centered in the band
        let label = &pyramid.levels[i].label;
        let label_y = top_y + LEVEL_HEIGHT / 2.0 + 5.0;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" font-weight=\"bold\">{}</text>",
            center_x, label_y, fg, escape_xml(label)
        ));

        // Description on the right side with horizontal line
        if !pyramid.levels[i].description.is_empty() {
            let desc = &pyramid.levels[i].description;
            let line_y = top_y + LEVEL_HEIGHT / 2.0;
            let line_start_x = center_x + bot_half_w;
            let desc_x = PADDING + MAX_WIDTH + DESC_GAP;

            // Horizontal line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                line_start_x, line_y, desc_x - 8.0, line_y, DESC_LINE_COLOR
            ));
            // Small dot at the start of line
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"{}\"/>",
                line_start_x, line_y, fg
            ));

            // Description text (multi-line)
            let desc_start_y = line_y - (desc.len() as f64 - 1.0) * DESC_FONT_SIZE * 0.7;
            for (j, line) in desc.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    desc_x,
                    desc_start_y + j as f64 * DESC_FONT_SIZE * 1.4 + DESC_FONT_SIZE * 0.35,
                    DESC_FONT_SIZE,
                    COLOR_DESC,
                    escape_xml(line)
                ));
            }
        }
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

    let pyramid = match parse(&input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("mdd-pyramid: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&pyramid));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "Top\nMiddle\nBottom\n";
        let p = parse(input).unwrap();
        assert_eq!(p.levels.len(), 3);
        assert_eq!(p.levels[0].label, "Top");
        assert_eq!(p.levels[1].label, "Middle");
        assert_eq!(p.levels[2].label, "Bottom");
        assert!(p.levels[0].description.is_empty());
    }

    #[test]
    fn parse_with_desc() {
        let input = r#"
Strategy : "Long-term direction"
Tactics : "Quarterly plans"
Operations : "Daily execution"
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.levels.len(), 3);
        assert_eq!(p.levels[0].label, "Strategy");
        assert_eq!(p.levels[0].description, vec!["Long-term direction"]);
        assert_eq!(p.levels[2].label, "Operations");
        assert_eq!(p.levels[2].description, vec!["Daily execution"]);
    }

    #[test]
    fn parse_multiline_desc() {
        let input = "Top : \"Line one\nLine two\"\nBottom\n";
        let p = parse(input).unwrap();
        assert_eq!(p.levels[0].description, vec!["Line one", "Line two"]);
        assert!(p.levels[1].description.is_empty());
    }

    #[test]
    fn parse_requires_at_least_two_levels() {
        let input = "OnlyOne\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "Top\nBottom\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
        assert!(svg.contains("Top"));
        assert!(svg.contains("Bottom"));
        assert!(svg.contains("<polygon"));
    }
}
