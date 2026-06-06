use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Stage {
    label: String,
    value: Option<f64>,
}

#[derive(Debug)]
struct Funnel {
    title: Option<String>,
    stages: Vec<Stage>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Funnel, String> {
    let mut title: Option<String> = None;
    let mut stages: Vec<Stage> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // title "..."
        if trimmed.starts_with("title ") {
            let rest = trimmed.strip_prefix("title ").unwrap().trim();
            title = Some(strip_quotes(rest).to_string());
            continue;
        }

        // stage Label : value  OR  stage Label
        if trimmed.starts_with("stage ") {
            let rest = trimmed.strip_prefix("stage ").unwrap().trim();
            if let Some(colon_pos) = rest.rfind(':') {
                let label = rest[..colon_pos].trim().to_string();
                let value_str = rest[colon_pos + 1..].trim();
                let value = value_str
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid value: {}", value_str))?;
                if value < 0.0 {
                    return Err(format!("Negative value: {}", value));
                }
                stages.push(Stage {
                    label,
                    value: Some(value),
                });
            } else {
                stages.push(Stage {
                    label: rest.to_string(),
                    value: None,
                });
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if stages.len() < 2 {
        return Err("At least 2 stages are required".to_string());
    }

    Ok(Funnel { title, stages })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STAGE_HEIGHT: f64 = 50.0;
const STAGE_GAP: f64 = 4.0;
const MAX_WIDTH: f64 = 400.0;
const MIN_WIDTH: f64 = 80.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const LABEL_FONT_SIZE: f64 = 14.0;
const VALUE_FONT_SIZE: f64 = 12.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

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

    let title_space = if funnel.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let total_h =
        PADDING + title_space + n as f64 * STAGE_HEIGHT + (n - 1) as f64 * STAGE_GAP + PADDING;
    let total_w = PADDING * 2.0 + MAX_WIDTH;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let center_x = total_w / 2.0;

    // Title
    let content_y = if let Some(ref title) = funnel.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>",
            center_x, title_y, escape_xml(title)
        ));
        PADDING + TITLE_HEIGHT + TITLE_GAP
    } else {
        PADDING
    };

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

fn main() {
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
        assert!(f.title.is_none());
        assert_eq!(f.stages.len(), 3);
        assert_eq!(f.stages[0].label, "A");
        assert_eq!(f.stages[1].label, "B");
        assert_eq!(f.stages[2].label, "C");
        assert!(f.stages[0].value.is_none());
    }

    #[test]
    fn parse_with_values() {
        let input = r#"
title "Sales Funnel"
stage Leads : 1000
stage Prospects : 500
stage Customers : 100
"#;
        let f = parse(input).unwrap();
        assert_eq!(f.title.as_deref(), Some("Sales Funnel"));
        assert_eq!(f.stages.len(), 3);
        assert_eq!(f.stages[0].label, "Leads");
        assert_eq!(f.stages[0].value, Some(1000.0));
        assert_eq!(f.stages[2].value, Some(100.0));
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
        let input = "title \"営業ファネル\"\nstage リード : 1000\nstage 受注 : 40\n";
        let f = parse(input).unwrap();
        assert_eq!(f.title.as_deref(), Some("営業ファネル"));
        assert_eq!(f.stages[0].label, "リード");
    }
}
