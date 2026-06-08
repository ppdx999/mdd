use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Entry {
    name: String,
    is_dir: bool,
    depth: usize,
    description: Vec<String>,
}

#[derive(Debug)]
struct DirTree {
    entries: Vec<Entry>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<DirTree, String> {
    let mut entries: Vec<Entry> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Count leading spaces for depth (2 spaces = 1 level)
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        let trimmed = line.trim();

        // Directory: ends with /
        // File: everything else
        // Optional description: name : "desc"
        let (name_part, desc_start) = if let Some(colon_pos) = trimmed.find(" : ") {
            (&trimmed[..colon_pos], Some(trimmed[colon_pos + 3..].trim()))
        } else {
            (trimmed, None)
        };

        let is_dir = name_part.ends_with('/');
        let name = if is_dir {
            name_part.trim_end_matches('/').to_string()
        } else {
            name_part.to_string()
        };

        let (description, consumed) = if let Some(ds) = desc_start {
            parse_multiline_desc(ds, &lines, i)?
        } else {
            (Vec::new(), 0)
        };
        i += consumed;

        entries.push(Entry {
            name,
            is_dir,
            depth,
            description,
        });

        i += 1;
    }

    if entries.is_empty() {
        return Err("At least 1 entry is required".to_string());
    }

    Ok(DirTree { entries })
}

fn parse_multiline_desc(
    start: &str,
    lines: &[&str],
    current: usize,
) -> Result<(Vec<String>, usize), String> {
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
// Tree structure helpers
// ---------------------------------------------------------------------------

/// For each entry, determine if it's the last sibling at its depth.
fn compute_is_last(entries: &[Entry]) -> Vec<bool> {
    let n = entries.len();
    let mut is_last = vec![false; n];

    for i in 0..n {
        let depth = entries[i].depth;
        // Look forward: is there another entry at the same depth
        // before a shallower entry (or end)?
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

/// For a given entry, determine which ancestor depths have continuing lines.
fn ancestor_continues(entries: &[Entry], idx: usize, is_last: &[bool]) -> Vec<bool> {
    let depth = entries[idx].depth;
    let mut continues = vec![false; depth];

    for d in 0..depth {
        // Find the nearest ancestor at this depth (scan backward)
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

const CHAR_W: f64 = 8.4;
const CJK_W: f64 = 14.0;
const MONO_CHAR_W: f64 = 8.4;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;
const LINE_HEIGHT: f64 = 24.0;
const INDENT_W: f64 = 24.0;
const TREE_PREFIX_W: f64 = 24.0;
const PADDING: f64 = 24.0;
const ICON_SIZE: f64 = 14.0;
const DESC_GAP: f64 = 30.0;
const DESC_LINE_HEIGHT: f64 = 15.0;
const DESC_LINE_COLOR: &str = "#ccc";

const COLOR_TEXT: &str = "#333";
const COLOR_DESC: &str = "#666";
const COLOR_TREE_LINE: &str = "#999";
const COLOR_DIR: &str = "#1565c0";
const COLOR_DIR_ICON: &str = "#f57f17";
const COLOR_FILE_ICON: &str = "#999";

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_W } else { CJK_W })
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

fn render_svg(tree: &DirTree) -> String {
    let is_last = compute_is_last(&tree.entries);

    // Compute tree area width
    let max_tree_w = tree
        .entries
        .iter()
        .map(|e| {
            let indent = e.depth as f64 * INDENT_W + TREE_PREFIX_W;
            indent + ICON_SIZE + 6.0 + text_width(&e.name)
        })
        .fold(0.0_f64, f64::max);

    // Compute description area width
    let has_desc = tree.entries.iter().any(|e| !e.description.is_empty());
    let max_desc_w = if has_desc {
        tree.entries
            .iter()
            .flat_map(|e| e.description.iter())
            .map(|d| text_width(d) * (DESC_FONT_SIZE / FONT_SIZE))
            .fold(0.0_f64, f64::max)
    } else {
        0.0
    };
    let desc_area_w = if has_desc {
        DESC_GAP + max_desc_w + 16.0
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + max_tree_w + desc_area_w;
    let total_h = PADDING * 2.0 + tree.entries.len() as f64 * LINE_HEIGHT;
    let desc_x = PADDING + max_tree_w + DESC_GAP;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: 'SF Mono', 'Menlo', 'Consolas', monospace; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_TEXT
    ));

    for (idx, entry) in tree.entries.iter().enumerate() {
        let y = PADDING + idx as f64 * LINE_HEIGHT;
        let cy = y + LINE_HEIGHT / 2.0;

        // Draw tree connector lines
        if entry.depth > 0 {
            let continues = ancestor_continues(&tree.entries, idx, &is_last);

            // Vertical continuation lines from ancestors
            for d in 0..entry.depth {
                if continues[d] {
                    let ax = PADDING + d as f64 * INDENT_W + TREE_PREFIX_W / 2.0;
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        ax, y, ax, y + LINE_HEIGHT, COLOR_TREE_LINE
                    ));
                }
            }

            // Branch connector for this entry
            let bx = PADDING + (entry.depth - 1) as f64 * INDENT_W + TREE_PREFIX_W / 2.0;
            let ex = PADDING + entry.depth as f64 * INDENT_W;

            if is_last[idx] {
                // └── (L-shape)
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    bx, y, bx, cy, COLOR_TREE_LINE
                ));
            } else {
                // ├── (T-shape, vertical continues)
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    bx, y, bx, y + LINE_HEIGHT, COLOR_TREE_LINE
                ));
            }
            // Horizontal line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                bx, cy, ex, cy, COLOR_TREE_LINE
            ));
        }

        // Icon + name
        let icon_x = PADDING + entry.depth as f64 * INDENT_W;
        let text_x = icon_x + ICON_SIZE + 6.0;

        if entry.is_dir {
            // Folder icon
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1.5\" fill=\"{}\"/>",
                icon_x,
                cy - ICON_SIZE / 2.0 + 2.0,
                ICON_SIZE,
                ICON_SIZE - 2.0,
                COLOR_DIR_ICON
            ));
            // Folder tab
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"6\" height=\"3\" rx=\"1\" fill=\"{}\"/>",
                icon_x,
                cy - ICON_SIZE / 2.0,
                COLOR_DIR_ICON
            ));

            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}/</text>",
                text_x,
                cy + FONT_SIZE * 0.35,
                COLOR_DIR,
                escape_xml(&entry.name)
            ));
        } else {
            // File icon
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                icon_x + 1.0,
                cy - ICON_SIZE / 2.0,
                ICON_SIZE - 2.0,
                ICON_SIZE,
                COLOR_FILE_ICON
            ));
            // Folded corner
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                icon_x + ICON_SIZE - 5.0,
                cy - ICON_SIZE / 2.0,
                icon_x + ICON_SIZE - 1.0,
                cy - ICON_SIZE / 2.0 + 4.0,
                icon_x + ICON_SIZE - 1.0,
                cy - ICON_SIZE / 2.0,
                COLOR_FILE_ICON
            ));

            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{}</text>",
                text_x,
                cy + FONT_SIZE * 0.35,
                escape_xml(&entry.name)
            ));
        }

        // Description on the right with horizontal line
        if !entry.description.is_empty() {
            let line_start_x = text_x + text_width(&entry.name) + if entry.is_dir { MONO_CHAR_W } else { 0.0 } + 8.0;

            // Horizontal line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                line_start_x, cy, desc_x - 8.0, cy, DESC_LINE_COLOR
            ));
            // Dot
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\"/>",
                line_start_x, cy, DESC_LINE_COLOR
            ));

            // Description text (multi-line, vertically centered on the line)
            let desc_start_y =
                cy - (entry.description.len() as f64 - 1.0) * DESC_LINE_HEIGHT * 0.5;
            for (j, desc_line) in entry.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    desc_x,
                    desc_start_y + j as f64 * DESC_LINE_HEIGHT + DESC_FONT_SIZE * 0.35,
                    DESC_FONT_SIZE,
                    COLOR_DESC,
                    escape_xml(desc_line)
                ));
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-dirtree - Render a directory tree as SVG

