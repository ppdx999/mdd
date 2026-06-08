use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Layer {
    name: String,
    description: Vec<String>,
    color: String,
}

#[derive(Debug)]
enum Item {
    Layer(Layer),
    Group {
        name: String,
        items: Vec<Item>,
    },
}

#[derive(Debug)]
struct Diagram {
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let lines: Vec<&str> = input.lines().collect();
    let (items, _) = parse_items(&lines, 0, false)?;
    if items.is_empty() {
        return Err("No layers defined".to_string());
    }
    Ok(Diagram { items })
}

fn parse_items(lines: &[&str], start: usize, in_group: bool) -> Result<(Vec<Item>, usize), String> {
    let mut items = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        if line == "}" {
            if in_group {
                return Ok((items, i + 1));
            }
            return Err("Unexpected '}'".to_string());
        }

        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap().trim();
            let name = if rest.ends_with('{') {
                let n = rest.trim_end_matches('{').trim();
                extract_quoted(n).unwrap_or(n.to_string())
            } else {
                return Err(format!("Group must end with '{{': {}", line));
            };

            let (sub_items, next) = parse_items(lines, i + 1, true)?;
            items.push(Item::Group {
                name,
                items: sub_items,
            });
            i = next;
            continue;
        }

        if line.starts_with("layer ") {
            let rest = line.strip_prefix("layer ").unwrap();
            let (layer, consumed) = parse_layer(rest, lines, i)?;
            items.push(Item::Layer(layer));
            i += 1 + consumed;
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_group {
        return Err("Unclosed group (missing '}')".to_string());
    }

    Ok((items, i))
}

fn parse_layer(rest: &str, lines: &[&str], current: usize) -> Result<(Layer, usize), String> {
    // layer Name { description } color=#xxx
    // layer Name { description }
    // layer Name color=#xxx
    // layer Name

    let (main_part, color) = if let Some(idx) = rest.find("color=") {
        let c = rest[idx + 6..].trim().to_string();
        (rest[..idx].trim(), c)
    } else {
        (rest.trim(), String::new())
    };

    // Check for { description } syntax
    if let Some(brace_pos) = main_part.find('{') {
        let name = main_part[..brace_pos].trim().to_string();
        let after_brace = main_part[brace_pos + 1..].trim();

        // Single-line: Name { desc }
        if let Some(end) = after_brace.strip_suffix('}') {
            let desc_text = end.trim();
            let description = if desc_text.is_empty() {
                Vec::new()
            } else {
                vec![desc_text.to_string()]
            };
            if name.is_empty() {
                return Err("Layer name cannot be empty".to_string());
            }
            return Ok((Layer { name, description, color }, 0));
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
        if name.is_empty() {
            return Err("Layer name cannot be empty".to_string());
        }
        return Ok((Layer { name, description: desc_lines, color }, extra));
    }

    // No description
    let name = main_part.to_string();
    if name.is_empty() {
        return Err("Layer name cannot be empty".to_string());
    }
    Ok((Layer { name, description: Vec::new(), color }, 0))
}

fn extract_quoted(s: &str) -> Option<String> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 12.0;

const LAYER_HEIGHT: f64 = 48.0;
const LAYER_H_PAD: f64 = 24.0;
const LAYER_GAP: f64 = 2.0;
const PADDING: f64 = 20.0;
const GROUP_PAD: f64 = 12.0;
const GROUP_HEADER: f64 = 28.0;
const MIN_LAYER_WIDTH: f64 = 200.0;
const DESC_GAP: f64 = 30.0;
const DESC_LINE_COLOR: &str = "#ccc";

const DEFAULT_COLORS: &[&str] = &[
    "#e3f2fd", // light blue
    "#e8f5e9", // light green
    "#fff8e1", // light yellow
    "#fce4ec", // light pink
    "#f3e5f5", // light purple
    "#e0f2f1", // light teal
    "#fff3e0", // light orange
    "#e8eaf6", // light indigo
];

const COLOR_TEXT: &str = "#333";
const COLOR_DESC: &str = "#666";
const COLOR_GROUP_BORDER: &str = "#999";
const COLOR_GROUP_BG: &str = "#fafafa";
const COLOR_GROUP_HEADER: &str = "#555";

// ---------------------------------------------------------------------------
// Layout computation
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

fn has_any_description(items: &[Item]) -> bool {
    items.iter().any(|item| match item {
        Item::Layer(l) => !l.description.is_empty(),
        Item::Group { items, .. } => has_any_description(items),
    })
}

fn max_desc_width(items: &[Item]) -> f64 {
    let mut max_w: f64 = 0.0;
    for item in items {
        match item {
            Item::Layer(l) => {
                for line in &l.description {
                    max_w = max_w.max(text_width(line));
                }
            }
            Item::Group { items, .. } => {
                max_w = max_w.max(max_desc_width(items));
            }
        }
    }
    max_w
}

fn max_layer_width(items: &[Item]) -> f64 {
    let mut max_w = MIN_LAYER_WIDTH;
    for item in items {
        match item {
            Item::Layer(l) => {
                let name_w = text_width(&l.name) + LAYER_H_PAD * 2.0;
                max_w = max_w.max(name_w);
            }
            Item::Group { name, items } => {
                let inner_w = max_layer_width(items);
                let group_w = inner_w + GROUP_PAD * 2.0;
                let header_w = text_width(name) + GROUP_PAD * 2.0;
                max_w = max_w.max(group_w).max(header_w);
            }
        }
    }
    max_w
}

fn items_height(items: &[Item]) -> f64 {
    let mut h = 0.0;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            h += LAYER_GAP;
        }
        match item {
            Item::Layer(_) => h += LAYER_HEIGHT,
            Item::Group { items: sub, .. } => {
                h += GROUP_HEADER + GROUP_PAD + items_height(sub) + GROUP_PAD;
            }
        }
    }
    h
}

