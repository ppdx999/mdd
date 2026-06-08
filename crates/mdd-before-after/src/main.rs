use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Item {
    text: String,
}

#[derive(Debug)]
struct Section {
    label: String,
    items: Vec<Item>,
}

#[derive(Debug)]
struct Diagram {
    before: Section,
    after: Section,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut before: Option<Section> = None;
    let mut after: Option<Section> = None;

    let mut current_section: Option<String> = None; // "before" or "after"
    let mut current_label = String::new();
    let mut current_items: Vec<Item> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // before "label" { or after "label" {
        if (trimmed.starts_with("before ") || trimmed == "before{" || trimmed.starts_with("before{"))
            && current_section.is_none()
        {
            let rest = trimmed.strip_prefix("before").unwrap().trim();
            let (label, _) = parse_section_header(rest, "Before")?;
            current_section = Some("before".to_string());
            current_label = label;
            current_items = Vec::new();
            continue;
        }

        if (trimmed.starts_with("after ") || trimmed == "after{" || trimmed.starts_with("after{"))
            && current_section.is_none()
        {
            let rest = trimmed.strip_prefix("after").unwrap().trim();
            let (label, _) = parse_section_header(rest, "After")?;
            current_section = Some("after".to_string());
            current_label = label;
            current_items = Vec::new();
            continue;
        }

        // closing brace
        if trimmed == "}" {
            match current_section.as_deref() {
                Some("before") => {
                    before = Some(Section {
                        label: current_label.clone(),
                        items: std::mem::take(&mut current_items),
                    });
                    current_section = None;
                }
                Some("after") => {
                    after = Some(Section {
                        label: current_label.clone(),
                        items: std::mem::take(&mut current_items),
                    });
                    current_section = None;
                }
                _ => return Err("Unexpected '}'".to_string()),
            }
            continue;
        }

