use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Stage {
    label: String,
    value: Option<f64>,
    description: Vec<String>,
}

#[derive(Debug)]
struct Funnel {
    stages: Vec<Stage>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Funnel, String> {
    let mut stages: Vec<Stage> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // stage Label : value : "desc"  OR  stage Label : "desc"  OR  stage Label : value  OR  stage Label
        if trimmed.starts_with("stage ") {
            let rest = trimmed.strip_prefix("stage ").unwrap().trim();
            let parts: Vec<&str> = rest.splitn(3, " : ").collect();
            match parts.len() {
                1 => {
                    stages.push(Stage {
                        label: parts[0].trim().to_string(),
                        value: None,
                        description: Vec::new(),
                    });
                }
                2 => {
                    let label = parts[0].trim().to_string();
                    let second = parts[1].trim();
                    if second.starts_with('"') {
                        let (desc, consumed) = parse_multiline_desc(second, &lines, i)?;
                        i += consumed;
                        stages.push(Stage {
                            label,
                            value: None,
                            description: desc,
                        });
                    } else {
                        let value = second
                            .parse::<f64>()
                            .map_err(|_| format!("Invalid value: {}", second))?;
                        if value < 0.0 {
                            return Err(format!("Negative value: {}", value));
                        }
                        stages.push(Stage {
                            label,
                            value: Some(value),
                            description: Vec::new(),
                        });
                    }
                }
                3 => {
                    let label = parts[0].trim().to_string();
                    let value_str = parts[1].trim();
                    let value = value_str
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid value: {}", value_str))?;
                    if value < 0.0 {
                        return Err(format!("Negative value: {}", value));
                    }
                    let third = parts[2].trim();
                    let (desc, consumed) = parse_multiline_desc(third, &lines, i)?;
                    i += consumed;
                    stages.push(Stage {
                        label,
                        value: Some(value),
                        description: desc,
                    });
                }
                _ => unreachable!(),
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if stages.len() < 2 {
        return Err("At least 2 stages are required".to_string());
    }

    Ok(Funnel { stages })
}

/// Parse a quoted description that may span multiple lines.
/// Returns (lines, extra_lines_consumed).
fn parse_multiline_desc(start: &str, lines: &[&str], current: usize) -> Result<(Vec<String>, usize), String> {
    let content = start.strip_prefix('"').unwrap_or(start);
    // Single-line case: closing quote on same line
    if let Some(end) = content.find('"') {
        return Ok((vec![content[..end].to_string()], 0));
    }
    // Multi-line: accumulate until closing quote
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
// Constants
// ---------------------------------------------------------------------------

const STAGE_HEIGHT: f64 = 50.0;
const STAGE_GAP: f64 = 4.0;
const MAX_WIDTH: f64 = 400.0;
const MIN_WIDTH: f64 = 80.0;
const PADDING: f64 = 40.0;
const LABEL_FONT_SIZE: f64 = 14.0;
const VALUE_FONT_SIZE: f64 = 12.0;
const DESC_FONT_SIZE: f64 = 12.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";
const COLOR_DESC: &str = "#666";
const DESC_GAP: f64 = 30.0;
const DESC_LINE_COLOR: &str = "#ccc";

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

// ---------------------------------------------------------------------------
// Text & sizing
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;

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

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(funnel: &Funnel) -> String {
    let n = funnel.stages.len();

    // Compute widths for each stage
    let widths: Vec<f64> = compute_widths(funnel);

    let has_desc = funnel.stages.iter().any(|s| !s.description.is_empty());
    let desc_area_w = if has_desc {
        let max_desc_w = funnel
            .stages
            .iter()
            .flat_map(|s| s.description.iter())
            .map(|d| text_width(d))
            .fold(0.0_f64, f64::max);
        DESC_GAP + max_desc_w + 16.0
    } else {
        0.0
    };

    let total_h =
        PADDING + n as f64 * STAGE_HEIGHT + (n - 1) as f64 * STAGE_GAP + PADDING;
    let total_w = PADDING * 2.0 + MAX_WIDTH + desc_area_w;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let center_x = PADDING + MAX_WIDTH / 2.0;

    let content_y = PADDING;

    // Render each stage as a trapezoid
    for i in 0..n {
        let top_w = widths[i];
        let bottom_w = if i + 1 < n { widths[i + 1] } else { top_w * 0.7 };
        let bottom_w = bottom_w.max(MIN_WIDTH * 0.5);

        let y_top = content_y + i as f64 * (STAGE_HEIGHT + STAGE_GAP);
        let y_bottom = y_top + STAGE_HEIGHT;

        let top_left = center_x - top_w / 2.0;
        let top_right = center_x + top_w / 2.0;
        let bottom_left = center_x - bottom_w / 2.0;
        let bottom_right = center_x + bottom_w / 2.0;

        let (bg_color, text_color) = COLORS[i % COLORS.len()];

        // Trapezoid polygon
        svg.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"white\" stroke-width=\"1\"/>",
            top_left, y_top,
            top_right, y_top,
            bottom_right, y_bottom,
            bottom_left, y_bottom,
            bg_color
        ));

        // Label centered
        let label_y = y_top + STAGE_HEIGHT / 2.0 + LABEL_FONT_SIZE / 3.0;
        let stage = &funnel.stages[i];

        if let Some(value) = stage.value {
            // Label on left-center, value on right
            let label_tw = text_width(&stage.label);
            let value_str = format_value(value);
            let value_tw = text_width(&value_str);
            let combined_w = label_tw + 16.0 + value_tw;

            if combined_w < top_w * 0.9 {
                // Enough room: label centered, value to the right
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    center_x - value_tw / 2.0 - 4.0,
                    label_y,
                    LABEL_FONT_SIZE,
                    text_color,
                    escape_xml(&stage.label)
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" fill=\"{}\" opacity=\"0.7\">{}</text>",
                    center_x + top_w / 2.0 - 12.0,
                    label_y,
                    VALUE_FONT_SIZE,
                    text_color,
                    escape_xml(&value_str)
                ));
            } else {
                // Not enough room: just center label
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    center_x,
                    label_y,
                    LABEL_FONT_SIZE,
                    text_color,
                    escape_xml(&stage.label)
                ));
            }
        } else {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                center_x,
                label_y,
                LABEL_FONT_SIZE,
                text_color,
                escape_xml(&stage.label)
            ));
        }

        // Description on the right side with horizontal line
        if !stage.description.is_empty() {
            let line_y = y_top + STAGE_HEIGHT / 2.0;
            let line_start_x = center_x + top_w / 2.0;
            let desc_x = PADDING + MAX_WIDTH + DESC_GAP;

            // Horizontal line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                line_start_x, line_y, desc_x - 8.0, line_y, DESC_LINE_COLOR
            ));
            // Small dot at the start of line
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"{}\"/>",
                line_start_x, line_y, text_color
            ));

            // Description text (multi-line)
            let desc_start_y = line_y - (stage.description.len() as f64 - 1.0) * DESC_FONT_SIZE * 0.7;
            for (j, line) in stage.description.iter().enumerate() {
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

fn compute_widths(funnel: &Funnel) -> Vec<f64> {
    let n = funnel.stages.len();
    let has_values = funnel.stages.iter().any(|s| s.value.is_some());

    if has_values {
        let max_value = funnel
            .stages
            .iter()
            .filter_map(|s| s.value)
            .fold(0.0_f64, f64::max);

        if max_value <= 0.0 {
            // Fallback to fixed ratio
            return fixed_ratio_widths(n);
        }

        funnel
            .stages
            .iter()
            .map(|s| {
                let v = s.value.unwrap_or(0.0);
                let ratio = v / max_value;
                (MIN_WIDTH + (MAX_WIDTH - MIN_WIDTH) * ratio).max(MIN_WIDTH)
            })
            .collect()
    } else {
        fixed_ratio_widths(n)
    }
}

fn fixed_ratio_widths(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let ratio = 1.0 - (i as f64 / n as f64) * 0.7;
            (MAX_WIDTH * ratio).max(MIN_WIDTH)
        })
        .collect()
}

