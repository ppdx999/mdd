use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Element {
    Node {
        label: String,
    },
    Group {
        label: String,
        children: Vec<String>,
    },
}

#[derive(Debug)]
struct Edge {
    from: usize,
    to: usize,
}

#[derive(Debug)]
struct Diagram {
    elements: Vec<Element>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut elements: Vec<Element> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    let mut in_group = false;
    let mut group_label = String::new();
    let mut group_children: Vec<String> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Inside group block
        if in_group {
            if line == "}" {
                let id = elements.len();
                name_to_id.insert(group_label.clone(), id);
                elements.push(Element::Group {
                    label: group_label.clone(),
                    children: group_children.clone(),
                });
                in_group = false;
                group_label.clear();
                group_children.clear();
                continue;
            }
            if line.starts_with("node ") {
                let label = line.strip_prefix("node ").unwrap().trim().to_string();
                group_children.push(label);
                continue;
            }
            return Err(format!("Invalid syntax inside group: {}", line));
        }

        if line.starts_with("node ") {
            let label = line.strip_prefix("node ").unwrap().trim().to_string();
            let id = elements.len();
            name_to_id.insert(label.clone(), id);
            elements.push(Element::Node { label });
            continue;
        }

        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(rest) = rest.strip_suffix(" {") {
                group_label = rest.trim().trim_matches('"').to_string();
                group_children.clear();
                in_group = true;
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from_name = parts[0].trim().trim_matches('"');
            let to_name = parts[1].trim().trim_matches('"');
            let from_id = name_to_id
                .get(from_name)
                .ok_or_else(|| format!("Unknown element: {}", from_name))?;
            let to_id = name_to_id
                .get(to_name)
                .ok_or_else(|| format!("Unknown element: {}", to_name))?;
            edges.push(Edge {
                from: *from_id,
                to: *to_id,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_group {
        return Err(format!("Unclosed group block: {}", group_label));
    }

    Ok(Diagram { elements, edges })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

// Node
const NODE_H_PAD: f64 = 16.0;
const NODE_V_PAD: f64 = 10.0;
const NODE_MIN_W: f64 = 80.0;
const NODE_MIN_H: f64 = 36.0;

// Group
const GROUP_H_PAD: f64 = 12.0;
const GROUP_HEADER_H: f64 = 28.0;
const GROUP_INNER_GAP: f64 = 8.0;
const GROUP_COLS_MAX: usize = 4;

// Colors
const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#999";
const COLOR_NODE_FILL: &str = "#f0f8ff";
const COLOR_NODE_STROKE: &str = "#336699";
const COLOR_GROUP_FILL: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#999";

// ---------------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------------

struct SpacingConfig {
    nodesep: f64,
    ranksep: f64,
    component_gap: f64,
    vertex_spacing: f64,
}

fn compute_spacing(diagram: &Diagram) -> SpacingConfig {
    let complexity = diagram.elements.len() + diagram.edges.len();
    let factor = if complexity <= 10 {
        1.0 + (complexity as f64 / 20.0).sqrt() * 0.4
    } else if complexity <= 30 {
        1.0 + (complexity as f64 / 10.0).sqrt() * 0.6
    } else {
        2.0 + (complexity - 30) as f64 * 0.06
    }
    .min(5.0);

    let elem_count = diagram.elements.len() as f64;
    SpacingConfig {
        nodesep: 25.0 * factor,
        ranksep: 50.0 * factor,
        component_gap: 30.0 * factor,
        vertex_spacing: 8.0 + elem_count * 3.0,
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

// ---------------------------------------------------------------------------
// Element sizing
// ---------------------------------------------------------------------------

fn node_box_size(label: &str) -> (f64, f64) {
    let w = (text_width(label) + NODE_H_PAD * 2.0).max(NODE_MIN_W);
    let h = (LINE_HEIGHT + NODE_V_PAD * 2.0).max(NODE_MIN_H);
    (w, h)
}

fn group_size(label: &str, children: &[String]) -> (f64, f64) {
    if children.is_empty() {
        let w = (text_width(label) + GROUP_H_PAD * 2.0).max(NODE_MIN_W);
        let h = GROUP_HEADER_H + GROUP_H_PAD;
        return (w, h);
    }

    // Compute child sizes
    let child_sizes: Vec<(f64, f64)> = children.iter().map(|c| node_box_size(c)).collect();

    // Arrange in grid (up to GROUP_COLS_MAX columns)
    let cols = children.len().min(GROUP_COLS_MAX);
    let rows = (children.len() + cols - 1) / cols;

    // Find max width per column and max height per row
    let mut col_widths = vec![0.0_f64; cols];
    let mut row_heights = vec![0.0_f64; rows];
    for (i, (w, h)) in child_sizes.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        col_widths[col] = col_widths[col].max(*w);
        row_heights[row] = row_heights[row].max(*h);
    }

    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (cols as f64 - 1.0).max(0.0) * GROUP_INNER_GAP;
    let inner_h: f64 =
        row_heights.iter().sum::<f64>() + (rows as f64 - 1.0).max(0.0) * GROUP_INNER_GAP;

    let header_w = text_width(label) + GROUP_H_PAD * 2.0;
    let w = inner_w.max(header_w) + GROUP_H_PAD * 2.0;
    let h = GROUP_HEADER_H + inner_h + GROUP_H_PAD * 2.0;
    (w, h)
}

fn element_size(elem: &Element) -> (f64, f64) {
    match elem {
        Element::Node { label } => node_box_size(label),
        Element::Group { label, children } => group_size(label, children),
    }
}

// ---------------------------------------------------------------------------
// Layout & SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let sp = compute_spacing(diagram);

    let config = Config {
        vertex_spacing: sp.vertex_spacing,
        ..Config::default()
    };

    // Top-down layout (no LTR swap)
    let vertices: Vec<(u32, (f64, f64))> = diagram
        .elements
        .iter()
        .enumerate()
        .map(|(i, elem)| {
            let (w, h) = element_size(elem);
            (i as u32, (w, h))
        })
        .collect();

    let edges: Vec<(u32, u32)> = diagram.edges.iter().map(|e| (e.from as u32, e.to as u32)).collect();

    let layouts = from_vertices_and_edges(&vertices, &edges, &config);

    // Post-scale for nodesep/ranksep
    let base = sp.vertex_spacing.max(1.0);
    let nodesep_ratio = sp.nodesep / base;
    let ranksep_ratio = sp.ranksep / base;

    let scaled_components: Vec<(HashMap<usize, (f64, f64)>, f64, f64)> = layouts
        .iter()
        .map(|(coords, _w, _h)| {
            let n = coords.len() as f64;
            let cx = coords.iter().map(|(_, (x, _))| x).sum::<f64>() / n;
            let cy = coords.iter().map(|(_, (_, y))| y).sum::<f64>() / n;

            let mut scaled: HashMap<usize, (f64, f64)> = HashMap::new();
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for &(id, (sx, sy)) in coords {
                // Top-down: X = horizontal (nodesep), Y = vertical (ranksep)
                let final_x = cx + (sx - cx) * nodesep_ratio;
                let final_y = cy + (sy - cy) * ranksep_ratio;

                let (w, h) = element_size(&diagram.elements[id]);
                min_x = min_x.min(final_x);
                min_y = min_y.min(final_y);
                max_x = max_x.max(final_x + w);
                max_y = max_y.max(final_y + h);

                scaled.insert(id, (final_x, final_y));
            }

            let comp_w = (max_x - min_x).max(0.0);
            let comp_h = (max_y - min_y).max(0.0);
            (scaled, comp_w, comp_h)
        })
        .collect();

    // Row-based packing
    let total_area: f64 = scaled_components
        .iter()
        .map(|(_, w, h)| (w + sp.component_gap) * (h + sp.component_gap))
        .sum();
    let target_width = total_area.sqrt() * 1.3;

    let mut comp_indices: Vec<usize> = (0..scaled_components.len()).collect();
    comp_indices.sort_by(|a, b| {
        scaled_components[*b]
            .2
            .partial_cmp(&scaled_components[*a].2)
            .unwrap()
    });

    let mut positions: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut row_x: f64 = 0.0;
    let mut row_y: f64 = 0.0;
    let mut row_max_height: f64 = 0.0;

    for &ci in &comp_indices {
        let (ref coords, comp_w, comp_h) = scaled_components[ci];

        if row_x > 0.0 && row_x + comp_w > target_width {
            row_y += row_max_height + sp.component_gap;
            row_x = 0.0;
            row_max_height = 0.0;
        }

        let cmin_x = coords.values().map(|(x, _)| *x).fold(f64::MAX, f64::min);
        let cmin_y = coords.values().map(|(_, y)| *y).fold(f64::MAX, f64::min);

        for (&id, &(x, y)) in coords {
            positions.insert(id, (x - cmin_x + row_x, y - cmin_y + row_y));
        }

        row_x += comp_w + sp.component_gap;
        row_max_height = row_max_height.max(comp_h);
    }

    // Compute SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (i, elem) in diagram.elements.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = element_size(elem);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING * 2.0;
    let svg_height = max_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );

    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Render edges (behind nodes)
    for edge in &diagram.edges {
        let (x1, y1) = positions.get(&edge.from).copied().unwrap_or((0.0, 0.0));
        let (x2, y2) = positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = element_size(&diagram.elements[edge.from]);
        let (tw, _th) = element_size(&diagram.elements[edge.to]);

        let cx1 = PADDING + x1 + fw / 2.0;
        let cx2 = PADDING + x2 + tw / 2.0;

        // Connect bottom of parent to top of child
        let ay1 = PADDING + y1 + fh;
        let ay2 = PADDING + y2;

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            cx1, ay1, cx2, ay2, COLOR_EDGE
        ));
    }

    // Render elements
    for (i, elem) in diagram.elements.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let px = PADDING + x;
        let py = PADDING + y;

        match elem {
            Element::Node { label } => render_node(&mut svg, px, py, label),
            Element::Group { label, children } => render_group(&mut svg, px, py, label, children),
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_node(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = node_box_size(label);
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, COLOR_NODE_FILL, COLOR_NODE_STROKE
    ));
    let cx = x + w / 2.0;
    let cy = y + h / 2.0 + LINE_HEIGHT * 0.35;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
        cx, cy, escape_xml(label)
    ));
}