Usage: mdd-dirtree < input.dirtree

Indent with 2 spaces per level. Directories end with \"/\".
Add descriptions with \" : \\\"text\\\"\".

Example:
  src/
    main.rs : \"entry point\"
    lib.rs
  Cargo.toml
  README.md
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

    let tree = match parse(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("mdd-dirtree: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&tree));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "src/\n  main.rs\n  lib.rs\n";
        let t = parse(input).unwrap();
        assert_eq!(t.entries.len(), 3);
        assert!(t.entries[0].is_dir);
        assert_eq!(t.entries[0].name, "src");
        assert_eq!(t.entries[0].depth, 0);
        assert!(!t.entries[1].is_dir);
        assert_eq!(t.entries[1].name, "main.rs");
        assert_eq!(t.entries[1].depth, 1);
    }

    #[test]
    fn parse_with_description() {
        let input = "src/ : \"ソースコード\"\n  main.rs : \"エントリポイント\"\n";
        let t = parse(input).unwrap();
        assert_eq!(t.entries[0].description, vec!["ソースコード"]);
        assert_eq!(t.entries[1].description, vec!["エントリポイント"]);
    }

    #[test]
    fn parse_multiline_desc() {
        let input = "src/ : \"ソースコード\n全体の構成\"\n  main.rs\n";
        let t = parse(input).unwrap();
        assert_eq!(
            t.entries[0].description,
            vec!["ソースコード", "全体の構成"]
        );
    }

    #[test]
    fn parse_nested() {
        let input = "project/\n  src/\n    components/\n      App.tsx\n    index.ts\n  package.json\n";
        let t = parse(input).unwrap();
        assert_eq!(t.entries[0].depth, 0);
        assert_eq!(t.entries[1].depth, 1);
        assert_eq!(t.entries[2].depth, 2);
        assert_eq!(t.entries[3].depth, 3);
        assert_eq!(t.entries[4].depth, 2);
        assert_eq!(t.entries[5].depth, 1);
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "src/\n  main.rs\nREADME.md\n";
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("src"));
        assert!(svg.contains("main.rs"));
    }

    #[test]
    fn is_last_sibling() {
        let input = "a/\n  x\n  y\nb/\n  z\n";
        let t = parse(input).unwrap();
        let is_last = compute_is_last(&t.entries);
        assert!(!is_last[0]); // a/ has sibling b/
        assert!(!is_last[1]); // x has sibling y
        assert!(is_last[2]);  // y is last child of a/
        assert!(is_last[3]);  // b/ is last at depth 0
        assert!(is_last[4]);  // z is last child of b/
    }
}