        // inside a section — each line is an item
        if current_section.is_some() {
            current_items.push(Item {
                text: trimmed.to_string(),
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_section.is_some() {
        return Err("Unclosed section (missing '}')".to_string());
    }

    let before = before.ok_or("Missing 'before' section")?;
    let after = after.ok_or("Missing 'after' section")?;

    Ok(Diagram {
        before,
        after,
    })
}

fn parse_section_header(rest: &str, default_label: &str) -> Result<(String, bool), String> {
    let rest = rest.trim();
    if rest == "{" || rest.is_empty() {
        return Ok((default_label.to_string(), rest == "{"));
    }
    // "label" { or "label"
    if rest.starts_with('"') {
        let end_quote = rest[1..]
            .find('"')
            .ok_or("Unterminated quote in section header")?;
        let label = rest[1..=end_quote].to_string();
        let after_quote = rest[end_quote + 2..].trim();
        if after_quote == "{" || after_quote.is_empty() {
            return Ok((label, after_quote == "{"));
        }
        return Err(format!("Expected '{{' after label, got: {}", after_quote));
    }
    // label {
    if rest.ends_with('{') {
        let label = rest.trim_end_matches('{').trim();
        if label.is_empty() {
            return Ok((default_label.to_string(), true));
        }
        return Ok((label.to_string(), true));
    }
    Ok((rest.to_string(), false))
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;

const PADDING: f64 = 24.0;
const ITEM_V_GAP: f64 = 8.0;
const ITEM_H_PAD: f64 = 16.0;
const ITEM_RADIUS: f64 = 8.0;

const SECTION_HEADER_HEIGHT: f64 = 36.0;
const SECTION_V_GAP: f64 = 12.0;
const ARROW_ZONE_WIDTH: f64 = 60.0;
const MIN_ITEM_WIDTH: f64 = 140.0;

// Colors
const COLOR_BEFORE_BG: &str = "#ffebee";
const COLOR_BEFORE_HEADER_BG: &str = "#ffcdd2";
const COLOR_AFTER_BG: &str = "#e8f5e9";
const COLOR_AFTER_HEADER_BG: &str = "#c8e6c9";
const COLOR_ITEM_BEFORE_BG: &str = "#fff";
const COLOR_ITEM_AFTER_BG: &str = "#fff";
const COLOR_TEXT: &str = "#333";
const COLOR_ARROW: &str = "#666";
const COLOR_BORDER_BEFORE: &str = "#ef9a9a";
const COLOR_BORDER_AFTER: &str = "#a5d6a7";

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

fn wrap_text(s: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let scale = font_size / FONT_SIZE;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0;

    for c in s.chars() {
        let cw = if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH };
        let w = cw * scale;
        if current_w + w > max_width && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
            current_w = 0.0;
        }
        current.push(c);
        current_w += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

const LINE_HEIGHT: f64 = 18.0;

/// Compute the height of a single item box given its wrapped line count.
fn item_box_height(num_lines: usize) -> f64 {
    let text_h = num_lines.max(1) as f64 * LINE_HEIGHT;
    // vertical padding inside the box
    text_h + 12.0
}

/// Pre-compute wrapped lines for each item in a section, given content width.
fn wrap_section_items(section: &Section, content_w: f64) -> Vec<Vec<String>> {
    let max_text_w = content_w - ITEM_H_PAD * 2.0;
    section
        .items
        .iter()
        .map(|item| wrap_text(&item.text, max_text_w, FONT_SIZE))
        .collect()
}

/// Compute the total body height for a section given wrapped items.
fn section_body_height(wrapped: &[Vec<String>], max_rows: usize) -> f64 {
    let mut h = SECTION_V_GAP;
    for i in 0..max_rows {
        let num_lines = if i < wrapped.len() { wrapped[i].len().max(1) } else { 1 };
        if i > 0 {
            h += ITEM_V_GAP;
        }
        h += item_box_height(num_lines);
    }
    h += SECTION_V_GAP;
    h
}

fn render_svg(diagram: &Diagram) -> String {
    let max_rows = diagram.before.items.len().max(diagram.after.items.len());

    // Compute section widths based on label width (content will wrap)
    let section_inner_pad: f64 = 12.0;

    let before_content_w = (text_width(&diagram.before.label) + ITEM_H_PAD * 2.0).max(MIN_ITEM_WIDTH);
    let after_content_w = (text_width(&diagram.after.label) + ITEM_H_PAD * 2.0).max(MIN_ITEM_WIDTH);

    let before_w = before_content_w + section_inner_pad * 2.0;
    let after_w = after_content_w + section_inner_pad * 2.0;

    // Pre-wrap items
    let before_wrapped = wrap_section_items(&diagram.before, before_content_w);
    let after_wrapped = wrap_section_items(&diagram.after, after_content_w);

    let before_body_h = section_body_height(&before_wrapped, max_rows);
    let after_body_h = section_body_height(&after_wrapped, max_rows);
    let section_body_h = before_body_h.max(after_body_h);
    let section_h = SECTION_HEADER_HEIGHT + section_body_h;

    let total_w = PADDING + before_w + ARROW_ZONE_WIDTH + after_w + PADDING;
    let total_h = PADDING + section_h + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_TEXT
    ));

    let content_y = PADDING;

    // Before section
    let before_x = PADDING;
    render_section(
        &mut svg,
        before_x,
        content_y,
        before_w,
        section_h,
        &diagram.before,
        &before_wrapped,
        COLOR_BEFORE_BG,
        COLOR_BEFORE_HEADER_BG,
        COLOR_ITEM_BEFORE_BG,
        COLOR_BORDER_BEFORE,
        before_content_w,
        section_inner_pad,
        max_rows,
    );

    // After section
    let after_x = PADDING + before_w + ARROW_ZONE_WIDTH;
    render_section(
        &mut svg,
        after_x,
        content_y,
        after_w,
        section_h,
        &diagram.after,
        &after_wrapped,
        COLOR_AFTER_BG,
        COLOR_AFTER_HEADER_BG,
        COLOR_ITEM_AFTER_BG,
        COLOR_BORDER_AFTER,
        after_content_w,
        section_inner_pad,
        max_rows,
    );

    // Arrows between corresponding items — use the larger wrapped heights
    let arrow_x1 = PADDING + before_w;
    let arrow_x2 = after_x;

    let paired = diagram.before.items.len().min(diagram.after.items.len());
    // Compute cumulative item positions (using the max of both columns)
    let mut item_mid_ys = Vec::new();
    {
        let mut y_off = content_y + SECTION_HEADER_HEIGHT + SECTION_V_GAP;
        for i in 0..max_rows {
            let bl = if i < before_wrapped.len() { before_wrapped[i].len().max(1) } else { 1 };
            let al = if i < after_wrapped.len() { after_wrapped[i].len().max(1) } else { 1 };
            let bh = item_box_height(bl.max(al));
            if i > 0 {
                y_off += ITEM_V_GAP;
            }
            item_mid_ys.push(y_off + bh / 2.0);
            y_off += bh;
        }
    }

    for i in 0..paired {
        let item_y = item_mid_ys[i];

        // Arrow line
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            arrow_x1 + 4.0, item_y, arrow_x2 - 4.0, item_y, COLOR_ARROW
        ));
        // Arrowhead
        let tip_x = arrow_x2 - 4.0;
        svg.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            tip_x, item_y,
            tip_x - 8.0, item_y - 5.0,
            tip_x - 8.0, item_y + 5.0,
            COLOR_ARROW
        ));
    }

    svg.push_str("</svg>");
    svg
}

