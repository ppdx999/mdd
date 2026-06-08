use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ListItem {
    label: String,
    description: Vec<String>,
    ordered: bool,
}

#[derive(Debug)]
struct ListV {
    items: Vec<ListItem>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ListV, String> {
    let mut items: Vec<ListItem> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // 1. "Label" { description } or 1. "Label" (ordered item, number is ignored)
        if let Some(rest) = strip_ordered_prefix(trimmed) {
            let (label, description, consumed) = parse_item(rest, &lines, i)?;
            items.push(ListItem { label, description, ordered: true });
            i += 1 + consumed;
            continue;
        }

        // - "Label" { description } or - "Label"
        if trimmed.starts_with("- ") {
            let rest = trimmed.strip_prefix("- ").unwrap().trim();
            let (label, description, consumed) = parse_item(rest, &lines, i)?;
            items.push(ListItem { label, description, ordered: false });
            i += 1 + consumed;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if items.is_empty() {
        return Err("At least 1 item is required".to_string());
    }

    Ok(ListV { items })
}

fn strip_ordered_prefix(s: &str) -> Option<&str> {
    let mut chars = s.chars();
    // Must start with at least one digit
    if !chars.next().map_or(false, |c| c.is_ascii_digit()) {
        return None;
    }
    // Skip remaining digits
    let rest = &s[1..];
    let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    // Must be followed by ". "
    after_digits.strip_prefix(". ").map(|r| r.trim())
}

fn parse_item(s: &str, lines: &[&str], current: usize) -> Result<(String, Vec<String>, usize), String> {
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
        return Ok((label, Vec::new(), 0));
    }

    if rest.starts_with('{') {
        let after_brace = rest[1..].trim();
        // Single-line: "Label" { desc }
        if let Some(end) = after_brace.strip_suffix('}') {
            let desc = end.trim().to_string();
            if desc.is_empty() {
                return Ok((label, Vec::new(), 0));
            }
            return Ok((label, vec![desc], 0));
        }
        // Multi-line block
        let mut desc_lines = Vec::new();
        if !after_brace.is_empty() {
            desc_lines.push(after_brace.to_string());
        }
        let mut extra = 0;
        for j in (current + 1)..lines.len() {
            extra += 1;
            let bl = lines[j].trim();
            if bl == "}" {
                break;
            }
            desc_lines.push(bl.to_string());
        }
        return Ok((label, desc_lines, extra));
    }

    Err(format!("Expected '{{' after label, got: {}", rest))
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const TITLE_FONT_SIZE: f64 = 15.0;
const DESC_FONT_SIZE: f64 = 11.0;
const COLOR_DARK: &str = "#333";

const BADGE_RADIUS: f64 = 16.0;
const ITEM_H_PAD: f64 = 16.0;
const ITEM_MIN_HEIGHT: f64 = 48.0;
const ITEM_GAP: f64 = 8.0;
const PADDING: f64 = 40.0;
const DESC_LINE_HEIGHT: f64 = 15.0;
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
    // Compute item heights and content width
    let badge_area = BADGE_RADIUS * 2.0 + ITEM_H_PAD;
    let mut max_content_w: f64 = 0.0;

    for item in &list.items {
        let label_w = text_width(&item.label);
        let desc_w = item
            .description
            .iter()
            .map(|d| text_width(d))
            .fold(0.0_f64, f64::max);
        let w = label_w.max(desc_w);
        if w > max_content_w {
            max_content_w = w;
        }
    }

    let total_w = PADDING * 2.0 + badge_area + max_content_w + ITEM_H_PAD;

    // Calculate item heights
    let item_heights: Vec<f64> = list
        .items
        .iter()
        .map(|item| {
            if item.description.is_empty() {
                ITEM_MIN_HEIGHT
            } else {
                ITEM_MIN_HEIGHT + item.description.len() as f64 * DESC_LINE_HEIGHT
            }
        })
        .collect();

    let separators_h = if list.items.len() > 1 {
        (list.items.len() - 1) as f64 * ITEM_GAP
    } else {
        0.0
    };

    let items_total_h: f64 = item_heights.iter().sum::<f64>() + separators_h;
    let total_h = PADDING * 2.0 + items_total_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let mut y = PADDING;

    // Items
    let mut ordered_index = 0usize;
    for (i, item) in list.items.iter().enumerate() {
        let (bg_color, fg_color) = COLORS[i % COLORS.len()];
        let item_h = item_heights[i];

        let badge_cx = PADDING + BADGE_RADIUS;
        let badge_cy = y + item_h / 2.0;

        if item.ordered {
            ordered_index += 1;
            // Numbered badge circle
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" />",
                badge_cx, badge_cy, BADGE_RADIUS, bg_color
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                badge_cx,
                badge_cy + 5.0,
                FONT_SIZE,
                fg_color,
                ordered_index
            ));
        } else {
            // Bullet dot
            let bullet_radius = 6.0;
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" />",
                badge_cx, badge_cy, bullet_radius, fg_color
            ));
        }

        // Label text (bold)
        let text_x = PADDING + badge_area;
        let label_y = if !item.description.is_empty() {
            y + ITEM_MIN_HEIGHT / 2.0 - 2.0
        } else {
            y + item_h / 2.0 + 5.0
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" stroke=\"{}\" stroke-width=\"0.5\">{}</text>",
            text_x,
            label_y,
            TITLE_FONT_SIZE,
            COLOR_DARK,
            escape_xml(&item.label)
        ));

        // Description text
        if !item.description.is_empty() {
            let desc_base_y = label_y + DESC_FONT_SIZE + 6.0;
            for (j, desc_line) in item.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#666\">{}</text>",
                    text_x,
                    desc_base_y + j as f64 * DESC_LINE_HEIGHT,
                    DESC_FONT_SIZE,
                    escape_xml(desc_line)
                ));
            }
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