fn render_group(svg: &mut String, x: f64, y: f64, label: &str, children: &[String]) {
    let (w, h) = group_size(label, children);

    // Group background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"5,5\"/>",
        x, y, w, h, COLOR_GROUP_FILL, COLOR_GROUP_STROKE
    ));

    // Header label
    let cx = x + w / 2.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        cx,
        y + GROUP_HEADER_H * 0.7,
        escape_xml(label)
    ));

    if children.is_empty() {
        return;
    }

    // Arrange children in grid
    let child_sizes: Vec<(f64, f64)> = children.iter().map(|c| node_box_size(c)).collect();
    let cols = children.len().min(GROUP_COLS_MAX);
    let rows = (children.len() + cols - 1) / cols;

    let mut col_widths = vec![0.0_f64; cols];
    let mut row_heights = vec![0.0_f64; rows];
    for (i, (cw, ch)) in child_sizes.iter().enumerate() {
        col_widths[i % cols] = col_widths[i % cols].max(*cw);
        row_heights[i / cols] = row_heights[i / cols].max(*ch);
    }

    // Center the grid within the group
    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (cols as f64 - 1.0).max(0.0) * GROUP_INNER_GAP;
    let start_x = x + (w - inner_w) / 2.0;
    let start_y = y + GROUP_HEADER_H + GROUP_H_PAD;

    for (i, child_label) in children.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;

        let col_x: f64 = col_widths[..col].iter().sum::<f64>() + col as f64 * GROUP_INNER_GAP;
        let row_y: f64 = row_heights[..row].iter().sum::<f64>() + row as f64 * GROUP_INNER_GAP;

        // Center node within its cell
        let (cw, ch) = child_sizes[i];
        let cell_w = col_widths[col];
        let cell_h = row_heights[row];
        let nx = start_x + col_x + (cell_w - cw) / 2.0;
        let ny = start_y + row_y + (cell_h - ch) / 2.0;

        render_node(svg, nx, ny, child_label);
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

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-tree: {}", e);
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
    fn parse_node() {
        let d = parse("node CEO\n").unwrap();
        assert_eq!(d.elements.len(), 1);
        assert!(matches!(&d.elements[0], Element::Node { label } if label == "CEO"));
    }

    #[test]
    fn parse_group() {
        let input = "group \"営業部\" {\n  node 部長\n  node 社員A\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.elements.len(), 1);
        match &d.elements[0] {
            Element::Group { label, children } => {
                assert_eq!(label, "営業部");
                assert_eq!(children.len(), 2);
                assert_eq!(children[0], "部長");
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn parse_edge() {
        let input = "node A\nnode B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, 0);
        assert_eq!(d.edges[0].to, 1);
    }

    #[test]
    fn parse_edge_with_quoted_group() {
        let input = "node CEO\ngroup \"営業部\" {\n  node 部長\n}\nCEO -> \"営業部\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
    }

    #[test]
    fn node_only_diagram() {
        let input = "node A\nnode B\nnode C\nA -> B\nB -> C\n";
        let d = parse(input).unwrap();
        assert_eq!(d.elements.len(), 3);
        assert_eq!(d.edges.len(), 2);
    }

    #[test]
    fn group_with_no_children() {
        let input = "group \"空グループ\" {\n}\n";
        let d = parse(input).unwrap();
        match &d.elements[0] {
            Element::Group { children, .. } => assert!(children.is_empty()),
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn render_produces_svg() {
        let input = "node A\nnode B\nA -> B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn spacing_scales_with_complexity() {
        let small = parse("node A\nnode B\nA -> B\n").unwrap();
        let small_sp = compute_spacing(&small);

        let big_input = "node A\nnode B\nnode C\nnode D\nnode E\n\
                         A -> B\nA -> C\nA -> D\nA -> E\nB -> C\n";
        let big = parse(big_input).unwrap();
        let big_sp = compute_spacing(&big);

        assert!(big_sp.nodesep > small_sp.nodesep);
        assert!(big_sp.ranksep > small_sp.ranksep);
    }
}
