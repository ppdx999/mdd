use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Group {
    name: String,
    items: Vec<String>,
}

#[derive(Debug)]
struct Groups {
    title: Option<String>,
    groups: Vec<Group>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Groups, String> {
    let mut title: Option<String> = None;
    let mut groups: Vec<Group> = Vec::new();
    let mut current_group: Option<String> = None;
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

        // group "name" {
        if trimmed.starts_with("group ") {
            if current_group.is_some() {
                return Err("Nested groups are not allowed".to_string());
            }
            let rest = trimmed.strip_prefix("group ").unwrap().trim();
            let name = parse_group_header(rest)?;
            current_group = Some(name);
            current_items = Vec::new();
            continue;
        }

        // closing brace
        if trimmed == "}" {
            match current_group.take() {
                Some(name) => {
                    if current_items.is_empty() {
                        return Err(format!("Group '{}' has no items", name));
                    }
                    groups.push(Group {
                        name,
                        items: std::mem::take(&mut current_items),
                    });
                }
                None => return Err("Unexpected '}'".to_string()),
            }
            continue;
        }

        // inside a group — each line is an item
        if current_group.is_some() {
            current_items.push(trimmed.to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_group.is_some() {
        return Err("Unclosed group (missing '}')".to_string());
    }

    if groups.is_empty() {
        return Err("At least 1 group is required".to_string());
    }

    if groups.len() > 4 {
        return Err(format!(
            "Too many groups ({}). Maximum is 4",
            groups.len()
        ));
    }

    Ok(Groups { title, groups })
}

fn parse_group_header(rest: &str) -> Result<String, String> {
    let rest = rest.trim();
    if rest.starts_with('"') {
        let end_quote = rest[1..]
            .find('"')
            .ok_or("Unterminated quote in group header")?;
        let name = rest[1..=end_quote].to_string();
        let after_quote = rest[end_quote + 2..].trim();
        if after_quote == "{" || after_quote.is_empty() {
            return Ok(name);
        }
        return Err(format!(
            "Expected '{{' after group name, got: {}",
            after_quote
        ));
    }
    // unquoted: name {
    if rest.ends_with('{') {
        let name = rest.trim_end_matches('{').trim();
        if name.is_empty() {
            return Err("Group name is required".to_string());
        }
        return Ok(name.to_string());
    }
    Err(format!("Expected '{{' in group header: {}", rest))
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

// Colors: (desc_bg, title_bg, accent)
const COLORS: &[(&str, &str, &str)] = &[
    ("#f0f7ff", "#90caf9", "#1565c0"),
    ("#f0f9f1", "#a5d6a7", "#2e7d32"),
    ("#fffef2", "#ffe082", "#f57f17"),
    ("#faf2fc", "#ce93d8", "#7b1fa2"),
];

const GROUP_MIN_WIDTH: f64 = 160.0;
const GROUP_H_PAD: f64 = 16.0;
const HEADER_HEIGHT: f64 = 36.0;
const ITEM_HEIGHT: f64 = 26.0;
const GROUP_GAP: f64 = 16.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const ITEM_INDENT: f64 = 24.0;
const BULLET_RADIUS: f64 = 3.0;

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                CHAR_WIDTH
            } else {
                CJK_CHAR_WIDTH
            }
        })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(groups: &Groups) -> String {
    // Compute each group's width based on content
    let group_widths: Vec<f64> = groups
        .groups
        .iter()
        .map(|g| {
            let header_w = text_width(&g.name) + GROUP_H_PAD * 2.0;
            let max_item_w = g
                .items
                .iter()
                .map(|item| ITEM_INDENT + text_width(item) + GROUP_H_PAD)
                .fold(0.0_f64, f64::max);
            header_w.max(max_item_w).max(GROUP_MIN_WIDTH)
        })
        .collect();

    // Find the tallest group (most items)
    let max_items = groups
        .groups
        .iter()
        .map(|g| g.items.len())
        .max()
        .unwrap_or(0);
    let group_body_h = max_items as f64 * ITEM_HEIGHT + GROUP_H_PAD;
    let group_h = HEADER_HEIGHT + group_body_h;

    let title_space = if groups.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let total_groups_w: f64 = group_widths.iter().sum::<f64>()
        + (groups.groups.len().saturating_sub(1)) as f64 * GROUP_GAP;
    let total_w = PADDING * 2.0 + total_groups_w;
    let total_h = PADDING * 2.0 + title_space + group_h;

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
    let content_y = if let Some(ref title) = groups.title {
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

    // Render each group
    let mut x = PADDING;
    for (i, group) in groups.groups.iter().enumerate() {
        let w = group_widths[i];
        let (bg_color, title_color, accent_color) = COLORS[i % COLORS.len()];

        // Outer rounded rect
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            x, content_y, w, group_h, bg_color, title_color
        ));

        // Header bar (top rounded corners)
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
            x, content_y, w, HEADER_HEIGHT, title_color
        ));
        // Fill bottom corners of header
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            x,
            content_y + HEADER_HEIGHT / 2.0,
            w,
            HEADER_HEIGHT / 2.0,
            title_color
        ));

        // Header text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" font-weight=\"bold\">{}</text>",
            x + w / 2.0,
            content_y + HEADER_HEIGHT / 2.0 + 5.0,
            accent_color,
            escape_xml(&group.name)
        ));

        // Items with bullet points
        for (j, item) in group.items.iter().enumerate() {
            let item_y =
                content_y + HEADER_HEIGHT + GROUP_H_PAD / 2.0 + j as f64 * ITEM_HEIGHT;
            let text_y = item_y + ITEM_HEIGHT / 2.0 + 4.0;
            let bullet_x = x + ITEM_INDENT / 2.0 + 4.0;
            let bullet_y = item_y + ITEM_HEIGHT / 2.0;

            // Bullet
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                bullet_x, bullet_y, BULLET_RADIUS, accent_color
            ));

            // Item text
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>",
                x + ITEM_INDENT,
                text_y,
                escape_xml(item)
            ));
        }

        x += w + GROUP_GAP;
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

    let groups = match parse(&input) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("mdd-group: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&groups));
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
group "Dev" {
  Alice
  Bob
}
"#;
        let g = parse(input).unwrap();
        assert!(g.title.is_none());
        assert_eq!(g.groups.len(), 1);
        assert_eq!(g.groups[0].name, "Dev");
        assert_eq!(g.groups[0].items.len(), 2);
        assert_eq!(g.groups[0].items[0], "Alice");
        assert_eq!(g.groups[0].items[1], "Bob");
    }

    #[test]
    fn parse_multiple_groups() {
        let input = r#"
title "Teams"
group "A" {
  X
}
group "B" {
  Y
  Z
}
"#;
        let g = parse(input).unwrap();
        assert_eq!(g.title.as_deref(), Some("Teams"));
        assert_eq!(g.groups.len(), 2);
        assert_eq!(g.groups[0].name, "A");
        assert_eq!(g.groups[1].name, "B");
        assert_eq!(g.groups[1].items.len(), 2);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
group "Test" {
  Item1
}
"#;
        let g = parse(input).unwrap();
        let svg = render_svg(&g);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_error_no_groups() {
        let input = r#"
title "Empty"
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_too_many_groups() {
        let input = r#"
group "A" { x }
group "B" { x }
group "C" { x }
group "D" { x }
group "E" { x }
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_empty_group() {
        let input = r#"
group "A" {
}
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_unclosed_group() {
        let input = r#"
group "A" {
  X
"#;
        assert!(parse(input).is_err());
    }
}