const HELP: &str = "\
mdd-list-v - Render a vertical list as SVG

Usage: mdd-list-v < input.list-v

Ordered items:   N. \"<label>\" [{ <description> }]
Unordered items: - \"<label>\" [{ <description> }]
Ordered and unordered items can be mixed.

Multi-line descriptions use a block:
  1. \"Label\" {
    line1
    line2
  }

Example:
  1. \"Create account\" { Register with email }
  2. \"Initial setup\" { Configure profile }
  3. \"Invite team\" { Add members }
  4. \"Go live\"
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
1. "First"
2. "Second"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "First");
        assert!(list.items[0].description.is_empty());
        assert_eq!(list.items[1].label, "Second");
    }

    #[test]
    fn parse_with_desc() {
        let input = r#"
1. "Label" { Description }
2. "Other" { Details }
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "Label");
        assert_eq!(list.items[0].description, vec!["Description"]);
        assert_eq!(list.items[1].label, "Other");
        assert_eq!(list.items[1].description, vec!["Details"]);
    }

    #[test]
    fn parse_multiline_desc() {
        let input = "1. \"Label\" {\n  line one\n  line two\n}\n";
        let list = parse(input).unwrap();
        assert_eq!(list.items[0].label, "Label");
        assert_eq!(list.items[0].description, vec!["line one", "line two"]);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
1. "Test"
"#;
        let list = parse(input).unwrap();
        let svg = render_svg(&list);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_no_items_error() {
        let input = "";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_mixed_items() {
        let input = r#"
1. "With Desc" { Some description }
2. "Without Desc"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert!(!list.items[0].description.is_empty());
        assert!(list.items[1].description.is_empty());
    }

    #[test]
    fn parse_unordered_basic() {
        let input = r#"
- "Alpha"
- "Beta"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "Alpha");
        assert!(!list.items[0].ordered);
        assert_eq!(list.items[1].label, "Beta");
        assert!(!list.items[1].ordered);
    }

    #[test]
    fn parse_unordered_with_desc() {
        let input = r#"
- "Feature" { Some detail }
- "Other"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].description, vec!["Some detail"]);
        assert!(list.items[1].description.is_empty());
    }

    #[test]
    fn parse_mixed_ordered_unordered() {
        let input = r#"
1. "Step 1"
- "Note"
2. "Step 2"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.items.len(), 3);
        assert!(list.items[0].ordered);
        assert!(!list.items[1].ordered);
        assert!(list.items[2].ordered);
    }

    #[test]
    fn render_unordered_no_number() {
        let input = r#"
- "Bullet"
"#;
        let list = parse(input).unwrap();
        let svg = render_svg(&list);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Bullet"));
        // Should not contain a badge number "1"
        assert!(!svg.contains(">1</text>"));
    }
}
