use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ListItem {
    label: String,
    description: Option<String>,
}

#[derive(Debug)]
struct ListV {
    title: Option<String>,
    items: Vec<ListItem>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ListV, String> {
    let mut title: Option<String> = None;
    let mut items: Vec<ListItem> = Vec::new();

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

        // item "Label" : "Description" or item "Label"
        if trimmed.starts_with("item ") {
            let rest = trimmed.strip_prefix("item ").unwrap().trim();
            let (label, description) = parse_item(rest)?;
            items.push(ListItem { label, description });
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if items.is_empty() {
        return Err("At least 1 item is required".to_string());
    }

    Ok(ListV { title, items })
}

fn parse_item(s: &str) -> Result<(String, Option<String>), String> {
    let s = s.trim();
    if !s.starts_with('"') {
        return Err(format!("Expected quoted label, got: {}", s));
    }

    let end_quote = s[1..]
        .find('"')
        .ok_or("Unterminated quote in item label")?;
    let label = s[1..=end_quote].to_string();
    let rest = s[end_quote + 2..].trim();

    if rest.is_empty() {
        return Ok((label, None));
    }

    if rest.starts_with(':') {
        let desc_part = rest[1..].trim();
        let desc = strip_quotes(desc_part).to_string();
        if desc.is_empty() {
            return Ok((label, None));
        }
        return Ok((label, Some(desc)));
    }

    Err(format!("Expected ':' after label, got: {}", rest))
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
const DESC_FONT_SIZE: f64 = 11.0;
const COLOR_DARK: &str = "#333";

const BADGE_RADIUS: f64 = 16.0;
const ITEM_H_PAD: f64 = 16.0;
const ITEM_MIN_HEIGHT: f64 = 48.0;
const ITEM_GAP: f64 = 8.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const SEPARATOR_COLOR: &str = "#e0e0e0";

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

fn render_svg(list: &ListV) -> String {
    let title_space = if list.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    // Compute item heights and content width
    let badge_area = BADGE_RADIUS * 2.0 + ITEM_H_PAD;
    let mut max_content_w: f64 = 0.0;

    for item in &list.items {
        let label_w = text_width(&item.label);
        let desc_w = item
            .description
            .as_ref()
            .map(|d| text_width(d))
            .unwrap_or(0.0);
        let w = label_w.max(desc_w);
        if w > max_content_w {
            max_content_w = w;
        }
    }

    // Title width consideration
    if let Some(ref t) = list.title {
        let tw = text_width(t);
        if tw > max_content_w {
            max_content_w = tw;
        }
    }

    let total_w = PADDING * 2.0 + badge_area + max_content_w + ITEM_H_PAD;

    // Calculate item heights
    let item_heights: Vec<f64> = list
        .items
        .iter()
        .map(|item| {
            if item.description.is_some() {
                ITEM_MIN_HEIGHT + DESC_FONT_SIZE + 4.0
            } else {
                ITEM_MIN_HEIGHT
            }
        })
        .collect();

    let separators_h = if list.items.len() > 1 {
        (list.items.len() - 1) as f64 * ITEM_GAP
    } else {
        0.0
    };

    let items_total_h: f64 = item_heights.iter().sum::<f64>() + separators_h;
    let total_h = PADDING * 2.0 + title_space + items_total_h;

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
    let mut y = PADDING;
    if let Some(ref title) = list.title {
        let title_y = y + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            escape_xml(title)
        ));
        y += TITLE_HEIGHT + TITLE_GAP;
    }

    // Items
    for (i, item) in list.items.iter().enumerate() {
        let (bg_color, fg_color) = COLORS[i % COLORS.len()];
        let item_h = item_heights[i];

        // Badge circle
        let badge_cx = PADDING + BADGE_RADIUS;
        let badge_cy = y + item_h / 2.0;
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" />",
            badge_cx, badge_cy, BADGE_RADIUS, bg_color
        ));

        // Badge number
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            badge_cx,
            badge_cy + 5.0,
            FONT_SIZE,
            fg_color,
            i + 1
        ));

        // Label text (bold)
        let text_x = PADDING + badge_area;
        let label_y = if item.description.is_some() {
            y + item_h / 2.0 - 2.0
        } else {
            y + item_h / 2.0 + 5.0
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
            text_x,
            label_y,
            escape_xml(&item.label)
        ));

        // Description text
        if let Some(ref desc) = item.description {
            let desc_y = label_y + DESC_FONT_SIZE + 6.0;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#666\">{}</text>",
                text_x,
                desc_y,
                DESC_FONT_SIZE,
                escape_xml(desc)
            ));
        }

        y += item_h;

        // Separator line between items
        if i < list.items.len() - 1 {
            let sep_y = y + ITEM_GAP / 2.0;
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" />",
                PADDING,
                sep_y,
                total_w - PADDING,
                sep_y,
                SEPARATOR_COLOR
            ));
            y += ITEM_GAP;
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

    let list = match parse(&input) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mdd-list-v: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&list));
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
title "My List"
item "First"
item "Second"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.title.as_deref(), Some("My List"));
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "First");
        assert!(list.items[0].description.is_none());
        assert_eq!(list.items[1].label, "Second");
    }

    #[test]
    fn parse_with_desc() {
        let input = r#"
item "Label" : "Description"
item "Other" : "Details"
"#;
        let list = parse(input).unwrap();
        assert!(list.title.is_none());
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "Label");
        assert_eq!(list.items[0].description.as_deref(), Some("Description"));
        assert_eq!(list.items[1].label, "Other");
        assert_eq!(list.items[1].description.as_deref(), Some("Details"));
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
item "Test"
"#;
        let list = parse(input).unwrap();
        let svg = render_svg(&list);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_no_items_error() {
        let input = r#"
title "Empty"
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_mixed_items() {
        let input = r#"
item "With Desc" : "Some description"
item "Without Desc"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert!(list.items[0].description.is_some());
        assert!(list.items[1].description.is_none());
    }
}