fn format_value(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-funnel - Render a funnel diagram as SVG

Usage: mdd-funnel < input.funnel

Each line defines a stage: \"stage Name\". Optionally add a numeric
value with \" : value\" and/or a description with \" : \"text\"\".
At least 2 stages are required. Widths are proportional to values.

Example:
  stage Awareness : \"Ads and social media\"
  stage Interest : \"Site visits\"
  stage Evaluation : \"Compare options\"
  stage Purchase : \"Buy\"
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

    let funnel = match parse(&input) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("mdd-funnel: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&funnel));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "stage A\nstage B\nstage C\n";
        let f = parse(input).unwrap();
        assert_eq!(f.stages.len(), 3);
        assert_eq!(f.stages[0].label, "A");
        assert_eq!(f.stages[1].label, "B");
        assert_eq!(f.stages[2].label, "C");
        assert!(f.stages[0].value.is_none());
        assert!(f.stages[0].description.is_empty());
    }

    #[test]
    fn parse_with_values() {
        let input = r#"
stage Leads : 1000
stage Prospects : 500
stage Customers : 100
"#;
        let f = parse(input).unwrap();
        assert_eq!(f.stages.len(), 3);
        assert_eq!(f.stages[0].label, "Leads");
        assert_eq!(f.stages[0].value, Some(1000.0));
        assert_eq!(f.stages[2].value, Some(100.0));
    }

    #[test]
    fn parse_with_description() {
        let input = "stage A : \"First step\"\nstage B : \"Second step\"\n";
        let f = parse(input).unwrap();
        assert_eq!(f.stages[0].description, vec!["First step"]);
        assert!(f.stages[0].value.is_none());
    }

    #[test]
    fn parse_with_value_and_description() {
        let input = "stage A : 1000 : \"Top of funnel\"\nstage B : 500 : \"Middle\"\n";
        let f = parse(input).unwrap();
        assert_eq!(f.stages[0].value, Some(1000.0));
        assert_eq!(f.stages[0].description, vec!["Top of funnel"]);
    }

    #[test]
    fn parse_multiline_description() {
        let input = "stage A : \"Line one\nLine two\nLine three\"\nstage B\n";
        let f = parse(input).unwrap();
        assert_eq!(f.stages[0].description, vec!["Line one", "Line two", "Line three"]);
    }

    #[test]
    fn render_produces_svg() {
        let input = "stage A\nstage B\n";
        let f = parse(input).unwrap();
        let svg = render_svg(&f);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
        assert!(svg.contains("polygon"));
    }

    #[test]
    fn parse_error_too_few_stages() {
        let input = "stage A\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_japanese() {
        let input = "stage リード : 1000\nstage 受注 : 40\n";
        let f = parse(input).unwrap();
        assert_eq!(f.stages[0].label, "リード");
    }
}
