use std::collections::HashMap;
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
struct Diagram {
    groups: Vec<Group>,
    colors: HashMap<String, String>,
    columns: usize,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut groups: Vec<Group> = Vec::new();
    let mut colors: HashMap<String, String> = HashMap::new();
    let mut columns: usize = 3;
    let mut current_group: Option<Group> = None;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "}" {
            if let Some(g) = current_group.take() {
                groups.push(g);
            } else {
                return Err("Unexpected }".to_string());
            }
            continue;
        }

        if trimmed.starts_with("columns ") {
            let rest = trimmed.strip_prefix("columns ").unwrap().trim();
            columns = rest
                .parse::<usize>()
                .map_err(|_| format!("Invalid columns value: {}", rest))?;
            if columns == 0 {
                return Err("columns must be at least 1".to_string());
            }
            continue;
        }

        if trimmed.starts_with("color ") {
            let rest = trimmed.strip_prefix("color ").unwrap();
            if let Some((name, color_val)) = rest.split_once(" : ") {
                colors.insert(
                    name.trim().trim_matches('"').to_string(),
                    resolve_color(color_val.trim()),
                );
                continue;
            }
            return Err(format!("Invalid color syntax: {}", trimmed));
        }

        if trimmed.starts_with("group ") {
            if current_group.is_some() {
                return Err("Nested groups are not supported".to_string());
            }
            let rest = trimmed.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                current_group = Some(Group {
                    name,
                    items: Vec::new(),
                });
                continue;
            }
            return Err(format!("Invalid group syntax: {}", trimmed));
        }

        if trimmed.starts_with("- ") {
            if let Some(ref mut g) = current_group {
                let item = trimmed.strip_prefix("- ").unwrap().trim().to_string();
                g.items.push(item);
                continue;
            }
            return Err(format!("Item outside of group: {}", trimmed));
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_group.is_some() {
        return Err("Unclosed group block".to_string());
    }

    if groups.is_empty() {
        return Err("No groups defined".to_string());
    }

    Ok(Diagram {
        groups,
        colors,
        columns,
    })
}

// ---------------------------------------------------------------------------
// Named colors
// ---------------------------------------------------------------------------

