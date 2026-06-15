use std::collections::HashMap;
use std::io::{self, Read};

use mdd_layout::edge::{build_smooth_path, clip_to_rect, midpoint_on_path, route_around_nodes};
use mdd_layout::text::{escape_xml, text_width};
use mdd_layout::{LayoutConfig, LayoutEdge, LayoutElement, LayoutGraph, LayoutGroup, LayoutNode};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum NodeKind {
    Entity,
    Process,
    DataStore { columns: Vec<String> },
}

#[derive(Debug)]
struct Node {
    label: String,
    kind: NodeKind,
}

#[derive(Debug)]
struct Boundary {
    label: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    NodeRef(usize),
    BoundaryRef(usize),
}

#[derive(Debug)]
struct Edge {
    from: String,
    to: String,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    nodes: Vec<Node>,
    boundaries: Vec<Boundary>,
    top_level: Vec<Element>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut boundaries: Vec<Boundary> = Vec::new();
    let mut top_level: Vec<Element> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    let mut in_datastore = false;
    let mut ds_name = String::new();
    let mut ds_columns: Vec<String> = Vec::new();

    // Stack for nested boundaries: (boundary_index, children_so_far)
    let mut boundary_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Inside datastore block
        if in_datastore {
            if line == "}" {
                let id = nodes.len();
                name_to_id.insert(ds_name.clone(), id);
                nodes.push(Node {
                    label: ds_name.clone(),
                    kind: NodeKind::DataStore {
                        columns: ds_columns.clone(),
                    },
                });
                let elem = Element::NodeRef(id);
                if let Some(parent) = boundary_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
                in_datastore = false;
                ds_name.clear();
                ds_columns.clear();
                continue;
            }
            ds_columns.push(line.to_string());
            continue;
        }

        // Close boundary block
        if line == "}" {
            if let Some((bidx, children)) = boundary_stack.pop() {
                boundaries[bidx].children = children;
                let elem = Element::BoundaryRef(bidx);
                if let Some(parent) = boundary_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
            } else {
                return Err("Unexpected }".to_string());
            }
            continue;
        }