fn color_for_index(idx: usize) -> &'static str {
    DEFAULT_COLORS[idx % DEFAULT_COLORS.len()]
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let has_desc = has_any_description(&diagram.items);
    let layer_w = max_layer_width(&diagram.items);
    let content_h = items_height(&diagram.items);

    let desc_area_w = if has_desc {
        DESC_GAP + max_desc_width(&diagram.items) + 16.0
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + layer_w + desc_area_w;
    let total_h = PADDING * 2.0 + content_h;

    let desc_x = PADDING + layer_w + DESC_GAP;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_TEXT
    ));

    let mut color_idx = 0;
    render_items(
        &mut svg,
        &diagram.items,
        PADDING,
        PADDING,
        layer_w,
        desc_x,
        &mut color_idx,
    );

    svg.push_str("</svg>");
    svg
}

fn render_items(
    svg: &mut String,
    items: &[Item],
    x: f64,
    start_y: f64,
    layer_w: f64,
    desc_x: f64,
    color_idx: &mut usize,
) {
    let mut y = start_y;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            y += LAYER_GAP;
        }
        match item {
            Item::Layer(layer) => {
                let bg = if layer.color.is_empty() {
                    color_for_index(*color_idx).to_string()
                } else {
                    layer.color.clone()
                };
                *color_idx += 1;

                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"#ccc\" stroke-width=\"1\"/>",
                    x, y, layer_w, LAYER_HEIGHT, bg
                ));

                // Name centered
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                    x + layer_w / 2.0,
                    y + LAYER_HEIGHT / 2.0 + 5.0,
                    escape_xml(&layer.name)
                ));

                // Description on the right side with horizontal line
                if !layer.description.is_empty() {
                    let line_y = y + LAYER_HEIGHT / 2.0;
                    let line_start_x = x + layer_w;

                    // Horizontal line
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        line_start_x, line_y, desc_x - 8.0, line_y, DESC_LINE_COLOR
                    ));
                    // Small dot at the start of line
                    svg.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"#999\"/>",
                        line_start_x, line_y
                    ));

                    // Description text (multi-line)
                    let desc_start_y = line_y - (layer.description.len() as f64 - 1.0) * DESC_FONT_SIZE * 0.7;
                    for (j, desc_line) in layer.description.iter().enumerate() {
                        svg.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                            desc_x,
                            desc_start_y + j as f64 * DESC_FONT_SIZE * 1.4 + DESC_FONT_SIZE * 0.35,
                            DESC_FONT_SIZE,
                            COLOR_DESC,
                            escape_xml(desc_line)
                        ));
                    }
                }

                y += LAYER_HEIGHT;
            }
            Item::Group { name, items: sub } => {
                let group_h = GROUP_HEADER + GROUP_PAD + items_height(sub) + GROUP_PAD;

                // Group background
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6,3\"/>",
                    x, y, layer_w, group_h, COLOR_GROUP_BG, COLOR_GROUP_BORDER
                ));

                // Group header
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"12\" fill=\"{}\">{}</text>",
                    x + GROUP_PAD,
                    y + GROUP_HEADER / 2.0 + 4.0,
                    COLOR_GROUP_HEADER,
                    escape_xml(name)
                ));

                // Render sub-items inside group
                render_items(
                    svg,
                    sub,
                    x + GROUP_PAD,
                    y + GROUP_HEADER + GROUP_PAD,
                    layer_w - GROUP_PAD * 2.0,
                    desc_x,
                    color_idx,
                );

                y += group_h;
            }
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-layer - Render a layer diagram as SVG