#[allow(clippy::too_many_arguments)]
fn render_section(
    svg: &mut String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    section: &Section,
    wrapped_items: &[Vec<String>],
    bg_color: &str,
    header_bg: &str,
    item_bg: &str,
    border_color: &str,
    content_w: f64,
    inner_pad: f64,
    max_rows: usize,
) {
    // Section background with rounded corners
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, bg_color, border_color
    ));

    // Header background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"10\" fill=\"{}\"/>",
        x, y, w, SECTION_HEADER_HEIGHT, header_bg
    ));
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        x,
        y + SECTION_HEADER_HEIGHT / 2.0,
        w,
        SECTION_HEADER_HEIGHT / 2.0,
        header_bg
    ));

    // Header label
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        x + w / 2.0,
        y + SECTION_HEADER_HEIGHT / 2.0 + 5.0,
        escape_xml(&section.label)
    ));

    // Items
    let item_x = x + inner_pad;
    let mut item_y = y + SECTION_HEADER_HEIGHT + SECTION_V_GAP;
    for i in 0..max_rows {
        let num_lines = if i < wrapped_items.len() { wrapped_items[i].len().max(1) } else { 1 };
        let box_h = item_box_height(num_lines);

        if i > 0 {
            item_y += ITEM_V_GAP;
        }

        if i < section.items.len() {
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                item_x, item_y, content_w, box_h, ITEM_RADIUS, item_bg, border_color
            ));

            let lines = &wrapped_items[i];
            let total_text_h = lines.len() as f64 * LINE_HEIGHT;
            let text_start_y = item_y + (box_h - total_text_h) / 2.0 + LINE_HEIGHT - 4.0;
            for (k, line) in lines.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                    item_x + content_w / 2.0,
                    text_start_y + k as f64 * LINE_HEIGHT,
                    escape_xml(line)
                ));
            }
        } else {
            // Empty slot
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>",
                item_x, item_y, content_w, box_h, ITEM_RADIUS, border_color
            ));
        }

        item_y += box_h;
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-before-after - Render a before/after comparison as SVG

Usage: mdd-before-after < input.before-after

Define a \"before\" and an \"after\" section, each with a label
and a list of items. Arrows connect corresponding items.

Example:
  before \"Before\" {
    Manual deploy
    No tests
  }

  after \"After\" {
    Auto CI/CD
    Full test coverage
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

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-before-after: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&diagram));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal() {
        let input = r#"
before "Before" {
  A
}

after "After" {
  B
}
"#;
        let d = parse(input).unwrap();
        assert_eq!(d.before.label, "Before");
        assert_eq!(d.before.items.len(), 1);
        assert_eq!(d.before.items[0].text, "A");
        assert_eq!(d.after.label, "After");
        assert_eq!(d.after.items[0].text, "B");
    }

    #[test]
    fn parse_multiple_items() {
        let input = r#"
before "Before" {
  A
  B
  C
}

after "After" {
  X
  Y
  Z
}
"#;
        let d = parse(input).unwrap();
        assert_eq!(d.before.items.len(), 3);
        assert_eq!(d.after.items.len(), 3);
    }

    #[test]
    fn parse_japanese() {
        let input = r#"
before "現状" {
  手動デプロイ
}

after "改善後" {
  自動CI/CD
}
"#;
        let d = parse(input).unwrap();
        assert_eq!(d.before.label, "現状");
        assert_eq!(d.after.items[0].text, "自動CI/CD");
    }

    #[test]
    fn parse_uneven_items() {
        let input = r#"
before "A" {
  X
  Y
}

after "B" {
  Z
}
"#;
        let d = parse(input).unwrap();
        assert_eq!(d.before.items.len(), 2);
        assert_eq!(d.after.items.len(), 1);
    }

    #[test]
    fn error_missing_before() {
        let input = r#"
after "After" {
  X
}
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_missing_after() {
        let input = r#"
before "Before" {
  X
}
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_unclosed_section() {
        let input = r#"
before "Before" {
  X
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
before "Before" {
  A
}

after "After" {
  B
}
"#;
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white")); // background
    }

    #[test]
    fn render_contains_items() {
        let input = r#"
before "Old" {
  Hello
}

after "New" {
  World
}
"#;
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Hello"));
        assert!(svg.contains("World"));
        assert!(svg.contains("Old"));
        assert!(svg.contains("New"));
    }

    #[test]
    fn render_contains_arrow() {
        let input = r#"
before "A" {
  X
}

after "B" {
  Y
}
"#;
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("<line"));
        assert!(svg.contains("<polygon"));
    }

}
