use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Option {
    label: String,
    items: Vec<String>,
}

#[derive(Debug)]
struct Compare {
    title: std::option::Option<String>,
    options: Vec<Option>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Compare, String> {
    let mut title: std::option::Option<String> = None;
    let mut options: Vec<Option> = Vec::new();
    let mut current_label: std::option::Option<String> = None;
    let mut current_items: Vec<String> = Vec::new();

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

        // option "Label" {
        if trimmed.starts_with("option ") {
            let rest = trimmed.strip_prefix("option ").unwrap().trim();
            let (label, _) = parse_option_header(rest)?;
            current_label = Some(label);
            current_items = Vec::new();
            continue;
        }

        // closing brace
        if trimmed == "}" {
            if let Some(label) = current_label.take() {
                options.push(Option {
                    label,
                    items: std::mem::take(&mut current_items),
                });
            } else {
                return Err("Unexpected '}'".to_string());
            }
            continue;
        }

        // inside an option block
        if current_label.is_some() {
            current_items.push(trimmed.to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_label.is_some() {
        return Err("Unclosed option (missing '}')".to_string());
    }

    if options.len() < 2 {
        return Err("At least 2 options are required".to_string());
    }

    if options.len() > 3 {
        return Err("At most 3 options are supported".to_string());
    }

    Ok(Compare { title, options })
}

fn parse_option_header(rest: &str) -> Result<(String, bool), String> {
    let rest = rest.trim();
    if rest.starts_with('"') {
        let end_quote = rest[1..]
            .find('"')
            .ok_or("Unterminated quote in option header")?;
        let label = rest[1..=end_quote].to_string();
        let after_quote = rest[end_quote + 2..].trim();
        if after_quote == "{" || after_quote.is_empty() {
            return Ok((label, after_quote == "{"));
        }
        return Err(format!("Expected '{{' after label, got: {}", after_quote));
    }
    if rest.ends_with('{') {
        let label = rest.trim_end_matches('{').trim();
        return Ok((label.to_string(), true));
    }
    Ok((rest.to_string(), false))
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

const COLUMN_MIN_WIDTH: f64 = 180.0;
const COLUMN_H_PAD: f64 = 20.0;
const HEADER_HEIGHT: f64 = 40.0;
const ITEM_HEIGHT: f64 = 28.0;
const COLUMN_GAP: f64 = 16.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const BULLET_RADIUS: f64 = 3.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"),
    ("#e8f5e9", "#2e7d32"),
    ("#fff8e1", "#f57f17"),
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

fn render_svg(compare: &Compare) -> String {
    let num_options = compare.options.len();
    let max_items = compare.options.iter().map(|o| o.items.len()).max().unwrap_or(0);

    // Compute column widths
    let column_widths: Vec<f64> = compare
        .options
        .iter()
        .map(|opt| {
            let label_w = text_width(&opt.label) + COLUMN_H_PAD * 2.0;
            let max_item_w = opt
                .items
                .iter()
                .map(|item| text_width(item) + COLUMN_H_PAD * 2.0 + BULLET_RADIUS * 2.0 + 12.0)
                .fold(0.0_f64, f64::max);
            label_w.max(max_item_w).max(COLUMN_MIN_WIDTH)
        })
        .collect();

    let total_columns_w: f64 = column_widths.iter().sum::<f64>()
        + COLUMN_GAP * (num_options.saturating_sub(1)) as f64;

    let title_space = if compare.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let column_body_h = max_items as f64 * ITEM_HEIGHT;
    let column_h = HEADER_HEIGHT + column_body_h;

    let total_w = PADDING * 2.0 + total_columns_w;
    let total_h = PADDING * 2.0 + title_space + column_h;

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
    let content_y = if let Some(ref title) = compare.title {
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

    // Columns
    let mut col_x = PADDING;
    for (i, opt) in compare.options.iter().enumerate() {
        let col_w = column_widths[i];
        let (bg_color, header_text_color) = COLORS[i % COLORS.len()];

        // Column background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"#fafafa\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
            col_x, content_y, col_w, column_h
        ));

        // Header bar
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
            col_x, content_y, col_w, HEADER_HEIGHT, bg_color
        ));
        // Fill bottom corners of header
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            col_x,
            content_y + HEADER_HEIGHT / 2.0,
            col_w,
            HEADER_HEIGHT / 2.0,
            bg_color
        ));

        // Header text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            col_x + col_w / 2.0,
            content_y + HEADER_HEIGHT / 2.0 + 5.0,
            header_text_color,
            escape_xml(&opt.label)
        ));

        // Items
        for (j, item) in opt.items.iter().enumerate() {
            let item_y = content_y + HEADER_HEIGHT + j as f64 * ITEM_HEIGHT;
            let text_y = item_y + ITEM_HEIGHT / 2.0 + 4.5;
            let bullet_x = col_x + COLUMN_H_PAD;
            let bullet_y = item_y + ITEM_HEIGHT / 2.0;

            // Bullet dot
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                bullet_x, bullet_y, BULLET_RADIUS, header_text_color
            ));

            // Item text
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>",
                bullet_x + BULLET_RADIUS * 2.0 + 6.0,
                text_y,
                escape_xml(item)
            ));
        }

        col_x += col_w + COLUMN_GAP;
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

    let compare = match parse(&input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mdd-compare: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&compare));
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
title "比較"
option "A" {
  項目1
  項目2
}
option "B" {
  項目3
}
"#;
        let c = parse(input).unwrap();
        assert_eq!(c.title.as_deref(), Some("比較"));
        assert_eq!(c.options.len(), 2);
        assert_eq!(c.options[0].label, "A");
        assert_eq!(c.options[0].items.len(), 2);
        assert_eq!(c.options[1].label, "B");
        assert_eq!(c.options[1].items.len(), 1);
    }

    #[test]
    fn parse_three_options() {
        let input = r#"
option "X" {
  a
}
option "Y" {
  b
}
option "Z" {
  c
}
"#;
        let c = parse(input).unwrap();
        assert!(c.title.is_none());
        assert_eq!(c.options.len(), 3);
        assert_eq!(c.options[2].label, "Z");
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
option "A" {
  item1
}
option "B" {
  item2
}
"#;
        let c = parse(input).unwrap();
        let svg = render_svg(&c);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn error_single_option() {
        let input = r#"
option "Only" {
  x
}
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_unclosed_option() {
        let input = r#"
option "A" {
  x
"#;
        assert!(parse(input).is_err());
    }
}
