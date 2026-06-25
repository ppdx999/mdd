use std::collections::HashMap;
use std::io::{self, Read};

use mdd_layout::text::{escape_xml, text_width};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MindMap {
    nodes: Vec<MmNode>,
    edges: Vec<(usize, usize)>, // parent -> child
}

#[derive(Debug)]
struct MmNode {
    text: String,
    description: Option<String>,
    depth: usize, // 0 = center, 1 = branch, 2+ = sub-item
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<MindMap, String> {
    let mut nodes: Vec<MmNode> = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();

    // Stack of (depth, node_index) to track parent at each level
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // center "..."
        if trimmed.starts_with("center ") {
            let rest = trimmed.strip_prefix("center ").unwrap().trim();
            let raw = strip_quotes(rest);
            let (text, description) = split_title_desc(raw);
            let idx = nodes.len();
            nodes.push(MmNode { text, description, depth: 0 });
            stack.clear();
            stack.push((0, idx));
            continue;
        }

        // Indented nodes
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;

        if depth == 0 {
            return Err(format!("Unknown syntax: {}", trimmed));
        }

        if nodes.is_empty() {
            return Err("Missing 'center' definition".to_string());
        }

        // Pop stack to find parent at depth-1
        while stack.len() > 1 && stack.last().unwrap().0 >= depth {
            stack.pop();
        }

        let parent_idx = stack.last().unwrap().1;
        let idx = nodes.len();
        let (text, description) = split_title_desc(trimmed);
        nodes.push(MmNode {
            text,
            description,
            depth,
        });
        edges.push((parent_idx, idx));
        stack.push((depth, idx));
    }

    if nodes.is_empty() {
        return Err("Missing 'center' definition".to_string());
    }
    if nodes.len() < 2 {
        return Err("At least 1 branch is required".to_string());
    }

