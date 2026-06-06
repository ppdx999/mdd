use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ScaleItem {
    label: String,
    value: f64,
}

#[derive(Debug)]
struct Scale {
    title: Option<String>,
    unit: Option<String>,
    items: Vec<ScaleItem>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Scale, String> {
    let mut title: Option<String> = None;
    let mut unit: Option<String> = None;
    let mut items: Vec<ScaleItem> = Vec::new();

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

        // unit "..."
        if trimmed.starts_with("unit ") {
            let rest = trimmed.strip_prefix("unit ").unwrap().trim();
            unit = Some(strip_quotes(rest).to_string());
            continue;
        }

        // item Label : value
        if trimmed.starts_with("item ") {
            let rest = trimmed.strip_prefix("item ").unwrap().trim();
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid item syntax (expected 'item Label : value'): {}", trimmed));
            }
            let label = parts[0].trim().to_string();
            let value_str = parts[1].trim();
            let value: f64 = value_str
                .parse()
                .map_err(|_| format!("Invalid number '{}' in: {}", value_str, trimmed))?;
            if value < 0.0 {
                return Err(format!("Negative value not allowed: {}", trimmed));
            }
            items.push(ScaleItem { label, value });
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if items.len() < 2 {
        return Err("At least 2 items are required".to_string());
    }

    Ok(Scale { title, unit, items })
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

const MAX_BAR_WIDTH: f64 = 400.0;
const BAR_HEIGHT: f64 = 32.0;
const BAR_GAP: f64 = 8.0;
const LABEL_HEIGHT: f64 = 20.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const VALUE_FONT_SIZE: f64 = 11.0;

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

fn format_value(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

fn render_svg(scale: &Scale) -> String {
    let max_value = scale
        .items
        .iter()
        .map(|i| i.value)
        .fold(0.0_f64, f64::max);

    // Compute label column width
    let label_col_w = scale
        .items
        .iter()
        .map(|i| text_width(&i.label))
        .fold(0.0_f64, f64::max)
        + 12.0;

    // Compute value text max width for right side
    let value_texts: Vec<String> = scale
        .items
        .iter()
        .map(|i| {
            let v = format_value(i.value);
            match &scale.unit {
                Some(u) => format!("{} {}", v, u),
                None => v,
            }
        })
        .collect();
    let value_col_w = value_texts
        .iter()
        .map(|t| text_width(t))
        .fold(0.0_f64, f64::max)
        + 12.0;

    let title_space = if scale.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let row_height = LABEL_HEIGHT + BAR_HEIGHT + BAR_GAP;
    let content_h = scale.items.len() as f64 * row_height;
    let total_w = PADDING + label_col_w + MAX_BAR_WIDTH + value_col_w + PADDING;
    let total_h = PADDING + title_space + content_h + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Title
    let content_y = if let Some(ref title) = scale.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            escape_xml(title)
        ));
        PADDING + TITLE_HEIGHT + TITLE_GAP
    } else {
        PADDING
    };

    // Render items
    let bar_x = PADDING + label_col_w;

    for (i, item) in scale.items.iter().enumerate() {
        let (bg_color, fg_color) = COLORS[i % COLORS.len()];
        let row_y = content_y + i as f64 * row_height;

        // Label above the bar
        let label_y = row_y + LABEL_HEIGHT - 4.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>",
            PADDING,
            label_y,
            FONT_SIZE,
            escape_xml(&item.label)
        ));

        // Bar
        let bar_y = row_y + LABEL_HEIGHT;
        let bar_w = if max_value > 0.0 {
            (item.value / max_value) * MAX_BAR_WIDTH
        } else {
            0.0
        };
        let bar_w = bar_w.max(2.0); // minimum visible width

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            bar_x, bar_y, bar_w, BAR_HEIGHT, bg_color, fg_color
        ));

        // Value text at the right end of the bar
        let value_text = &value_texts[i];
        let value_x = bar_x + bar_w + 8.0;
        let value_y = bar_y + BAR_HEIGHT / 2.0 + 4.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            value_x,
            value_y,
            VALUE_FONT_SIZE,
            fg_color,
            escape_xml(value_text)
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

    let scale = match parse(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd-scale: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&scale));
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
item A : 100
item B : 50
"#;
        let s = parse(input).unwrap();
        assert!(s.title.is_none());
        assert!(s.unit.is_none());
        assert_eq!(s.items.len(), 2);
        assert_eq!(s.items[0].label, "A");
        assert_eq!(s.items[0].value, 100.0);
        assert_eq!(s.items[1].label, "B");
        assert_eq!(s.items[1].value, 50.0);
    }

    #[test]
    fn parse_with_unit() {
        let input = r#"
title "Test"
unit "kg"
item X : 10
item Y : 20
"#;
        let s = parse(input).unwrap();
        assert_eq!(s.title.as_deref(), Some("Test"));
        assert_eq!(s.unit.as_deref(), Some("kg"));
        assert_eq!(s.items.len(), 2);
    }

    #[test]
    fn parse_requires_two_items() {
        let input = "item A : 100\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
item A : 100
item B : 50
"#;
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