Usage: mdd-layer < input.layer

Each line defines a layer: layer <name> [{ description }] [color=#hex]
Multi-line descriptions use a block:
  layer Name {
    line1
    line2
  }
Layers are stacked top to bottom. Group layers with:
  group \"<name>\" { ... }
Lines starting with # are comments.

Example:
  layer Presentation
  layer Business Logic
  layer Data Access
  layer Database
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
            eprintln!("mdd-layer: {}", e);
            std::process::exit(1);
        }
    };

    let svg = render_svg(&diagram);
    print!("{}", svg);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_layer() {
        let input = "layer Presentation\n";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 1);
        match &d.items[0] {
            Item::Layer(l) => {
                assert_eq!(l.name, "Presentation");
                assert!(l.description.is_empty());
            }
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn parse_layer_with_description() {
        let input = "layer UI { Controllers, Views }\n";
        let d = parse(input).unwrap();
        match &d.items[0] {
            Item::Layer(l) => {
                assert_eq!(l.name, "UI");
                assert_eq!(l.description, vec!["Controllers, Views"]);
            }
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn parse_multiline_description() {
        let input = "layer UI {\n  Controllers\n  Views\n  Helpers\n}\n";
        let d = parse(input).unwrap();
        match &d.items[0] {
            Item::Layer(l) => {
                assert_eq!(l.description, vec!["Controllers", "Views", "Helpers"]);
            }
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn parse_layer_with_color() {
        let input = "layer DB color=#e0f7fa\n";
        let d = parse(input).unwrap();
        match &d.items[0] {
            Item::Layer(l) => {
                assert_eq!(l.name, "DB");
                assert_eq!(l.color, "#e0f7fa");
            }
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn parse_layer_with_desc_and_color() {
        let input = "layer DB { PostgreSQL } color=#e0f7fa\n";
        let d = parse(input).unwrap();
        match &d.items[0] {
            Item::Layer(l) => {
                assert_eq!(l.name, "DB");
                assert_eq!(l.description, vec!["PostgreSQL"]);
                assert_eq!(l.color, "#e0f7fa");
            }
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn parse_multiple_layers() {
        let input = "layer A\nlayer B\nlayer C\n";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 3);
    }

    #[test]
    fn parse_group() {
        let input = "group \"App\" {\n  layer A\n  layer B\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 1);
        match &d.items[0] {
            Item::Group { name, items } => {
                assert_eq!(name, "App");
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn parse_japanese_layers() {
        let input = "layer プレゼンテーション層\nlayer ビジネスロジック層\n";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 2);
        match &d.items[0] {
            Item::Layer(l) => assert_eq!(l.name, "プレゼンテーション層"),
            _ => panic!("Expected Layer"),
        }
    }

    #[test]
    fn error_on_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn error_on_unclosed_group() {
        assert!(parse("group \"X\" {\nlayer A\n").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "layer A\nlayer B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_layer_names() {
        let input = "layer Alpha\nlayer Beta\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
    }

    #[test]
    fn render_group_contains_dashed_rect() {
        let input = "group \"G\" {\n  layer X\n}\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn skip_comments_and_blank_lines() {
        let input = "# comment\n\nlayer A\n\n# another\nlayer B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 2);
    }
}
