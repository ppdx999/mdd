use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Entry {
    label: String,
    depth: usize,
    description: String,
}

#[derive(Debug)]
struct Outline {
    entries: Vec<Entry>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Outline, String> {
    let mut entries: Vec<Entry> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        let trimmed = line.trim();

        let (label, description) = if let Some((l, d)) = trimmed.split_once(" : ") {
            (l.trim().to_string(), strip_quotes(d.trim()).to_string())
        } else {
            (trimmed.to_string(), String::new())
        };

        entries.push(Entry { label, depth, description });
    }

    if entries.is_empty() {
        return Err("At least 1 entry required".to_string());
    }

    Ok(Outline { entries })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Tree structure helpers
// ---------------------------------------------------------------------------

fn compute_is_last(entries: &[Entry]) -> Vec<bool> {
    let n = entries.len();
    let mut is_last = vec![false; n];

    for i in 0..n {
        let depth = entries[i].depth;
        let mut found_sibling = false;
        for j in (i + 1)..n {
            if entries[j].depth == depth {
                found_sibling = true;
                break;
            }
            if entries[j].depth < depth {
                break;
            }
        }
        is_last[i] = !found_sibling;
    }

    is_last
}

fn ancestor_continues(entries: &[Entry], idx: usize, is_last: &[bool]) -> Vec<bool> {
    let depth = entries[idx].depth;
    let mut continues = vec![false; depth];

    for d in 0..depth {
        for j in (0..idx).rev() {
            if entries[j].depth == d {
                continues[d] = !is_last[j];
                break;
            }
            if entries[j].depth < d {
                break;
            }
        }
    }

    continues
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;
const LINE_H: f64 = 26.0;
const INDENT: f64 = 24.0;
const TREE_MID: f64 = 12.0;
const PAD: f64 = 24.0;
const DESC_GAP: f64 = 20.0;

const COLOR_TEXT: &str = "#333";
const COLOR_TREE: &str = "#ccc";
const COLOR_DESC: &str = "#888";

// Depth-based label colors
const DEPTH_COLORS: &[&str] = &[
    "#1565c0", // depth 0: blue
    "#333",    // depth 1: dark
    "#555",    // depth 2: medium
    "#777",    // depth 3+: light
];

// ---------------------------------------------------------------------------
// Sizing & helpers
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn depth_color(depth: usize) -> &'static str {
    DEPTH_COLORS[depth.min(DEPTH_COLORS.len() - 1)]
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(outline: &Outline) -> String {
    let is_last = compute_is_last(&outline.entries);

    // Compute tree area width
    let max_tree_w = outline.entries.iter().map(|e| {
        e.depth as f64 * INDENT + TREE_MID + 8.0 + text_width(&e.label)
    }).fold(0.0_f64, f64::max);

    // Compute description area width
    let has_desc = outline.entries.iter().any(|e| !e.description.is_empty());
    let max_desc_w = if has_desc {
        outline.entries.iter()
            .filter(|e| !e.description.is_empty())
            .map(|e| text_width(&e.description) * (DESC_FONT_SIZE / FONT_SIZE))
            .fold(0.0_f64, f64::max)
    } else {
        0.0
    };
    let desc_area_w = if has_desc { DESC_GAP + max_desc_w + 16.0 } else { 0.0 };

    let total_w = PAD * 2.0 + max_tree_w + desc_area_w;
    let total_h = PAD * 2.0 + outline.entries.len() as f64 * LINE_H;
    let desc_x = PAD + max_tree_w + DESC_GAP;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_TEXT
    ));

    for (idx, entry) in outline.entries.iter().enumerate() {
        let y = PAD + idx as f64 * LINE_H;
        let cy = y + LINE_H / 2.0;

        // Tree connector lines
        if entry.depth > 0 {
            let continues = ancestor_continues(&outline.entries, idx, &is_last);

            // Vertical continuation lines from ancestors
            for d in 0..entry.depth {
                if continues[d] {
                    let ax = PAD + d as f64 * INDENT + TREE_MID;
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        ax, y, ax, y + LINE_H, COLOR_TREE
                    ));
                }
            }

            // Branch connector
            let bx = PAD + (entry.depth - 1) as f64 * INDENT + TREE_MID;
            let ex = PAD + entry.depth as f64 * INDENT;

            if is_last[idx] {
                // └──
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    bx, y, bx, cy, COLOR_TREE
                ));
            } else {
                // ├──
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    bx, y, bx, y + LINE_H, COLOR_TREE
                ));
            }
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                bx, cy, ex, cy, COLOR_TREE
            ));
        }

        // Label
        let label_x = PAD + entry.depth as f64 * INDENT + if entry.depth > 0 { 4.0 } else { 0.0 };
        let font_weight = if entry.depth == 0 { " font-weight=\"bold\"" } else { "" };
        let color = depth_color(entry.depth);
        let font_size = if entry.depth == 0 { 14.0 } else { FONT_SIZE };

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\"{}>{}</text>",
            label_x, cy + font_size * 0.35, font_size, color, font_weight, escape_xml(&entry.label)
        ));

        // Description (right side, with connecting dotted line)
        if !entry.description.is_empty() {
            let label_end = label_x + text_width(&entry.label);
            // Dotted line from label to description
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"2,2\"/>",
                label_end + 6.0, cy, desc_x - 6.0, cy, COLOR_TREE
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                desc_x, cy + DESC_FONT_SIZE * 0.35, DESC_FONT_SIZE, COLOR_DESC, escape_xml(&entry.description)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-outline - Render a hierarchical outline as SVG

Usage: mdd-outline < input.txt

Indentation-based hierarchy (2 spaces per level).
Optional description with \" : \".

Example:
  Dashboard
    Sales Summary
      GET /api/sales/summary : \"daily totals\"
    Notifications
      GET /api/notifications
  User Management
    User Search
      GET /api/users
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

    let outline = match parse(&input) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("mdd-outline: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&outline));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "A\n  B\n  C\n";
        let b = parse(input).unwrap();
        assert_eq!(b.entries.len(), 3);
        assert_eq!(b.entries[0].depth, 0);
        assert_eq!(b.entries[1].depth, 1);
    }

    #[test]
    fn parse_deep() {
        let input = "A\n  B\n    C\n      D\n";
        let b = parse(input).unwrap();
        assert_eq!(b.entries[3].depth, 3);
        assert_eq!(b.entries[3].label, "D");
    }

    #[test]
    fn parse_with_description() {
        let input = "A : \"top level\"\n  B : \"child\"\n";
        let b = parse(input).unwrap();
        assert_eq!(b.entries[0].description, "top level");
        assert_eq!(b.entries[1].description, "child");
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "A\n  B\n  C\n";
        let b = parse(input).unwrap();
        let svg = render_svg(&b);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(">A<"));
        assert!(svg.contains(">B<"));
    }

    #[test]
    fn multiple_roots() {
        let input = "A\n  A1\nB\n  B1\n";
        let b = parse(input).unwrap();
        assert_eq!(b.entries.len(), 4);
        assert_eq!(b.entries[2].depth, 0);
    }
}