fn resolve_color(name: &str) -> String {
    match name.trim() {
        "blue" => "#e3f2fd".to_string(),
        "green" => "#e8f5e9".to_string(),
        "red" => "#ffebee".to_string(),
        "amber" | "yellow" => "#fff8e1".to_string(),
        "orange" => "#fff3e0".to_string(),
        "teal" => "#e0f2f1".to_string(),
        "purple" => "#f3e5f5".to_string(),
        "pink" => "#fce4ec".to_string(),
        "grey" | "gray" => "#f5f5f5".to_string(),
        "indigo" => "#e8eaf6".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Default palette (cycles for groups without explicit color)
// ---------------------------------------------------------------------------

const PALETTE: &[&str] = &[
    "#e3f2fd", // blue
    "#e8f5e9", // green
    "#fff8e1", // amber
    "#f3e5f5", // purple
    "#e0f2f1", // teal
    "#fff3e0", // orange
    "#fce4ec", // pink
    "#e8eaf6", // indigo
];

fn group_color<'a>(group: &Group, index: usize, colors: &'a HashMap<String, String>) -> &'a str {
    if let Some(c) = colors.get(&group.name) {
        return c.as_str();
    }
    PALETTE[index % PALETTE.len()]
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const PADDING: f64 = 24.0;
const GROUP_GAP: f64 = 16.0;
const HEADER_H: f64 = 32.0;
const ITEM_H: f64 = 24.0;
const ITEM_PAD: f64 = 12.0;
const GROUP_H_PAD: f64 = 14.0;
const GROUP_V_PAD: f64 = 10.0;
const MIN_GROUP_W: f64 = 120.0;

const COLOR_DARK: &str = "#333";
const COLOR_BORDER: &str = "#ccc";
const COLOR_HEADER_TEXT: &str = "#333";

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let cols = diagram.columns.min(diagram.groups.len());

    // Compute each group's natural size
    let group_sizes: Vec<(f64, f64)> = diagram
        .groups
        .iter()
        .map(|g| {
            let header_w = text_width(&g.name) + GROUP_H_PAD * 2.0;
            let max_item_w = g
                .items
                .iter()
                .map(|item| text_width(item) + ITEM_PAD * 2.0 + GROUP_H_PAD * 2.0)
                .fold(0.0_f64, f64::max);
            let w = header_w.max(max_item_w).max(MIN_GROUP_W);
            let h = HEADER_H + g.items.len() as f64 * ITEM_H + GROUP_V_PAD;
            (w, h)
        })
        .collect();

    // Compute column widths and row heights
    let rows = (diagram.groups.len() + cols - 1) / cols;
    let mut col_widths = vec![0.0_f64; cols];
    let mut row_heights = vec![0.0_f64; rows];

    for (i, (w, h)) in group_sizes.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        col_widths[col] = col_widths[col].max(*w);
        row_heights[row] = row_heights[row].max(*h);
    }

    let total_w = PADDING * 2.0 + col_widths.iter().sum::<f64>() + GROUP_GAP * (cols - 1) as f64;
    let total_h =
        PADDING * 2.0 + row_heights.iter().sum::<f64>() + GROUP_GAP * (rows - 1) as f64;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Render each group
    for (i, group) in diagram.groups.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;

        let x = PADDING
            + col_widths[..col].iter().sum::<f64>()
            + col as f64 * GROUP_GAP;
        let y = PADDING
            + row_heights[..row].iter().sum::<f64>()
            + row as f64 * GROUP_GAP;
        let w = col_widths[col];
        let h = row_heights[row];

        let bg = group_color(group, i, &diagram.colors);

        // Group card background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"white\" stroke=\"{}\" stroke-width=\"1\"/>",
            x, y, w, h, COLOR_BORDER
        ));

        // Header background
        svg.push_str(&format!(
            "<clipPath id=\"clip-header-{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\"/></clipPath>",
            i, x, y, w, HEADER_H
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" clip-path=\"url(#clip-header-{})\"/>",
            x, y, w, HEADER_H, bg, i
        ));
        // Bottom edge of header (straight line where it meets items)
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            x,
            y + HEADER_H,
            x + w,
            y + HEADER_H,
            COLOR_BORDER
        ));

        // Header text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            x + GROUP_H_PAD,
            y + HEADER_H / 2.0 + FONT_SIZE * 0.35,
            COLOR_HEADER_TEXT,
            escape_xml(&group.name)
        ));

        // Items
        for (j, item) in group.items.iter().enumerate() {
            let iy = y + HEADER_H + j as f64 * ITEM_H + ITEM_H / 2.0 + FONT_SIZE * 0.35;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>",
                x + GROUP_H_PAD + ITEM_PAD,
                iy,
                escape_xml(item)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
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
mdd-group-multi - Render multiple groups in a grid layout as SVG

Usage: mdd-group-multi < input.group-multi

Define groups with \"group \"Name\" { ... }\" containing \"- item\" lines.
Set column count with \"columns N\" (default: 3). Optionally set
group colors with \"color \"Name\" : color\".

Example:
  group \"Frontend\" {
  - React
  - TypeScript
  }
  group \"Backend\" {
  - Rust
  - PostgreSQL
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
            eprintln!("mdd-group-multi: {}", e);
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
    fn parse_simple() {
        let input = "group \"A\" {\n- item1\n- item2\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].name, "A");
        assert_eq!(d.groups[0].items, vec!["item1", "item2"]);
    }

    #[test]
    fn parse_multiple_groups() {
        let input = "group \"A\" {\n- x\n}\ngroup \"B\" {\n- y\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 2);
    }

    #[test]
    fn parse_columns() {
        let input = "columns 4\ngroup \"A\" {\n- x\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.columns, 4);
    }

    #[test]
    fn parse_color() {
        let input = "color \"A\" : blue\ngroup \"A\" {\n- x\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.colors.get("A").unwrap(), "#e3f2fd");
    }

    #[test]
    fn parse_error_no_groups() {
        let result = parse("columns 3\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_unclosed() {
        let result = parse("group \"A\" {\n- x\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_nested() {
        let result = parse("group \"A\" {\ngroup \"B\" {\n}\n}\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_error_item_outside() {
        let result = parse("- orphan\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_japanese() {
        let input = "group \"開発チーム\" {\n- フロントエンド\n- バックエンド\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups[0].name, "開発チーム");
        assert_eq!(d.groups[0].items[0], "フロントエンド");
    }

    #[test]
    fn render_produces_svg() {
        let input = "group \"A\" {\n- x\n- y\n}\ngroup \"B\" {\n- z\n}\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_group_and_items() {
        let input = "group \"Team\" {\n- Alice\n- Bob\n}\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Team"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Bob"));
    }

    #[test]
    fn default_columns_is_three() {
        let input = "group \"A\" {\n- x\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.columns, 3);
    }

    #[test]
    fn color_hex_passthrough() {
        let input = "color \"X\" : #abcdef\ngroup \"X\" {\n- a\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.colors.get("X").unwrap(), "#abcdef");
    }
}
