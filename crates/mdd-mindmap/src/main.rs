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
            let text = strip_quotes(rest).to_string();
            let idx = nodes.len();
            nodes.push(MmNode { text, depth: 0 });
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
        nodes.push(MmNode {
            text: trimmed.to_string(),
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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";
const NODE_H_PAD: f64 = 14.0;
const NODE_V_PAD: f64 = 8.0;
const MIN_NODE_W: f64 = 60.0;
const PADDING: f64 = 60.0;
const CENTER_FONT_SIZE: f64 = 15.0;
const RING_GAP: f64 = 100.0; // distance between depth rings

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
    let font = if node.depth == 0 { CENTER_FONT_SIZE } else { FONT_SIZE };
    let char_scale = font / FONT_SIZE;
    let tw = text_width(&node.text) * char_scale + NODE_H_PAD * 2.0;
    let w = tw.max(MIN_NODE_W);
    let h = font + NODE_V_PAD * 2.0;
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
// Radial tree layout
// ---------------------------------------------------------------------------

/// Count total descendants (including self) for subtree sizing.
fn subtree_weight(node_idx: usize, children: &HashMap<usize, Vec<usize>>) -> usize {
    let mut count = 1;
    if let Some(kids) = children.get(&node_idx) {
        for &kid in kids {
            count += subtree_weight(kid, children);
        }
    }
    count
}

/// Radial layout: place nodes in concentric rings around center.
/// Returns positions as (center_x, center_y) for each node index.
fn radial_layout(map: &MindMap) -> Vec<(f64, f64)> {
    let n = map.nodes.len();
    let mut positions = vec![(0.0_f64, 0.0_f64); n];

    // Build children map
    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(parent, child) in &map.edges {
        children.entry(parent).or_default().push(child);
    }

    // Center node at origin
    positions[0] = (0.0, 0.0);

    // Recursively assign angular ranges
    let two_pi = 2.0 * std::f64::consts::PI;
    assign_positions(0, -std::f64::consts::FRAC_PI_2, two_pi, 0, &children, map, &mut positions);

    positions
}

fn assign_positions(
    node_idx: usize,
    start_angle: f64,
    sweep: f64,
    depth: usize,
    children: &HashMap<usize, Vec<usize>>,
    map: &MindMap,
    positions: &mut Vec<(f64, f64)>,
) {
    let kids = match children.get(&node_idx) {
        Some(k) => k,
        None => return,
    };

    let total_weight: usize = kids.iter().map(|&k| subtree_weight(k, children)).sum();
    if total_weight == 0 {
        return;
    }

    let radius = RING_GAP * (depth + 1) as f64;
    let mut current_angle = start_angle;

    for &kid in kids {
        let weight = subtree_weight(kid, children) as f64;
        let child_sweep = sweep * weight / total_weight as f64;
        let mid_angle = current_angle + child_sweep / 2.0;

        positions[kid] = (radius * mid_angle.cos(), radius * mid_angle.sin());

        assign_positions(kid, current_angle, child_sweep, depth + 1, children, map, positions);
        current_angle += child_sweep;
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(map: &MindMap) -> String {
    let centers = radial_layout(map);

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

        let pcx = px + pw / 2.0;
        let pcy = py + ph / 2.0;
        let ccx = cx + cw / 2.0;
        let ccy = cy + ch / 2.0;

        let (p_conn_x, p_conn_y) = nearest_rect_point(ccx, ccy, px, py, pw, ph);
        let (c_conn_x, c_conn_y) = nearest_rect_point(pcx, pcy, cx, cy, cw, ch);

        let dx = c_conn_x - p_conn_x;
        let dy = c_conn_y - p_conn_y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let ctrl_dist = len * 0.4;

        let stroke_w = if map.nodes[parent].depth == 0 { 2.5 } else { 1.5 };

        svg.push_str(&format!(
            "<path d=\"M {},{} C {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
            p_conn_x, p_conn_y,
            p_conn_x + dx / len * ctrl_dist, p_conn_y + dy / len * ctrl_dist,
            c_conn_x - dx / len * ctrl_dist, c_conn_y - dy / len * ctrl_dist,
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
        let font_size = if node.depth == 0 { CENTER_FONT_SIZE } else { FONT_SIZE };
        let opacity = if node.depth >= 2 { " opacity=\"0.8\"" } else { "" };
        let font_weight = if node.depth <= 1 { " font-weight=\"bold\"" } else { "" };

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>\n",
            x, y, w, h, rx, bg, accent, stroke_w, opacity
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\"{}>{}</text>\n",
            x + w / 2.0,
            y + h / 2.0 + font_size * 0.35,
            font_size, font_weight,
            escape_xml(&node.text)
        ));
    }

    svg.push_str("</svg>");
    svg
}

/// Find the nearest point on rectangle edge to a target point.
fn nearest_rect_point(
    target_x: f64, target_y: f64,
    rx: f64, ry: f64, rw: f64, rh: f64,
) -> (f64, f64) {
    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;
    let dx = target_x - cx;
    let dy = target_y - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy + rh / 2.0);
    }
    let mut t = f64::MAX;
    if dx.abs() > 1e-9 {
        t = t.min((rw / 2.0) / dx.abs());
    }
    if dy.abs() > 1e-9 {
        t = t.min((rh / 2.0) / dy.abs());
    }
    (cx + dx * t, cy + dy * t)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-mindmap - Render a mind map as SVG

Usage: mdd-mindmap < input.mindmap

First line declares the center node with: center \"Topic\"
Branches are indented 2 spaces. Deeper nesting uses more indentation.

Example:
  center \"Project\"
    Design
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