    Ok(MindMap { nodes, edges })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn split_title_desc(s: &str) -> (String, Option<String>) {
    if let Some((title, desc)) = s.split_once('|') {
        let t = title.trim().to_string();
        let d = desc.trim().to_string();
        if d.is_empty() {
            (t, None)
        } else {
            (t, Some(d))
        }
    } else {
        (s.to_string(), None)
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_COLOR: &str = "#666";
const LINE_GAP: f64 = 8.0; // gap between title and description lines
const COLOR_DARK: &str = "#333";
const NODE_H_PAD: f64 = 14.0;
const NODE_V_PAD: f64 = 8.0;
const MIN_NODE_W: f64 = 60.0;
const PADDING: f64 = 60.0;
const CENTER_FONT_SIZE: f64 = 26.0;
const CENTER_H_PAD: f64 = 14.0;
const CENTER_V_PAD: f64 = 8.0;
const HORIZONTAL_GAP: f64 = 40.0; // gap between depth levels
const LEAF_MARGIN: f64 = 6.0; // gap between leaf siblings (same parent)
const SUBTREE_MARGIN: f64 = 20.0; // gap between sibling subtrees (different parent groups)
const BRANCH_GAP: f64 = 32.0; // gap between top-level branches

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

// ---------------------------------------------------------------------------
// Sizing
// ---------------------------------------------------------------------------

fn node_size(node: &MmNode) -> (f64, f64) {
    let (font, h_pad, v_pad) = if node.depth == 0 {
        (CENTER_FONT_SIZE, CENTER_H_PAD, CENTER_V_PAD)
    } else {
        (FONT_SIZE, NODE_H_PAD, NODE_V_PAD)
    };
    let char_scale = font / FONT_SIZE;
    // Bold text is ~5-7% wider than normal
    let bold = node.depth <= 1 || node.description.is_some();
    let bold_factor = if bold { 1.07 } else { 1.0 };
    let title_w = text_width(&node.text) * char_scale * bold_factor + h_pad * 2.0;

    let (w, h) = if let Some(desc) = &node.description {
        let desc_scale = DESC_FONT_SIZE / FONT_SIZE;
        let desc_w = text_width(desc) * desc_scale + h_pad * 2.0;
        let w = title_w.max(desc_w).max(MIN_NODE_W);
        let h = font + LINE_GAP + DESC_FONT_SIZE + v_pad * 2.0;
        (w, h)
    } else {
        (title_w.max(MIN_NODE_W), font + v_pad * 2.0)
    };
    (w, h)
}

/// Determine which color branch a node belongs to (by its top-level ancestor).
fn branch_color_index(node_idx: usize, edges: &[(usize, usize)]) -> usize {
    let mut current = node_idx;
    loop {
        if let Some(edge) = edges.iter().find(|(_, child)| *child == current) {
            if edge.0 == 0 {
                return edges
                    .iter()
                    .filter(|(p, _)| *p == 0)
                    .position(|(_, c)| *c == current)
                    .unwrap_or(0);
            }
            current = edge.0;
        } else {
            return 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Horizontal tree layout
// ---------------------------------------------------------------------------

/// Pick margin between sibling nodes: small for leaves, larger for subtrees.
fn sibling_margin(kids: &[usize], children: &HashMap<usize, Vec<usize>>) -> f64 {
    let any_has_children = kids
        .iter()
        .any(|&k| children.get(&k).is_some_and(|c| !c.is_empty()));
    if any_has_children {
        SUBTREE_MARGIN
    } else {
        LEAF_MARGIN
    }
}

/// Compute the vertical space needed for a subtree.
fn subtree_height(
    node_idx: usize,
    children: &HashMap<usize, Vec<usize>>,
    nodes: &[MmNode],
) -> f64 {
    match children.get(&node_idx) {
        Some(kids) if !kids.is_empty() => {
            let margin = sibling_margin(kids, children);
            let sum: f64 = kids
                .iter()
                .map(|&k| subtree_height(k, children, nodes))
                .sum();
            sum + (kids.len() - 1) as f64 * margin
        }
        _ => {
            let (_, h) = node_size(&nodes[node_idx]);
            h
        }
    }
}

/// Recursively position a subtree within a vertical range.
/// `depth_inner_edge[d]` is the inner-edge x for depth d (left edge for right side).
fn layout_subtree(
    node_idx: usize,
    y_start: f64,
    y_end: f64,
    direction: f64,
    children: &HashMap<usize, Vec<usize>>,
    nodes: &[MmNode],
    positions: &mut [(f64, f64)],
    depth_inner_edge: &[f64],
) {
    let depth = nodes[node_idx].depth;
    let y = (y_start + y_end) / 2.0;
    let x = if depth == 0 {
        0.0
    } else {
        let (w, _) = node_size(&nodes[node_idx]);
        // Right side: left-align (inner edge = left edge)
        // Left side: right-align (inner edge = right edge, negated)
        (depth_inner_edge[depth] + w / 2.0) * direction
    };
    positions[node_idx] = (x, y);

    let kids = match children.get(&node_idx) {
        Some(k) if !k.is_empty() => k,
        _ => return,
    };

    let margin = sibling_margin(kids, children);
    let mut current_y = y_start;
    for &kid in kids {
        let h = subtree_height(kid, children, nodes);
        layout_subtree(
            kid,
            current_y,
            current_y + h,
            direction,
            children,
            nodes,
            positions,
            depth_inner_edge,
        );
        current_y += h + margin;
    }
}

/// Layout one side (left or right) of the tree.
fn layout_side(
    kids: &[usize],
    direction: f64,
    children: &HashMap<usize, Vec<usize>>,
    nodes: &[MmNode],
    positions: &mut [(f64, f64)],
    depth_inner_edge: &[f64],
) {
    if kids.is_empty() {
        return;
    }
    let total_h: f64 = kids
        .iter()
        .map(|&k| subtree_height(k, children, nodes))
        .sum::<f64>()
        + (kids.len() - 1) as f64 * BRANCH_GAP;
    let mut y = -total_h / 2.0;
    for &kid in kids {
        let h = subtree_height(kid, children, nodes);
        layout_subtree(
            kid, y, y + h, direction, children, nodes, positions, depth_inner_edge,
        );
        y += h + BRANCH_GAP;
    }
}

/// Horizontal tree layout: branches extend left and right from center.
fn tree_layout(map: &MindMap) -> Vec<(f64, f64)> {
    let n = map.nodes.len();
    let mut positions = vec![(0.0_f64, 0.0_f64); n];

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(parent, child) in &map.edges {
        children.entry(parent).or_default().push(child);
    }

    let root_kids = match children.get(&0) {
        Some(k) => k.clone(),
        None => return positions,
    };

    // Compute max node width per depth for horizontal positioning
    let max_depth = map.nodes.iter().map(|nd| nd.depth).max().unwrap_or(0);
    let mut max_width = vec![0.0_f64; max_depth + 1];
    for node in &map.nodes {
        let (w, _) = node_size(node);
        max_width[node.depth] = max_width[node.depth].max(w);
    }

    // Inner-edge x positions per depth.
    // For right side: this is the left edge of nodes at depth d.
    // For left side: negated, this is the right edge.
    let mut depth_inner_edge = vec![0.0_f64; max_depth + 1];
    for d in 1..=max_depth {
        if d == 1 {
            // First level starts after center node's right edge + gap
            depth_inner_edge[d] = max_width[0] / 2.0 + HORIZONTAL_GAP;
        } else {
            depth_inner_edge[d] = depth_inner_edge[d - 1] + max_width[d - 1] + HORIZONTAL_GAP;
        }
    }

    // Split branches: first half right, second half left
    let n_right = (root_kids.len() + 1) / 2;

    positions[0] = (0.0, 0.0);
    layout_side(
        &root_kids[..n_right],
        1.0,
        &children,
        &map.nodes,
        &mut positions,
        &depth_inner_edge,
    );
    layout_side(
        &root_kids[n_right..],
        -1.0,
        &children,
        &map.nodes,
        &mut positions,
        &depth_inner_edge,
    );

    positions
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(map: &MindMap) -> String {
    let centers = tree_layout(map);

    // Compute node rects (top-left x, y, w, h) from center positions
    let mut rects: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for (i, node) in map.nodes.iter().enumerate() {
        let (w, h) = node_size(node);
        let (cx, cy) = centers[i];
        let x = cx - w / 2.0;
        let y = cy - h / 2.0;
        rects.push((x, y, w, h));
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    // Shift everything so min is at PADDING
    let offset_x = PADDING - min_x;
    let offset_y = PADDING - min_y;
    for r in rects.iter_mut() {
        r.0 += offset_x;
        r.1 += offset_y;
    }

    let svg_width = max_x - min_x + PADDING * 2.0;
    let svg_height = max_y - min_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Draw edges (curves behind nodes)
    for &(parent, child) in &map.edges {
        let (px, py, pw, ph) = rects[parent];
        let (cx, cy, cw, ch) = rects[child];

        let color_idx = branch_color_index(child, &map.edges);
        let (_, accent) = COLORS[color_idx % COLORS.len()];

        let pcy = py + ph / 2.0;
        let ccy = cy + ch / 2.0;

        // Connect from side-edge centers: parent's outer edge to child's inner edge
        let child_is_right = (cx + cw / 2.0) > (px + pw / 2.0);
        let (p_conn_x, p_conn_y) = if child_is_right {
            (px + pw, pcy) // parent right edge center
        } else {
            (px, pcy) // parent left edge center
        };
        let (c_conn_x, c_conn_y) = if child_is_right {
            (cx, ccy) // child left edge center
        } else {
            (cx + cw, ccy) // child right edge center
        };

        let stroke_w = if map.nodes[parent].depth == 0 { 2.5 } else { 1.5 };

        svg.push_str(&format!(
            "<path d=\"M {},{} L {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
            p_conn_x, p_conn_y,
            c_conn_x, c_conn_y,
            accent, stroke_w
        ));
    }

    // Draw nodes (on top of edges)
    for (i, node) in map.nodes.iter().enumerate() {
        let (x, y, w, h) = rects[i];

        let (bg, accent) = if node.depth == 0 {
            ("#e8eaf6", "#283593")
        } else {
            let color_idx = branch_color_index(i, &map.edges);
            COLORS[color_idx % COLORS.len()]
        };

        let rx = if node.depth == 0 { 14.0 } else { 6.0 };
        let stroke_w = if node.depth == 0 { 2.5 } else { 1.5 };
        let (font_size, v_pad) = if node.depth == 0 {
            (CENTER_FONT_SIZE, CENTER_V_PAD)
        } else {
            (FONT_SIZE, NODE_V_PAD)
        };
        let opacity = if node.depth >= 2 { " opacity=\"0.8\"" } else { "" };
        let font_weight = if node.depth <= 1 || node.description.is_some() {
            " font-weight=\"bold\""
        } else {
            ""
        };

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            x, y, w, h, rx, bg, accent, stroke_w, opacity
        ));

        if let Some(desc) = &node.description {
            // Two-line node: title above, description below
            let title_y = y + v_pad + font_size * 0.8;
            let desc_y = title_y + LINE_GAP + DESC_FONT_SIZE * 0.8;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\"{}>{}</text>\n",
                x + w / 2.0, title_y, font_size, font_weight,
                escape_xml(&node.text)
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>\n",
                x + w / 2.0, desc_y, DESC_FONT_SIZE, DESC_COLOR,
                escape_xml(desc)
            ));
        } else {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\"{}>{}</text>\n",
                x + w / 2.0,
                y + h / 2.0 + font_size * 0.35,
                font_size, font_weight,
                escape_xml(&node.text)
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
mdd-mindmap - Render a mind map as SVG

Usage: mdd-mindmap < input.mindmap

First line declares the center node with: center \"Topic\"
Branches are indented 2 spaces. Deeper nesting uses more indentation.
Use | to add a description below the title: Title | Description

Example:
  center \"Project | Overview\"
    Design | Visual design
      Colors
      Layout
    Backend
      API
        REST
        GraphQL
      Database
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
        assert_eq!(m.nodes.len(), 3);
        assert_eq!(m.nodes[0].text, "Topic");
        assert_eq!(m.nodes[0].depth, 0);
        assert_eq!(m.nodes[1].text, "Branch1");
        assert_eq!(m.nodes[1].depth, 1);
        assert_eq!(m.edges.len(), 2);
    }

    #[test]
    fn parse_with_subitems() {
        let input = r#"
center "Center"
  A
    A1
    A2
  B
    B1
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.nodes.len(), 6);
        assert_eq!(m.nodes[2].text, "A1");
        assert_eq!(m.nodes[2].depth, 2);
        assert!(m.edges.contains(&(1, 2)));
        assert!(m.edges.contains(&(1, 3)));
        assert!(m.edges.contains(&(0, 4)));
    }

    #[test]
    fn parse_deep_nesting() {
        let input = r#"
center "Root"
  L1
    L2
      L3
        L4
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.nodes.len(), 5);
        assert_eq!(m.nodes[4].depth, 4);
        assert!(m.edges.contains(&(3, 4)));
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