        // boundary "Name" {
        if line.starts_with("boundary ") {
            let rest = line.strip_prefix("boundary ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let bidx = boundaries.len();
                boundaries.push(Boundary {
                    label: name,
                    children: Vec::new(),
                });
                boundary_stack.push((bidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid boundary syntax: {}", line));
        }

        if line.starts_with("entity ") {
            let label = line.strip_prefix("entity ").unwrap().trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::Entity,
            });
            let elem = Element::NodeRef(id);
            if let Some(parent) = boundary_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        if line.starts_with("process ") {
            let label = line.strip_prefix("process ").unwrap().trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::Process,
            });
            let elem = Element::NodeRef(id);
            if let Some(parent) = boundary_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        if line.starts_with("datastore ") {
            let rest = line.strip_prefix("datastore ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                ds_name = name.trim().to_string();
                ds_columns.clear();
                in_datastore = true;
                continue;
            }
            // Single-line datastore without columns
            let label = rest.trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::DataStore {
                    columns: Vec::new(),
                },
            });
            let elem = Element::NodeRef(id);
            if let Some(parent) = boundary_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        if line.starts_with("flow ") || line.contains(" -> ") {
            let line = if let Some(rest) = line.strip_prefix("flow ") {
                rest
            } else {
                line
            };
            // Parse: From -> To : "label"  or  From -> To
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            if parts.len() < 2 {
                return Err(format!("Invalid flow syntax: {}", line));
            }
            let from = parts[0].trim().to_string();
            let rest = parts[1];

            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim().to_string(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };

            // Validate node names exist
            if !name_to_id.contains_key(&from) {
                return Err(format!("Unknown node: {}", from));
            }
            if !name_to_id.contains_key(&to) {
                return Err(format!("Unknown node: {}", to));
            }

            edges.push(Edge { from, to, label });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_datastore {
        return Err(format!("Unclosed datastore block: {}", ds_name));
    }

    if !boundary_stack.is_empty() {
        return Err("Unclosed boundary block".to_string());
    }

    Ok(Diagram {
        nodes,
        boundaries,
        top_level,
        edges,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;
const MAX_LINE_CHARS: usize = 16;

// Entity
const ENTITY_H_PAD: f64 = 20.0;
const ENTITY_V_PAD: f64 = 12.0;
const ENTITY_MIN_W: f64 = 120.0;
const ENTITY_MIN_H: f64 = 40.0;

// Process
const PROCESS_MIN_R: f64 = 35.0;
const PROCESS_PAD: f64 = 20.0;

// DataStore
const DS_H_PAD: f64 = 16.0;
const DS_HEADER_H: f64 = 24.0;
const DS_MIN_W: f64 = 140.0;
const DS_COL_GAP: f64 = 12.0;
const DS_MAX_ROWS: usize = 8;

// Colors
const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_ENTITY_FILL: &str = "#fff5ee";
const COLOR_ENTITY_STROKE: &str = "#996633";
const COLOR_PROCESS_FILL: &str = "#f0f8ff";
const COLOR_PROCESS_STROKE: &str = "#336699";
const COLOR_DS_FILL: &str = "#f0fff0";
const COLOR_DS_STROKE: &str = "#339966";
const COLOR_BOUNDARY_STROKE: &str = "#cc3333";
const COLOR_BOUNDARY_FILL: &str = "#fff8f8";

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn split_label(label: &str) -> Vec<String> {
    if label.contains(' ') {
        return label.split_whitespace().map(|s| s.to_string()).collect();
    }
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in label.chars() {
        if ch.is_uppercase() && !current.is_empty() {
            words.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn wrap_lines(label: &str) -> Vec<String> {
    let words = split_label(label);
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word;
        } else if current_line.len() + 1 + word.len() <= MAX_LINE_CHARS {
            current_line.push(' ');
            current_line.push_str(&word);
        } else {
            lines.push(current_line);
            current_line = word;
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

// ---------------------------------------------------------------------------
// Node sizing
// ---------------------------------------------------------------------------

fn entity_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f64, f64::max);
    let w = (max_w + ENTITY_H_PAD * 2.0).max(ENTITY_MIN_W);
    let h = (lines.len() as f64 * LINE_HEIGHT + ENTITY_V_PAD * 2.0).max(ENTITY_MIN_H);
    (w, h)
}

fn process_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f64, f64::max);
    let text_h = lines.len() as f64 * LINE_HEIGHT;
    let r = ((max_w + PROCESS_PAD).max(text_h + PROCESS_PAD) / 2.0).max(PROCESS_MIN_R);
    let d = r * 2.0;
    (d, d)
}

/// Compute multi-column grid layout for datastore columns.
fn ds_column_layout(columns: &[String]) -> (usize, Vec<f64>, usize) {
    if columns.is_empty() {
        return (1, vec![0.0], 0);
    }
    let num_cols = ((columns.len() + DS_MAX_ROWS - 1) / DS_MAX_ROWS).max(1);
    let num_rows = (columns.len() + num_cols - 1) / num_cols;
    let mut col_widths = vec![0.0_f64; num_cols];
    for (i, col) in columns.iter().enumerate() {
        let c = i / num_rows;
        col_widths[c] = col_widths[c].max(text_width(col));
    }
    (num_cols, col_widths, num_rows)
}

fn datastore_size(label: &str, columns: &[String]) -> (f64, f64) {
    let header_w = text_width(label) + DS_H_PAD * 2.0;
    let (num_cols, col_widths, num_rows) = ds_column_layout(columns);
    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let w = header_w.max(inner_w + DS_H_PAD * 2.0).max(DS_MIN_W);
    let h = DS_HEADER_H + num_rows as f64 * LINE_HEIGHT + 8.0;
    (w, h)
}

fn node_size(node: &Node) -> (f64, f64) {
    match &node.kind {
        NodeKind::Entity => entity_size(&node.label),
        NodeKind::Process => process_size(&node.label),
        NodeKind::DataStore { columns } => datastore_size(&node.label, columns),
    }
}

// ---------------------------------------------------------------------------
// Build LayoutGraph from Diagram
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> LayoutGraph {
    let mut graph = LayoutGraph::new();

    // Add nodes with their computed sizes
    for node in &diagram.nodes {
        let (w, h) = node_size(node);
        graph.nodes.push(LayoutNode {
            name: node.label.clone(),
            width: w,
            height: h,
        });
    }

    // Add edges
    for edge in &diagram.edges {
        graph.edges.push(LayoutEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
        });
    }

    // Add boundaries as groups and build element tree
    for boundary in &diagram.boundaries {
        graph.groups.push(LayoutGroup {
            name: boundary.label.clone(),
            children: convert_elements(&boundary.children),
        });
    }

    graph.top_level = convert_elements(&diagram.top_level);

    graph
}

fn convert_elements(elements: &[Element]) -> Vec<LayoutElement> {
    elements
        .iter()
        .map(|e| match e {
            Element::NodeRef(i) => LayoutElement::NodeRef(*i),
            Element::BoundaryRef(i) => LayoutElement::GroupRef(*i),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);
    // Compute max process radius for group padding
    let max_process_r = diagram.nodes.iter()
        .filter(|n| matches!(n.kind, NodeKind::Process))
        .map(|n| { let (w, _) = process_size(&n.label); w / 2.0 })
        .fold(0.0_f64, f64::max);
    let group_pad = (40.0_f64).max(max_process_r * 0.7 + 30.0);

    let config = LayoutConfig {
        padding: PADDING,
        group_h_pad: group_pad,
        group_v_pad: group_pad,
        group_header_h: 28.0,
        ..LayoutConfig::default()
    };
    let result = mdd_layout::layout(&graph, &config);
    let positions = &result.positions;
    let edge_waypoints = &result.edge_waypoints;

    // Compute SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (_, (x, y, w, h)) in positions {
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING;
    let svg_height = max_y + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );

    // Styles & arrow marker
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render boundaries and nodes (back to front)
    render_elements_recursive(
        &mut svg,
        &diagram.top_level,
        &diagram.nodes,
        &diagram.boundaries,
        positions,
    );

    // Build node bounds for edge routing
    let all_bounds: Vec<(String, f64, f64, f64, f64)> = diagram
        .nodes
        .iter()
        .filter_map(|n| {
            positions.get(&n.label).map(|(x, y, w, h)| {
                (n.label.clone(), x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)
            })
        })
        .collect();

    // Reciprocal edge counting
    let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
    for e in &diagram.edges {
        let key = if e.from <= e.to {
            (e.from.clone(), e.to.clone())
        } else {
            (e.to.clone(), e.from.clone())
        };
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();

    // Render edges
    for edge in &diagram.edges {
        let from_pos = positions.get(&edge.from);
        let to_pos = positions.get(&edge.to);
        if from_pos.is_none() || to_pos.is_none() {
            continue;
        }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        let pair_key = if edge.from <= edge.to {
            (edge.from.clone(), edge.to.clone())
        } else {
            (edge.to.clone(), edge.from.clone())
        };
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = {
            let seen = pair_seen.entry(pair_key).or_insert(0);
            let v = *seen;
            *seen += 1;
            v
        };
        let offset = if total > 1 {
            (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0
        } else {
            0.0
        };

        // Use virtual node waypoints if available, otherwise route around nodes
        let edge_key = format!("\u{2192}{}\u{2192}{}", edge.from, edge.to);
        let layout_edge_key = format!("{}→{}", edge.from, edge.to);
        let route = if let Some(waypoints) = edge_waypoints.get(&layout_edge_key) {
            let mut r = vec![(cx1, cy1)];
            r.extend(waypoints);
            r.push((cx2, cy2));
            r
        } else {
            route_around_nodes(cx1, cy1, cx2, cy2, &edge.from, &edge.to, &all_bounds, offset)
        };

        let start_target = if route.len() > 1 {
            route[1]
        } else {
            (cx2, cy2)
        };
        let end_target = if route.len() > 1 {
            route[route.len() - 2]
        } else {
            (cx1, cy1)
        };

        // Clip start/end to node boundaries (circle for process, rect for others)
        let from_node = diagram.nodes.iter().find(|n| n.label == edge.from);
        let to_node = diagram.nodes.iter().find(|n| n.label == edge.to);

        let (ax1, ay1) = clip_to_node_boundary(
            cx1, cy1, start_target.0, start_target.1, from_node, fw, fh,
        );
        let (ax2, ay2) = clip_to_node_boundary(
            cx2, cy2, end_target.0, end_target.1, to_node, tw, th,
        );

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 {
            clipped.extend_from_slice(&route[1..route.len() - 1]);
        }
        clipped.push((ax2, ay2));

        let path_d = if clipped.len() == 2 {
            format!(
                "M{},{} L{},{}",
                clipped[0].0, clipped[0].1, clipped[1].0, clipped[1].1
            )
        } else {
            build_smooth_path(&clipped)
        };

        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            path_d, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            let (mx, my) = midpoint_on_path(&clipped);
            let lw = text_width(&edge.label);
            // Offset label perpendicular to the edge direction to avoid overlapping nodes
            let dx = clipped.last().unwrap().0 - clipped[0].0;
            let dy = clipped.last().unwrap().1 - clipped[0].1;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let offset_x = -dy / len * 28.0;
            let offset_y = dx / len * 28.0;
            let mut lx = mx + offset_x;
            let mut ly = my + offset_y;

            // Push label away from any overlapping circle
            for node in &diagram.nodes {
                if !matches!(node.kind, NodeKind::Process) { continue; }
                if let Some(&(nx, ny, nw, _nh)) = positions.get(&node.label) {
                    let ncx = nx + nw / 2.0;
                    let ncy = ny + nw / 2.0; // circle: w == h
                    let nr = nw / 2.0;
                    let dist = ((lx - ncx).powi(2) + (ly - ncy).powi(2)).sqrt();
                    if dist < nr + 20.0 {
                        let push_dx = lx - ncx;
                        let push_dy = ly - ncy;
                        let push_dist = (push_dx * push_dx + push_dy * push_dy).sqrt().max(1.0);
                        let push = nr + 24.0 - dist;
                        lx += push_dx / push_dist * push;
                        ly += push_dy / push_dist * push;
                    }
                }
            }
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.85\"/>",
                lx - lw / 2.0 - 3.0,
                ly - 12.0,
                lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                lx,
                ly,
                COLOR_EDGE,
                escape_xml(&edge.label)
            ));
        }

        // Suppress unused variable warning
        let _ = edge_key;
    }

    svg.push_str("</svg>");
    svg
}

/// Clip from center to target, handling circular process nodes vs rectangular nodes.
fn clip_to_node_boundary(
    cx: f64,
    cy: f64,
    tx: f64,
    ty: f64,
    node: Option<&Node>,
    w: f64,
    h: f64,
) -> (f64, f64) {
    let is_circle = node
        .map(|n| matches!(n.kind, NodeKind::Process))
        .unwrap_or(false);

    if is_circle {
        let r = w / 2.0;
        let dx = tx - cx;
        let dy = ty - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1.0 {
            // Target is at center; push straight down
            (cx, cy + r)
        } else {
            (cx + dx / dist * r, cy + dy / dist * r)
        }
    } else {
        clip_to_rect(cx, cy, tx, ty, w / 2.0, h / 2.0)
    }
}

fn render_elements_recursive(
    svg: &mut String,
    elements: &[Element],
    nodes: &[Node],
    boundaries: &[Boundary],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        match elem {
            Element::BoundaryRef(bi) => {
                let boundary = &boundaries[*bi];
                if let Some(&(x, y, w, h)) = positions.get(&boundary.label) {
                    // Boundary background (dashed red border for trust boundaries)
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"8,4\"/>",
                        x, y, w, h, COLOR_BOUNDARY_FILL, COLOR_BOUNDARY_STROKE
                    ));
                    // Boundary label
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" fill=\"{}\">{}</text>",
                        x + 8.0,
                        y + 20.0,
                        COLOR_BOUNDARY_STROKE,
                        escape_xml(&boundary.label)
                    ));
                }
                // Recurse into children
                render_elements_recursive(
                    svg,
                    &boundary.children,
                    nodes,
                    boundaries,
                    positions,
                );
            }
            Element::NodeRef(ni) => {
                let node = &nodes[*ni];
                if let Some(&(x, y, _w, _h)) = positions.get(&node.label) {
                    match &node.kind {
                        NodeKind::Entity => render_entity(svg, x, y, &node.label),
                        NodeKind::Process => render_process(svg, x, y, &node.label),
                        NodeKind::DataStore { columns } => {
                            render_datastore(svg, x, y, &node.label, columns)
                        }
                    }
                }
            }
        }
    }
}

