use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SubItem {
    text: String,
}

#[derive(Debug)]
struct Branch {
    text: String,
    children: Vec<SubItem>,
}

#[derive(Debug)]
struct MindMap {
    title: Option<String>,
    center: String,
    branches: Vec<Branch>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<MindMap, String> {
    let mut title: Option<String> = None;
    let mut center: Option<String> = None;
    let mut branches: Vec<Branch> = Vec::new();

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

        // center "..."
        if trimmed.starts_with("center ") {
            let rest = trimmed.strip_prefix("center ").unwrap().trim();
            center = Some(strip_quotes(rest).to_string());
            continue;
        }

        // Determine indentation level
        let indent = line.len() - line.trim_start().len();
        let level = indent / 2;

        if level == 1 {
            // Main branch
            branches.push(Branch {
                text: trimmed.to_string(),
                children: Vec::new(),
            });
        } else if level >= 2 {
            // Sub-item of the last branch
            if let Some(branch) = branches.last_mut() {
                branch.children.push(SubItem {
                    text: trimmed.to_string(),
                });
            } else {
                return Err(format!("Sub-item without a branch: {}", trimmed));
            }
        } else {
            return Err(format!("Unknown syntax: {}", trimmed));
        }
    }

    let center = center.ok_or("Missing 'center' definition")?;

    if branches.is_empty() {
        return Err("At least 1 branch is required".to_string());
    }

    Ok(MindMap {
        title,
        center,
        branches,
    })
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

const CENTER_W: f64 = 140.0;
const CENTER_H: f64 = 48.0;
const BRANCH_H: f64 = 32.0;
const BRANCH_H_PAD: f64 = 12.0;
const MIN_BRANCH_W: f64 = 80.0;
const SUB_ITEM_H: f64 = 24.0;
const SUB_ITEM_H_PAD: f64 = 8.0;
const BRANCH_GAP_Y: f64 = 12.0;
const BRANCH_OFFSET_X: f64 = 60.0;
const PADDING: f64 = 40.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const CENTER_FONT_SIZE: f64 = 15.0;
const SUB_FONT_SIZE: f64 = 12.0;

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

/// Compute the width needed for a branch node.
fn branch_width(branch: &Branch) -> f64 {
    let tw = text_width(&branch.text) + BRANCH_H_PAD * 2.0;
    tw.max(MIN_BRANCH_W)
}

/// Compute the width needed for a sub-item node.
fn sub_item_width(item: &SubItem) -> f64 {
    let tw = text_width(&item.text) + SUB_ITEM_H_PAD * 2.0;
    tw.max(MIN_BRANCH_W)
}

/// Total height of a branch including its sub-items.
fn branch_total_height(branch: &Branch) -> f64 {
    let mut h = BRANCH_H;
    if !branch.children.is_empty() {
        h += BRANCH_GAP_Y;
        h += branch.children.len() as f64 * SUB_ITEM_H
            + (branch.children.len().saturating_sub(1)) as f64 * 4.0;
    }
    h
}

/// Max width of a branch and all its sub-items (including sub-item offset).
fn branch_max_width(branch: &Branch) -> f64 {
    let bw = branch_width(branch);
    let sw: f64 = branch
        .children
        .iter()
        .map(|c| sub_item_width(c) + 16.0)
        .fold(0.0_f64, f64::max);
    bw.max(sw)
}

fn render_svg(map: &MindMap) -> String {
    // Split branches: even-indexed (0,2,4..) go right, odd-indexed (1,3,5..) go left
    let mut right_branches: Vec<(usize, &Branch)> = Vec::new();
    let mut left_branches: Vec<(usize, &Branch)> = Vec::new();
    for (i, branch) in map.branches.iter().enumerate() {
        if i % 2 == 0 {
            right_branches.push((i, branch));
        } else {
            left_branches.push((i, branch));
        }
    }

    // Compute side heights and widths
    let side_height = |branches: &[(usize, &Branch)]| -> f64 {
        if branches.is_empty() {
            return 0.0;
        }
        let total: f64 = branches
            .iter()
            .map(|(_, b)| branch_total_height(b))
            .sum();
        total + (branches.len().saturating_sub(1)) as f64 * BRANCH_GAP_Y
    };

    let side_max_width = |branches: &[(usize, &Branch)]| -> f64 {
        branches
            .iter()
            .map(|(_, b)| branch_max_width(b))
            .fold(0.0_f64, f64::max)
    };

    let right_h = side_height(&right_branches);
    let left_h = side_height(&left_branches);
    let right_w = side_max_width(&right_branches);
    let left_w = side_max_width(&left_branches);

    let max_side_h = right_h.max(left_h);
    let content_h = max_side_h.max(CENTER_H);

    let center_w = {
        let tw = text_width(&map.center) + BRANCH_H_PAD * 2.0;
        tw.max(CENTER_W)
    };

    let title_space = if map.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let total_w = PADDING + left_w + BRANCH_OFFSET_X + center_w + BRANCH_OFFSET_X + right_w + PADDING;
    let total_h = PADDING + title_space + content_h + PADDING;

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
    let content_y = if let Some(ref title) = map.title {
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

    // Center node position
    let center_x = PADDING + left_w + BRANCH_OFFSET_X;
    let center_y = content_y + (content_h - CENTER_H) / 2.0;
    let center_cx = center_x + center_w / 2.0;
    let center_cy = center_y + CENTER_H / 2.0;

    // Draw center node
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"#e8eaf6\" stroke=\"#283593\" stroke-width=\"2\"/>",
        center_x, center_y, center_w, CENTER_H
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\">{}</text>",
        center_cx,
        center_cy + 5.0,
        CENTER_FONT_SIZE,
        escape_xml(&map.center)
    ));

