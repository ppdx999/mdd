use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Item {
    text: String,
    done: bool,
    description: Vec<String>,
}

#[derive(Debug)]
struct TodoList {
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<TodoList, String> {
    let mut items: Vec<Item> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // [x] done item  OR  [ ] pending item
        // with optional description: [x] Label : "desc"
        if trimmed.starts_with("[x] ") || trimmed.starts_with("[ ] ") {
            let done = trimmed.starts_with("[x]");
            let rest = trimmed[4..].trim();

            if let Some((label, desc_part)) = rest.split_once(" : ") {
                let (desc, consumed) = parse_multiline_desc(desc_part.trim(), &lines, i)?;
                i += consumed;
                items.push(Item {
                    text: label.trim().to_string(),
                    done,
                    description: desc,
                });
            } else {
                items.push(Item {
                    text: rest.to_string(),
                    done,
                    description: Vec::new(),
                });
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if items.is_empty() {
        return Err("At least 1 item is required".to_string());
    }

    Ok(TodoList { items })
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
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;

const ITEM_HEIGHT: f64 = 36.0;
const ITEM_HEIGHT_WITH_DESC: f64 = 36.0;
const ITEM_GAP: f64 = 4.0;
const ITEM_H_PAD: f64 = 16.0;
const CHECKBOX_SIZE: f64 = 16.0;
const CHECKBOX_LEFT: f64 = 14.0;
const TEXT_LEFT: f64 = 40.0;
const MIN_WIDTH: f64 = 250.0;
const PADDING: f64 = 24.0;
const DESC_LINE_HEIGHT: f64 = 15.0;

const COLOR_DARK: &str = "#333";
const COLOR_DESC: &str = "#888";
const COLOR_DONE_TEXT: &str = "#999";
const COLOR_CHECK: &str = "#2e7d32";
const COLOR_CHECK_BG: &str = "#e8f5e9";
const COLOR_UNCHECKED_BG: &str = "#fff";
const COLOR_UNCHECKED_BORDER: &str = "#ccc";
const COLOR_DONE_BG: &str = "#fafafa";
const COLOR_PENDING_BG: &str = "#fff";
const COLOR_BORDER: &str = "#e8e8e8";
const COLOR_STRIKETHROUGH: &str = "#bbb";

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

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

fn item_height(item: &Item) -> f64 {
    if item.description.is_empty() {
        ITEM_HEIGHT
    } else {
        ITEM_HEIGHT_WITH_DESC + item.description.len() as f64 * DESC_LINE_HEIGHT
    }
}

fn render_svg(todo: &TodoList) -> String {
    // Compute width
    let max_text_w = todo.items.iter()
        .map(|item| {
            let tw = text_width(&item.text);
            let dw = item.description.iter()
                .map(|d| text_width(d) * (DESC_FONT_SIZE / FONT_SIZE))
                .fold(0.0_f64, f64::max);
            tw.max(dw)
        })
        .fold(0.0_f64, f64::max);
    let content_w = (TEXT_LEFT + max_text_w + ITEM_H_PAD).max(MIN_WIDTH);

    let total_items_h: f64 = todo.items.iter().map(|item| item_height(item)).sum::<f64>()
        + (todo.items.len().saturating_sub(1)) as f64 * ITEM_GAP;

    let total_w = PADDING * 2.0 + content_w;
    let total_h = PADDING * 2.0 + total_items_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Render items
    let mut y = PADDING;
    for item in &todo.items {
        let ih = item_height(item);
        let ix = PADDING;

        // Row background
        let bg = if item.done { COLOR_DONE_BG } else { COLOR_PENDING_BG };
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            ix, y, content_w, ih, bg, COLOR_BORDER
        ));

        // Checkbox
        let cb_x = ix + CHECKBOX_LEFT;
        let cb_y = y + (ITEM_HEIGHT - CHECKBOX_SIZE) / 2.0;

        if item.done {
            // Filled checkbox with checkmark
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cb_x, cb_y, CHECKBOX_SIZE, CHECKBOX_SIZE, COLOR_CHECK_BG, COLOR_CHECK
            ));
            // Checkmark
            let cx = cb_x + CHECKBOX_SIZE / 2.0;
            let cy = cb_y + CHECKBOX_SIZE / 2.0;
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                cx - 4.0, cy, cx - 1.0, cy + 3.5, cx + 4.5, cy - 3.5,
                COLOR_CHECK
            ));
        } else {
            // Empty checkbox
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cb_x, cb_y, CHECKBOX_SIZE, CHECKBOX_SIZE, COLOR_UNCHECKED_BG, COLOR_UNCHECKED_BORDER
            ));
        }

        // Item text
        let text_x = ix + TEXT_LEFT;
        let text_y = y + ITEM_HEIGHT / 2.0 + FONT_SIZE * 0.35;
        let text_color = if item.done { COLOR_DONE_TEXT } else { COLOR_DARK };

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            text_x, text_y, text_color, escape_xml(&item.text)
        ));

        // Strikethrough for done items
        if item.done {
            let tw = text_width(&item.text);
            let strike_y = y + ITEM_HEIGHT / 2.0;
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                text_x, strike_y, text_x + tw, strike_y, COLOR_STRIKETHROUGH
            ));
        }

        // Description
        if !item.description.is_empty() {
            let desc_color = if item.done { COLOR_DONE_TEXT } else { COLOR_DESC };
            for (j, desc_line) in item.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    text_x,
                    y + ITEM_HEIGHT + j as f64 * DESC_LINE_HEIGHT + DESC_FONT_SIZE * 0.5,
                    DESC_FONT_SIZE,
                    desc_color,
                    escape_xml(desc_line)
                ));
            }
        }

        y += ih + ITEM_GAP;
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

    let todo = match parse(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("mdd-todo: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&todo));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "[x] Done task\n[ ] Pending task\n";
        let t = parse(input).unwrap();
        assert_eq!(t.items.len(), 2);
        assert!(t.items[0].done);
        assert_eq!(t.items[0].text, "Done task");
        assert!(!t.items[1].done);
        assert_eq!(t.items[1].text, "Pending task");
    }

    #[test]
    fn parse_with_description() {
        let input = "[ ] Task A : \"Details here\"\n[ ] Task B\n";
        let t = parse(input).unwrap();
        assert_eq!(t.items[0].description, vec!["Details here"]);
        assert!(t.items[1].description.is_empty());
    }

    #[test]
    fn parse_multiline_description() {
        let input = "[ ] Task A : \"Line one\nLine two\"\n[ ] Task B\n";
        let t = parse(input).unwrap();
        assert_eq!(t.items[0].description, vec!["Line one", "Line two"]);
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn parse_error_unknown() {
        let input = "foo bar\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "[x] Done\n[ ] Todo\n";
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Done"));
        assert!(svg.contains("Todo"));
    }

    #[test]
    fn render_contains_checkmark() {
        let input = "[x] Done\n[ ] Todo\n";
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        assert!(svg.contains("polyline")); // checkmark
    }

    #[test]
    fn render_contains_strikethrough() {
        let input = "[x] Done\n[ ] Todo\n";
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        // Done item should have strikethrough line
        assert!(svg.matches("<line").count() >= 1);
    }
}