fn render_entity(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = entity_size(label);
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, COLOR_ENTITY_FILL, COLOR_ENTITY_STROKE
    ));

    let lines = wrap_lines(label);
    let total_h = lines.len() as f64 * LINE_HEIGHT;
    let start_y = y + (h - total_h) / 2.0 + LINE_HEIGHT * 0.75;
    let cx = x + w / 2.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            cx,
            start_y + i as f64 * LINE_HEIGHT,
            escape_xml(line)
        ));
    }
}

fn render_process(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, _h) = process_size(label);
    let r = w / 2.0;
    let cx = x + r;
    let cy = y + r;

    svg.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, cy, r, COLOR_PROCESS_FILL, COLOR_PROCESS_STROKE
    ));

    let lines = wrap_lines(label);
    let total_h = lines.len() as f64 * LINE_HEIGHT;
    let start_y = cy - total_h / 2.0 + LINE_HEIGHT * 0.75;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            cx,
            start_y + i as f64 * LINE_HEIGHT,
            escape_xml(line)
        ));
    }
}

fn render_datastore(svg: &mut String, x: f64, y: f64, label: &str, columns: &[String]) {
    let (w, h) = datastore_size(label, columns);

    // Background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        x, y, w, h, COLOR_DS_FILL
    ));

    // Top and bottom lines
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x,
        y,
        x + w,
        y,
        COLOR_DS_STROKE
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x,
        y + h,
        x + w,
        y + h,
        COLOR_DS_STROKE
    ));

    // Header (table name)
    let cx = x + w / 2.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        cx,
        y + DS_HEADER_H * 0.75,
        escape_xml(label)
    ));

    // Separator line under header
    if !columns.is_empty() {
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"3,3\"/>",
            x, y + DS_HEADER_H, x + w, y + DS_HEADER_H, COLOR_DS_STROKE
        ));
    }

    // Column names in multi-column grid
    let (num_cols, col_widths, num_rows) = ds_column_layout(columns);
    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let grid_start_x = x + (w - inner_w) / 2.0;

    for (i, col) in columns.iter().enumerate() {
        let display_col = i / num_rows;
        let display_row = i % num_rows;

        let col_x: f64 =
            col_widths[..display_col].iter().sum::<f64>() + display_col as f64 * DS_COL_GAP;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
            grid_start_x + col_x,
            y + DS_HEADER_H + (display_row as f64 + 0.75) * LINE_HEIGHT,
            "#555",
            escape_xml(col)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-dfd - Render a data flow diagram as SVG

Usage: mdd-dfd < input.dfd

Define entities (rectangles), processes (circles), and datastores
(open-ended rectangles with optional columns). Connect them with
\"->\" edges, optionally labelled with \" : <label>\".
Use \"boundary\" blocks to group nodes into trust boundaries.

Example:
  boundary \"Trust Boundary\" {
    process HandleRequest
    datastore UserDB {
      id
      name
    }
  }
  entity ExternalUser
  flow ExternalUser -> HandleRequest : \"HTTP Request\"
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
            eprintln!("mdd-dfd: {}", e);
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
    fn parse_entity() {
        let d = parse("entity Customer\n").unwrap();
        assert_eq!(d.nodes.len(), 1);
        assert!(matches!(d.nodes[0].kind, NodeKind::Entity));
        assert_eq!(d.nodes[0].label, "Customer");
    }

    #[test]
    fn parse_process() {
        let d = parse("process HandleOrder\n").unwrap();
        assert_eq!(d.nodes.len(), 1);
        assert!(matches!(d.nodes[0].kind, NodeKind::Process));
    }

    #[test]
    fn parse_datastore() {
        let input = "datastore Orders {\n  注文ID\n  顧客ID\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.nodes.len(), 1);
        match &d.nodes[0].kind {
            NodeKind::DataStore { columns } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns[0], "注文ID");
            }
            _ => panic!("Expected DataStore"),
        }
    }

    #[test]
    fn parse_edge_with_label() {
        let input = "entity A\nprocess B\nA -> B : \"data\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "data");
    }

    #[test]
    fn parse_edge_without_label() {
        let input = "entity A\nprocess B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "");
    }

    #[test]
    fn parse_boundary() {
        let input = "boundary \"Trust Boundary\" {\n  process HandleRequest\n  entity InternalService\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.boundaries.len(), 1);
        assert_eq!(d.boundaries[0].label, "Trust Boundary");
        assert_eq!(d.boundaries[0].children.len(), 2);
        assert_eq!(d.top_level.len(), 1);
        assert!(matches!(d.top_level[0], Element::BoundaryRef(0)));
    }

    #[test]
    fn parse_boundary_with_datastore() {
        let input = "boundary \"Internal\" {\n  process Handler\n  datastore UserDB {\n    id\n    name\n  }\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.boundaries.len(), 1);
        assert_eq!(d.boundaries[0].children.len(), 2);
        assert_eq!(d.nodes.len(), 2);
    }

    #[test]
    fn parse_flow_keyword() {
        let input = "entity A\nprocess B\nflow A -> B : \"request\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "request");
    }

    #[test]
    fn render_produces_svg() {
        let input = "entity A\nprocess B\nA -> B : \"test\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("marker"));
    }

    #[test]
    fn render_with_boundary() {
        let input = "boundary \"Trust Boundary\" {\n  process HandleRequest\n}\nentity ExternalUser\nExternalUser -> HandleRequest : \"HTTP\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Trust Boundary"));
        assert!(svg.contains("stroke-dasharray"));
    }
}