    // Draw right branches
    {
        let start_y = content_y + (content_h - right_h) / 2.0;
        let mut cur_y = start_y;
        for &(idx, branch) in &right_branches {
            let (bg, accent) = COLORS[idx % COLORS.len()];
            let node_w = branch_width(branch);
            let bx = center_x + center_w + BRANCH_OFFSET_X;
            let by = cur_y;

            // Connection line from center to branch
            let branch_cy = by + BRANCH_H / 2.0;
            svg.push_str(&format!(
                "<path d=\"M {},{} C {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                center_x + center_w, center_cy,
                center_x + center_w + BRANCH_OFFSET_X / 2.0, center_cy,
                bx - BRANCH_OFFSET_X / 2.0, branch_cy,
                bx, branch_cy,
                accent
            ));

            // Branch node
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx, by, node_w, BRANCH_H, bg, accent
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                bx + node_w / 2.0,
                by + BRANCH_H / 2.0 + 5.0,
                escape_xml(&branch.text)
            ));

            // Sub-items with vertical spine + horizontal connectors
            if !branch.children.is_empty() {
                let spine_x = bx + 8.0;
                let mut sub_y = by + BRANCH_H + BRANCH_GAP_Y;

                // Vertical spine from bottom of branch to last child
                let last_child_y = sub_y + (branch.children.len() - 1) as f64 * (SUB_ITEM_H + 4.0);
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    spine_x, by + BRANCH_H,
                    spine_x, last_child_y + SUB_ITEM_H / 2.0,
                    accent
                ));

                for child in &branch.children {
                    let sw = sub_item_width(child);
                    let sub_x = bx + 16.0;

                    // Horizontal connector from spine to sub-item
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        spine_x, sub_y + SUB_ITEM_H / 2.0,
                        sub_x, sub_y + SUB_ITEM_H / 2.0,
                        accent
                    ));

                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" opacity=\"0.7\"/>",
                        sub_x, sub_y, sw, SUB_ITEM_H, bg, accent
                    ));
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\">{}</text>",
                        sub_x + sw / 2.0,
                        sub_y + SUB_ITEM_H / 2.0 + 4.0,
                        SUB_FONT_SIZE,
                        escape_xml(&child.text)
                    ));
                    sub_y += SUB_ITEM_H + 4.0;
                }
            }

            cur_y += branch_total_height(branch) + BRANCH_GAP_Y;
        }
    }

    // Draw left branches
    {
        let start_y = content_y + (content_h - left_h) / 2.0;
        let mut cur_y = start_y;
        for &(idx, branch) in &left_branches {
            let (bg, accent) = COLORS[idx % COLORS.len()];
            let node_w = branch_width(branch);
            let bx = PADDING + left_w - node_w;
            let by = cur_y;

            // Connection line from center to branch
            let branch_cy = by + BRANCH_H / 2.0;
            svg.push_str(&format!(
                "<path d=\"M {},{} C {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                center_x, center_cy,
                center_x - BRANCH_OFFSET_X / 2.0, center_cy,
                bx + node_w + BRANCH_OFFSET_X / 2.0, branch_cy,
                bx + node_w, branch_cy,
                accent
            ));

            // Branch node
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                bx, by, node_w, BRANCH_H, bg, accent
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                bx + node_w / 2.0,
                by + BRANCH_H / 2.0 + 5.0,
                escape_xml(&branch.text)
            ));

            // Sub-items with vertical spine + horizontal connectors
            if !branch.children.is_empty() {
                let spine_x = bx + node_w - 8.0;
                let mut sub_y = by + BRANCH_H + BRANCH_GAP_Y;

                // Vertical spine from bottom of branch to last child
                let last_child_y = sub_y + (branch.children.len() - 1) as f64 * (SUB_ITEM_H + 4.0);
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    spine_x, by + BRANCH_H,
                    spine_x, last_child_y + SUB_ITEM_H / 2.0,
                    accent
                ));

                for child in &branch.children {
                    let sw = sub_item_width(child);
                    let sub_x = bx - 16.0;

                    // Horizontal connector from spine to sub-item
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        spine_x, sub_y + SUB_ITEM_H / 2.0,
                        sub_x + sw, sub_y + SUB_ITEM_H / 2.0,
                        accent
                    ));

                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" opacity=\"0.7\"/>",
                        sub_x, sub_y, sw, SUB_ITEM_H, bg, accent
                    ));
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\">{}</text>",
                        sub_x + sw / 2.0,
                        sub_y + SUB_ITEM_H / 2.0 + 4.0,
                        SUB_FONT_SIZE,
                        escape_xml(&child.text)
                    ));
                    sub_y += SUB_ITEM_H + 4.0;
                }
            }

            cur_y += branch_total_height(branch) + BRANCH_GAP_Y;
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

    let map = match parse(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mdd-mindmap: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&map));
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
center "Topic"
  Branch1
  Branch2
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.center, "Topic");
        assert_eq!(m.branches.len(), 2);
        assert_eq!(m.branches[0].text, "Branch1");
        assert_eq!(m.branches[1].text, "Branch2");
        assert!(m.title.is_none());
    }

    #[test]
    fn parse_with_subitems() {
        let input = r#"
title "Test"
center "Center"
  A
    A1
    A2
  B
    B1
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.title.as_deref(), Some("Test"));
        assert_eq!(m.center, "Center");
        assert_eq!(m.branches.len(), 2);
        assert_eq!(m.branches[0].text, "A");
        assert_eq!(m.branches[0].children.len(), 2);
        assert_eq!(m.branches[0].children[0].text, "A1");
        assert_eq!(m.branches[0].children[1].text, "A2");
        assert_eq!(m.branches[1].text, "B");
        assert_eq!(m.branches[1].children.len(), 1);
        assert_eq!(m.branches[1].children[0].text, "B1");
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
center "Topic"
  Branch1
  Branch2
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_error_no_center() {
        let input = "  Branch1\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_no_branches() {
        let input = "center \"Topic\"\n";
        assert!(parse(input).is_err());
    }
}
