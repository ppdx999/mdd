use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Metric {
    label: String,
    value: String,
    change: Option<String>,
}

#[derive(Debug)]
struct Kpi {
    metrics: Vec<Metric>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Kpi, String> {
    let mut metrics: Vec<Metric> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // metric "Label" : "Value" [: "Change"]
        if trimmed.starts_with("metric ") {
            let rest = trimmed.strip_prefix("metric ").unwrap().trim();
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() < 2 {
                return Err(format!("Invalid metric syntax: {}", trimmed));
            }
            let label = strip_quotes(parts[0].trim()).to_string();
            let value = strip_quotes(parts[1].trim()).to_string();
            let change = if parts.len() >= 3 {
                Some(strip_quotes(parts[2].trim()).to_string())
            } else {
                None
            };
            metrics.push(Metric {
                label,
                value,
                change,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if metrics.is_empty() {
        return Err("At least 1 metric is required".to_string());
    }

    Ok(Kpi { metrics })
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
const COLOR_DARK: &str = "#333";

const CARD_MIN_WIDTH: f64 = 140.0;
const CARD_H_PAD: f64 = 20.0;
const CARD_HEIGHT: f64 = 90.0;
const CARD_GAP: f64 = 12.0;
const ACCENT_HEIGHT: f64 = 4.0;
const VALUE_FONT_SIZE: f64 = 24.0;
const LABEL_FONT_SIZE: f64 = 12.0;
const CHANGE_FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 40.0;
const BORDER_COLOR: &str = "#e0e0e0";

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

fn change_color(change: &str) -> &'static str {
    let lower = change.to_lowercase();
    if change.starts_with('+') || lower.contains("up") || change.contains("増") {
        "#2e7d32"
    } else if change.starts_with('-') || lower.contains("down") || change.contains("減") {
        "#c62828"
    } else {
        "#666"
    }
}

fn render_svg(kpi: &Kpi) -> String {
    // Compute card widths based on content
    let card_widths: Vec<f64> = kpi
        .metrics
        .iter()
        .map(|m| {
            let value_w = text_width(&m.value) * (VALUE_FONT_SIZE / FONT_SIZE);
            let label_w = text_width(&m.label) * (LABEL_FONT_SIZE / FONT_SIZE);
            let change_w = m
                .change
                .as_ref()
                .map(|c| text_width(c) * (CHANGE_FONT_SIZE / FONT_SIZE))
                .unwrap_or(0.0);
            let content_w = value_w.max(label_w).max(change_w);
            (content_w + CARD_H_PAD * 2.0).max(CARD_MIN_WIDTH)
        })
        .collect();

    let cards_total_w: f64 =
        card_widths.iter().sum::<f64>() + (kpi.metrics.len().saturating_sub(1)) as f64 * CARD_GAP;

    let total_w = PADDING * 2.0 + cards_total_w;
    let total_h = PADDING * 2.0 + CARD_HEIGHT;

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

    // Cards
    let mut card_x = PADDING;
    for (i, metric) in kpi.metrics.iter().enumerate() {
        let cw = card_widths[i];
        let (bg_color, accent_color) = COLORS[i % COLORS.len()];

        // Card background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            card_x, content_y, cw, CARD_HEIGHT, bg_color, BORDER_COLOR
        ));

        // Accent bar (top)
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
            card_x, content_y, cw, ACCENT_HEIGHT + 8.0, accent_color
        ));
        // Fill bottom corners of accent to make it a flat-bottom bar
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            card_x,
            content_y + ACCENT_HEIGHT,
            cw,
            8.0,
            bg_color
        ));

        let center_x = card_x + cw / 2.0;

        // Value text
        let value_y = content_y + ACCENT_HEIGHT + 32.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            center_x,
            value_y,
            VALUE_FONT_SIZE,
            COLOR_DARK,
            escape_xml(&metric.value)
        ));

        // Label text
        let label_y = value_y + 20.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"#666\">{}</text>",
            center_x,
            label_y,
            LABEL_FONT_SIZE,
            escape_xml(&metric.label)
        ));

        // Change text
        if let Some(ref change) = metric.change {
            let change_y = label_y + 16.0;
            let color = change_color(change);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                center_x,
                change_y,
                CHANGE_FONT_SIZE,
                color,
                escape_xml(change)
            ));
        }

        card_x += cw + CARD_GAP;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-kpi - Render KPI metric cards as SVG

Usage: mdd-kpi < input.kpi

Each line defines a metric card:
  metric \"<label>\" : \"<value>\" [: \"<change>\"]
The optional change field is colored green for \"+\" / red for \"-\".

Example:
  metric \"Uptime\" : \"99.97%\"
  metric \"Latency\" : \"142ms\" : \"-12%\"
  metric \"Errors\" : \"0.02%\" : \"+0.01%\"
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

    let kpi = match parse(&input) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("mdd-kpi: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&kpi));
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
metric "Revenue" : "$1M"
metric "Users" : "5000"
"#;
        let kpi = parse(input).unwrap();
        assert_eq!(kpi.metrics.len(), 2);
        assert_eq!(kpi.metrics[0].label, "Revenue");
        assert_eq!(kpi.metrics[0].value, "$1M");
        assert!(kpi.metrics[0].change.is_none());
    }

    #[test]
    fn parse_with_change() {
        let input = r#"
metric "Sales" : "100" : "+15%"
"#;
        let kpi = parse(input).unwrap();
        assert_eq!(kpi.metrics[0].change.as_deref(), Some("+15%"));
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
metric "Revenue" : "$1M"
"#;
        let kpi = parse(input).unwrap();
        let svg = render_svg(&kpi);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_error_no_metrics() {
        let input = r#"
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn change_color_positive() {
        assert_eq!(change_color("+15%"), "#2e7d32");
        assert_eq!(change_color("-3%"), "#c62828");
        assert_eq!(change_color("neutral"), "#666");
    }
}
