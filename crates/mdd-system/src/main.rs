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
    /// Rectangle (default): process, service, component
    Process,
    /// External entity / actor (double-bordered rect)
    Entity,
    /// Database / datastore (cylinder-like)
    DataStore { columns: Vec<String> },
    /// Actor (stick figure)
    Actor,
    /// File / document (dog-ear rect)
    File { columns: Vec<String> },
    /// Message queue (parallelogram)
    Queue,
    /// Non-persistent data structure (dashed cylinder)
    Data { columns: Vec<String> },
}

impl NodeKind {
    fn keyword(&self) -> &'static str {
        match self {
            NodeKind::Process => "process",
            NodeKind::Entity => "entity",
            NodeKind::DataStore { .. } => "datastore",
            NodeKind::Actor => "actor",
            NodeKind::File { .. } => "file",
            NodeKind::Queue => "queue",
            NodeKind::Data { .. } => "data",
        }
    }
}

#[derive(Debug)]
struct Node {
    label: String,
    kind: NodeKind,
}

#[derive(Debug)]
struct Group {
    label: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    NodeRef(usize),
    GroupRef(usize),
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
    groups: Vec<Group>,
    top_level: Vec<Element>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut top_level: Vec<Element> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    // Block state for datastore/file/data { ... } multi-line definitions
    let mut in_block: Option<&str> = None; // "datastore", "file", or "data"
    let mut block_name = String::new();
    let mut block_columns: Vec<String> = Vec::new();

    // Edge-data block state: A -> B : data Name { ... }
    let mut in_edge_data: Option<(String, String)> = None; // (from, to)
    let mut edge_data_name = String::new();
    let mut edge_data_columns: Vec<String> = Vec::new();

    let mut group_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    let simple_kinds: &[(&str, fn() -> NodeKind)] = &[
        ("process ", || NodeKind::Process),
        ("entity ", || NodeKind::Entity),
        ("actor ", || NodeKind::Actor),
        ("queue ", || NodeKind::Queue),
    ];

    // Block-capable kinds: keyword → constructor taking columns
    let block_kinds: &[(&str, fn(Vec<String>) -> NodeKind)] = &[
        ("datastore ", |cols| NodeKind::DataStore { columns: cols }),
        ("file ", |cols| NodeKind::File { columns: cols }),
        ("data ", |cols| NodeKind::Data { columns: cols }),
    ];

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Inside a block (datastore/file/cache)
        if let Some(block_type) = in_block {
            if line == "}" {
                let make_kind: fn(Vec<String>) -> NodeKind = match block_type {
                    "datastore" => |cols| NodeKind::DataStore { columns: cols },
                    "file" => |cols| NodeKind::File { columns: cols },
                    "data" => |cols| NodeKind::Data { columns: cols },
                    _ => unreachable!(),
                };
                let id = nodes.len();
                name_to_id.insert(block_name.clone(), id);
                nodes.push(Node { label: block_name.clone(), kind: make_kind(block_columns.clone()) });
                let elem = Element::NodeRef(id);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
                in_block = None;
                block_name.clear();
                block_columns.clear();
                continue;
            }
            block_columns.push(line.to_string());
            continue;
        }

        // Inside edge-data block: A -> B : data Name { columns... }
        if let Some((ref from, ref to)) = in_edge_data {
            if line == "}" {
                let data_label = edge_data_name.clone();
                let from = from.clone();
                let to = to.clone();

                // Create data node
                let id = nodes.len();
                name_to_id.insert(data_label.clone(), id);
                nodes.push(Node { label: data_label.clone(), kind: NodeKind::Data { columns: edge_data_columns.clone() } });
                top_level.push(Element::NodeRef(id));

                // Create two edges: from -> data, data -> to
                edges.push(Edge { from, to: data_label.clone(), label: String::new() });
                edges.push(Edge { from: data_label, to, label: String::new() });

                in_edge_data = None;
                edge_data_name.clear();
                edge_data_columns.clear();
                continue;
            }
            edge_data_columns.push(line.to_string());
            continue;
        }

        // Close group
        if line == "}" {
            if let Some((gidx, children)) = group_stack.pop() {
                groups[gidx].children = children;
                let elem = Element::GroupRef(gidx);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
            } else {
                return Err("Unexpected }".to_string());
            }
            continue;
        }

        // group "Name" {
        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let gidx = groups.len();
                groups.push(Group { label: name, children: Vec::new() });
                group_stack.push((gidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        // Block-capable kinds: datastore/file/cache Name { ... } or single-line
        let mut block_matched = false;
        for (prefix, make_kind) in block_kinds {
            if line.starts_with(prefix) {
                let rest = line.strip_prefix(prefix).unwrap();
                if let Some(name) = rest.strip_suffix(" {") {
                    block_name = name.trim().to_string();
                    block_columns.clear();
                    in_block = Some(prefix.trim());
                    block_matched = true;
                    break;
                }
                // Single-line (no columns)
                let label = rest.trim().to_string();
                let id = nodes.len();
                name_to_id.insert(label.clone(), id);
                nodes.push(Node { label, kind: make_kind(Vec::new()) });
                let elem = Element::NodeRef(id);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
                block_matched = true;
                break;
            }
        }
        if block_matched { continue; }

        // Simple node kinds
        let mut matched = false;
        for (prefix, make_kind) in simple_kinds {
            if line.starts_with(prefix) {
                let label = line.strip_prefix(prefix).unwrap().trim().to_string();
                let id = nodes.len();
                name_to_id.insert(label.clone(), id);
                nodes.push(Node { label, kind: make_kind() });
                let elem = Element::NodeRef(id);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
                matched = true;
                break;
            }
        }
        if matched { continue; }

        // Edge
        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            if parts.len() < 2 {
                return Err(format!("Invalid edge syntax: {}", line));
            }
            let from = parts[0].trim().to_string();
            let rest = parts[1];
            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (to_part.trim().to_string(), label_part.trim().to_string())
            } else {
                (rest.trim().to_string(), String::new())
            };

            if !name_to_id.contains_key(&from) {
                return Err(format!("Unknown node: {}", from));
            }
            if !name_to_id.contains_key(&to) {
                return Err(format!("Unknown node: {}", to));
            }

            // Check for inline data: A -> B : data Name { ... } or A -> B : data "Name"
            if label.starts_with("data ") {
                let data_rest = label.strip_prefix("data ").unwrap().trim();

                // data Name { (multi-line block)
                if let Some(name) = data_rest.strip_suffix(" {") {
                    edge_data_name = name.trim().trim_matches('"').to_string();
                    edge_data_columns.clear();
                    in_edge_data = Some((from, to));
                    continue;
                }

                // data "Name" (single-line, no columns)
                let data_label = data_rest.trim_matches('"').to_string();
                let id = nodes.len();
                name_to_id.insert(data_label.clone(), id);
                nodes.push(Node { label: data_label.clone(), kind: NodeKind::Data { columns: Vec::new() } });
                top_level.push(Element::NodeRef(id));
                edges.push(Edge { from: from.clone(), to: data_label.clone(), label: String::new() });
                edges.push(Edge { from: data_label, to, label: String::new() });
                continue;
            }

            let label = label.trim_matches('"').to_string();
            edges.push(Edge { from, to, label });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_block.is_some() {
        return Err(format!("Unclosed block: {}", block_name));
    }
    if in_edge_data.is_some() {
        return Err(format!("Unclosed edge data block: {}", edge_data_name));
    }
    if !group_stack.is_empty() {
        return Err("Unclosed group block".to_string());
    }

    Ok(Diagram { nodes, groups, top_level, edges })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;
const MAX_LINE_CHARS: usize = 16;

const NODE_H_PAD: f64 = 20.0;
const NODE_V_PAD: f64 = 12.0;
const NODE_MIN_W: f64 = 120.0;
const NODE_MIN_H: f64 = 40.0;

const ACTOR_W: f64 = 60.0;
const ACTOR_H: f64 = 80.0;

const DS_H_PAD: f64 = 16.0;
const DS_HEADER_H: f64 = 24.0;
const DS_MIN_W: f64 = 140.0;
const DS_COL_GAP: f64 = 12.0;
const DS_MAX_ROWS: usize = 8;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";

// Per-kind colors: (fill, stroke)
const COLOR_PROCESS: (&str, &str) = ("#f0f8ff", "#336699");
const COLOR_ENTITY: (&str, &str) = ("#fff5ee", "#996633");
const COLOR_DATASTORE: (&str, &str) = ("#f0fff0", "#339966");
const COLOR_ACTOR: (&str, &str) = ("#fff", "#333");
const COLOR_FILE: (&str, &str) = ("#fffde7", "#f9a825");
const COLOR_QUEUE: (&str, &str) = ("#f3e5f5", "#7b1fa2");
const COLOR_DATA: (&str, &str) = ("#e0f2f1", "#00897b");
const COLOR_GROUP: (&str, &str) = ("#fff8f8", "#cc3333");

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn wrap_lines(label: &str) -> Vec<String> {
    let words: Vec<String> = if label.contains(' ') {
        label.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        let mut w = Vec::new();
        let mut cur = String::new();
        for ch in label.chars() {
            if ch.is_uppercase() && !cur.is_empty() { w.push(cur); cur = String::new(); }
            cur.push(ch);
        }
        if !cur.is_empty() { w.push(cur); }
        w
    };

    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in words {
        if cur.is_empty() {
            cur = word;
        } else if cur.len() + 1 + word.len() <= MAX_LINE_CHARS {
            cur.push(' '); cur.push_str(&word);
        } else {
            lines.push(cur); cur = word;
        }
    }
    if !cur.is_empty() { lines.push(cur); }
    lines
}

// ---------------------------------------------------------------------------
// Node sizing
// ---------------------------------------------------------------------------

fn rect_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f64, f64::max);
    let w = (max_w + NODE_H_PAD * 2.0).max(NODE_MIN_W);
    let h = (lines.len() as f64 * LINE_HEIGHT + NODE_V_PAD * 2.0).max(NODE_MIN_H);
    (w, h)
}

fn ds_column_layout(columns: &[String]) -> (usize, Vec<f64>, usize) {
    if columns.is_empty() { return (1, vec![0.0], 0); }
    let num_cols = ((columns.len() + DS_MAX_ROWS - 1) / DS_MAX_ROWS).max(1);
    let num_rows = (columns.len() + num_cols - 1) / num_cols;
    let mut col_widths = vec![0.0_f64; num_cols];
    for (i, col) in columns.iter().enumerate() {
        let c = i / num_rows;
        col_widths[c] = col_widths[c].max(text_width(col));
    }
    (num_cols, col_widths, num_rows)
}

const CYLINDER_RY: f64 = 10.0; // ellipse vertical radius for cylinder caps

fn datastore_size(label: &str, columns: &[String]) -> (f64, f64) {
    let header_w = text_width(label) + DS_H_PAD * 2.0;
    let (num_cols, col_widths, num_rows) = ds_column_layout(columns);
    let inner_w: f64 = col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let w = header_w.max(inner_w + DS_H_PAD * 2.0).max(DS_MIN_W);
    let body_h = DS_HEADER_H + num_rows as f64 * LINE_HEIGHT + 8.0;
    let h = body_h + CYLINDER_RY * 2.0; // top + bottom ellipse space
    (w, h)
}

fn process_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f64, f64::max);
    let text_h = lines.len() as f64 * LINE_HEIGHT;
    // Circle: diameter = max of text width/height + padding
    let d = ((max_w + 24.0).max(text_h + 24.0)).max(70.0);
    (d, d)
}

fn queue_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f64, f64::max);
    // Horizontal cylinder: body + ellipse caps + stacking offset
    let w = (max_w + 40.0 + 24.0).max(NODE_MIN_W + 24.0); // body + right ellipse + stack offset
    let h = (lines.len() as f64 * LINE_HEIGHT + NODE_V_PAD * 2.0).max(50.0);
    (w, h)
}

fn columned_size(label: &str, columns: &[String], colors: (&str, &str)) -> (f64, f64) {
    if columns.is_empty() {
        return rect_size(label);
    }
    datastore_size(label, columns)
}

fn node_size(node: &Node) -> (f64, f64) {
    match &node.kind {
        NodeKind::Process => process_size(&node.label),
        NodeKind::Entity => rect_size(&node.label),
        NodeKind::DataStore { columns } => datastore_size(&node.label, columns),
        NodeKind::File { columns } => columned_size(&node.label, columns, COLOR_FILE),
        NodeKind::Data { columns } => columned_size(&node.label, columns, COLOR_DATA),
        NodeKind::Actor => (ACTOR_W, ACTOR_H),
        NodeKind::Queue => queue_size(&node.label),
    }
}

fn is_circle(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Process)
}

// ---------------------------------------------------------------------------
// Build LayoutGraph
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> LayoutGraph {
    let mut graph = LayoutGraph::new();

    for node in &diagram.nodes {
        let (w, h) = node_size(node);
        graph.nodes.push(LayoutNode { name: node.label.clone(), width: w, height: h });
    }

    for edge in &diagram.edges {
        graph.edges.push(LayoutEdge { from: edge.from.clone(), to: edge.to.clone(), label: edge.label.clone() });
    }

    for group in &diagram.groups {
        graph.groups.push(LayoutGroup {
            name: group.label.clone(),
            children: convert_elements(&group.children),
        });
    }

    graph.top_level = convert_elements(&diagram.top_level);
    graph
}

fn convert_elements(elements: &[Element]) -> Vec<LayoutElement> {
    elements.iter().map(|e| match e {
        Element::NodeRef(i) => LayoutElement::NodeRef(*i),
        Element::GroupRef(i) => LayoutElement::GroupRef(*i),
    }).collect()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);

    let config = LayoutConfig {
        padding: PADDING,
        group_h_pad: 40.0,
        group_v_pad: 40.0,
        group_header_h: 28.0,
        ..LayoutConfig::default()
    };
    let result = mdd_layout::layout(&graph, &config);
    let positions = &result.positions;
    let edge_waypoints = &result.edge_waypoints;

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
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render groups and nodes
    render_elements(&mut svg, &diagram.top_level, &diagram.nodes, &diagram.groups, positions);

    // Build bounds for edge routing
    let all_bounds: Vec<(String, f64, f64, f64, f64)> = diagram.nodes.iter()
        .filter_map(|n| positions.get(&n.label).map(|(x, y, w, h)| {
            (n.label.clone(), x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)
        }))
        .collect();

    // Reciprocal edge counting
    let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
    for e in &diagram.edges {
        let key = if e.from <= e.to { (e.from.clone(), e.to.clone()) } else { (e.to.clone(), e.from.clone()) };
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();

    // Render edges
    for edge in &diagram.edges {
        let from_pos = positions.get(&edge.from);
        let to_pos = positions.get(&edge.to);
        if from_pos.is_none() || to_pos.is_none() { continue; }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();
        let cx1 = fx + fw / 2.0; let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0; let cy2 = ty + th / 2.0;

        let pair_key = if edge.from <= edge.to { (edge.from.clone(), edge.to.clone()) } else { (edge.to.clone(), edge.from.clone()) };
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = { let seen = pair_seen.entry(pair_key).or_insert(0); let v = *seen; *seen += 1; v };
        let offset = if total > 1 { (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0 } else { 0.0 };

        let layout_edge_key = format!("{}→{}", edge.from, edge.to);
        let route = if let Some(waypoints) = edge_waypoints.get(&layout_edge_key) {
            let mut r = vec![(cx1, cy1)]; r.extend(waypoints); r.push((cx2, cy2)); r
        } else {
            route_around_nodes(cx1, cy1, cx2, cy2, &edge.from, &edge.to, &all_bounds, offset)
        };

        let start_t = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_t = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };

        let from_node = diagram.nodes.iter().find(|n| n.label == edge.from);
        let to_node = diagram.nodes.iter().find(|n| n.label == edge.to);
        let (ax1, ay1) = clip_node(cx1, cy1, start_t.0, start_t.1, fw, fh, from_node);
        let (ax2, ay2) = clip_node(cx2, cy2, end_t.0, end_t.1, tw, th, to_node);

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 { clipped.extend_from_slice(&route[1..route.len() - 1]); }
        clipped.push((ax2, ay2));

        let path_d = if clipped.len() == 2 {
            format!("M{},{} L{},{}", clipped[0].0, clipped[0].1, clipped[1].0, clipped[1].1)
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
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.85\"/>",
                mx - lw / 2.0 - 3.0, my - 12.0, lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                mx, my, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn clip_node(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64, node: Option<&Node>) -> (f64, f64) {
    let circ = node.map(|n| is_circle(&n.kind)).unwrap_or(false);
    if circ {
        let r = w / 2.0;
        let dx = tx - cx; let dy = ty - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1.0 { (cx, cy + r) } else { (cx + dx / dist * r, cy + dy / dist * r) }
    } else {
        clip_to_rect(cx, cy, tx, ty, w / 2.0, h / 2.0)
    }
}

fn render_elements(svg: &mut String, elements: &[Element], nodes: &[Node], groups: &[Group], positions: &HashMap<String, (f64, f64, f64, f64)>) {
    for elem in elements {
        match elem {
            Element::GroupRef(gi) => {
                let group = &groups[*gi];
                if let Some(&(x, y, w, h)) = positions.get(&group.label) {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"8,4\"/>",
                        x, y, w, h, COLOR_GROUP.0, COLOR_GROUP.1
                    ));
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" fill=\"{}\">{}</text>",
                        x + 8.0, y + 20.0, COLOR_GROUP.1, escape_xml(&group.label)
                    ));
                }
                render_elements(svg, &group.children, nodes, groups, positions);
            }
            Element::NodeRef(ni) => {
                let node = &nodes[*ni];
                if let Some(&(x, y, _w, _h)) = positions.get(&node.label) {
                    render_node(svg, node, x, y);
                }
            }
        }
    }
}

fn render_node(svg: &mut String, node: &Node, x: f64, y: f64) {
    match &node.kind {
        NodeKind::Process => render_process(svg, x, y, &node.label),
        NodeKind::Entity => render_entity(svg, x, y, &node.label),
        NodeKind::DataStore { columns } => render_datastore(svg, x, y, &node.label, columns),
        NodeKind::Actor => render_actor(svg, x, y, &node.label),
        NodeKind::File { columns } => render_file(svg, x, y, &node.label, columns),
        NodeKind::Queue => render_queue(svg, x, y, &node.label),
        NodeKind::Data { columns } => render_data(svg, x, y, &node.label, columns),
    }
}

fn render_process(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = process_size(label);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let r = w / 2.0;

    // Circle
    svg.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx, cy, r, COLOR_PROCESS.0, COLOR_PROCESS.1
    ));

    // Circular arrow around the top-right
    let ar = r + 6.0; // arrow arc radius (just outside circle)
    let a1 = -std::f64::consts::FRAC_PI_4; // start at -45deg (top-right)
    let a2 = std::f64::consts::FRAC_PI_2 + std::f64::consts::FRAC_PI_4; // end at 135deg
    let sx = cx + ar * a1.cos();
    let sy = cy + ar * a1.sin();
    let ex = cx + ar * a2.cos();
    let ey = cy + ar * a2.sin();
    svg.push_str(&format!(
        "<path d=\"M{},{} A{},{} 0 1 1 {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
        sx, sy, ar, ar, ex, ey, COLOR_PROCESS.1
    ));

    // Text
    let lines = wrap_lines(label);
    let total_h = lines.len() as f64 * LINE_HEIGHT;
    let start_y = cy - total_h / 2.0 + LINE_HEIGHT * 0.75;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            cx, start_y + i as f64 * LINE_HEIGHT, escape_xml(line)
        ));
    }
}

fn render_entity(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = rect_size(label);
    // Double border for external entity
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, COLOR_ENTITY.0, COLOR_ENTITY.1
    ));
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + 3.0, y + 3.0, w - 6.0, h - 6.0, COLOR_ENTITY.1
    ));
    let lines = wrap_lines(label);
    let total_h = lines.len() as f64 * LINE_HEIGHT;
    let start_y = y + (h - total_h) / 2.0 + LINE_HEIGHT * 0.75;
    let cx = x + w / 2.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            cx, start_y + i as f64 * LINE_HEIGHT, escape_xml(line)
        ));
    }
}

fn render_datastore(svg: &mut String, x: f64, y: f64, label: &str, columns: &[String]) {
    let (w, h) = datastore_size(label, columns);
    let ry = CYLINDER_RY;
    let rx = w / 2.0;
    let cx = x + w / 2.0;

    // Cylinder body (rect between top and bottom ellipses)
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        x, y + ry, w, h - ry * 2.0, COLOR_DATASTORE.0
    ));
    // Left/right sides
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y + ry, x, y + h - ry, COLOR_DATASTORE.1
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x + w, y + ry, x + w, y + h - ry, COLOR_DATASTORE.1
    ));
    // Top ellipse (full)
    svg.push_str(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx, y + ry, rx, ry, COLOR_DATASTORE.0, COLOR_DATASTORE.1
    ));
    // Bottom ellipse (half, front arc only)
    svg.push_str(&format!(
        "<path d=\"M{},{} A{},{} 0 0 0 {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y + h - ry, rx, ry, x + w, y + h - ry, COLOR_DATASTORE.0, COLOR_DATASTORE.1
    ));
    // Second rim line (stacked look)
    svg.push_str(&format!(
        "<path d=\"M{},{} A{},{} 0 0 0 {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.8\" opacity=\"0.4\"/>",
        x, y + ry + 5.0, rx, ry, x + w, y + ry + 5.0, COLOR_DATASTORE.1
    ));

    // Header text
    let header_y = y + ry + DS_HEADER_H * 0.75;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        cx, header_y, escape_xml(label)
    ));

    // Columns
    if !columns.is_empty() {
        let sep_y = y + ry + DS_HEADER_H;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"3,3\"/>",
            x + 4.0, sep_y, x + w - 4.0, sep_y, COLOR_DATASTORE.1
        ));
        let (num_cols, col_widths, num_rows) = ds_column_layout(columns);
        let inner_w: f64 = col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
        let grid_x = x + (w - inner_w) / 2.0;
        for (i, col) in columns.iter().enumerate() {
            let dc = i / num_rows;
            let dr = i % num_rows;
            let col_x: f64 = col_widths[..dc].iter().sum::<f64>() + dc as f64 * DS_COL_GAP;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#555\">{}</text>",
                grid_x + col_x, sep_y + (dr as f64 + 0.75) * LINE_HEIGHT, escape_xml(col)
            ));
        }
    }
}

fn render_actor(svg: &mut String, x: f64, y: f64, label: &str) {
    let cx = x + ACTOR_W / 2.0;
    svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>", cx, y + 12.0, COLOR_DARK));
    svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>", cx, y + 22.0, cx, y + 45.0, COLOR_DARK));
    svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>", cx - 15.0, y + 32.0, cx + 15.0, y + 32.0, COLOR_DARK));
    svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>", cx, y + 45.0, cx - 12.0, y + 60.0, COLOR_DARK));
    svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>", cx, y + 45.0, cx + 12.0, y + 60.0, COLOR_DARK));
    let lines = wrap_lines(label);
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx, y + 75.0 + i as f64 * LINE_HEIGHT, escape_xml(line)
        ));
    }
}

fn render_file(svg: &mut String, x: f64, y: f64, label: &str, columns: &[String]) {
    let (w, h) = columned_size(label, columns, COLOR_FILE);
    let ear = 12.0;
    // Dog-ear rectangle
    let d = format!(
        "M{},{} L{},{} L{},{} L{},{} L{},{} Z",
        x, y, x + w - ear, y, x + w, y + ear, x + w, y + h, x, y + h,
    );
    svg.push_str(&format!(
        "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        d, COLOR_FILE.0, COLOR_FILE.1
    ));
    svg.push_str(&format!(
        "<path d=\"M{},{} L{},{} L{},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + w - ear, y, x + w - ear, y + ear, x + w, y + ear, COLOR_FILE.1
    ));

    if columns.is_empty() {
        let lines = wrap_lines(label);
        let total_h = lines.len() as f64 * LINE_HEIGHT;
        let start_y = y + (h - total_h) / 2.0 + LINE_HEIGHT * 0.75;
        let cx = x + w / 2.0;
        for (i, line) in lines.iter().enumerate() {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                cx, start_y + i as f64 * LINE_HEIGHT, escape_xml(line)
            ));
        }
    } else {
        render_columned_body(svg, x, y, w, label, columns, COLOR_FILE.1);
    }
}

fn render_queue(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = queue_size(label);
    let erx = 12.0; // ellipse x-radius for cylinder caps
    let stack_offset = 8.0; // offset for stacked look

    // Back cylinders (stacked behind, shifted left)
    for i in (1..=2).rev() {
        let ox = x + stack_offset * i as f64;
        let opacity = if i == 2 { "0.25" } else { "0.45" };
        // Right ellipse of back cylinder
        svg.push_str(&format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" opacity=\"{}\"/>",
            ox + w - stack_offset * 2.0 - erx, y + h / 2.0, erx, h / 2.0, COLOR_QUEUE.0, COLOR_QUEUE.1, opacity
        ));
    }

    // Front cylinder
    let body_x = x;
    let body_w = w - stack_offset * 2.0 - erx * 2.0;
    let cy = y + h / 2.0;

    // Body rect
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        body_x + erx, y, body_w, h, COLOR_QUEUE.0
    ));
    // Top/bottom lines
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        body_x + erx, y, body_x + erx + body_w, y, COLOR_QUEUE.1
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        body_x + erx, y + h, body_x + erx + body_w, y + h, COLOR_QUEUE.1
    ));
    // Left ellipse (full)
    svg.push_str(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        body_x + erx, cy, erx, h / 2.0, COLOR_QUEUE.0, COLOR_QUEUE.1
    ));
    // Right ellipse (full)
    svg.push_str(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        body_x + erx + body_w, cy, erx, h / 2.0, COLOR_QUEUE.0, COLOR_QUEUE.1
    ));

    // Text
    let lines = wrap_lines(label);
    let total_h = lines.len() as f64 * LINE_HEIGHT;
    let start_y = cy - total_h / 2.0 + LINE_HEIGHT * 0.75;
    let text_cx = body_x + erx + body_w / 2.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            text_cx, start_y + i as f64 * LINE_HEIGHT, escape_xml(line)
        ));
    }
}

fn render_data(svg: &mut String, x: f64, y: f64, label: &str, columns: &[String]) {
    let (w, h) = columned_size(label, columns, COLOR_DATA);
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        x, y, w, h, COLOR_DATA.0
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, x + w, y, COLOR_DATA.1
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y + h, x + w, y + h, COLOR_DATA.1
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>",
        x, y, x, y + h, COLOR_DATA.1
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>",
        x + w, y, x + w, y + h, COLOR_DATA.1
    ));

    if columns.is_empty() {
        let lines = wrap_lines(label);
        let total_h = lines.len() as f64 * LINE_HEIGHT;
        let start_y = y + (h - total_h) / 2.0 + LINE_HEIGHT * 0.75;
        let cx = x + w / 2.0;
        for (i, line) in lines.iter().enumerate() {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                cx, start_y + i as f64 * LINE_HEIGHT, escape_xml(line)
            ));
        }
    } else {
        render_columned_body(svg, x, y, w, label, columns, COLOR_DATA.1);
    }
}

/// Shared rendering for header + column list body (used by datastore, file, cache)
fn render_columned_body(svg: &mut String, x: f64, y: f64, w: f64, label: &str, columns: &[String], stroke: &str) {
    let cx = x + w / 2.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        cx, y + DS_HEADER_H * 0.75, escape_xml(label)
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"3,3\"/>",
        x, y + DS_HEADER_H, x + w, y + DS_HEADER_H, stroke
    ));
    let (num_cols, col_widths, num_rows) = ds_column_layout(columns);
    let inner_w: f64 = col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let grid_x = x + (w - inner_w) / 2.0;
    for (i, col) in columns.iter().enumerate() {
        let dc = i / num_rows;
        let dr = i % num_rows;
        let col_x: f64 = col_widths[..dc].iter().sum::<f64>() + dc as f64 * DS_COL_GAP;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#555\">{}</text>",
            grid_x + col_x, y + DS_HEADER_H + (dr as f64 + 0.75) * LINE_HEIGHT, escape_xml(col)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-system - Render a system diagram as SVG

Usage: mdd-system < input.txt

Node types:
  process  Name          Rounded rectangle (service, component)
  entity   Name          Double-bordered rectangle (external system)
  datastore Name { ... } Cylinder (database, with optional columns)
  actor    Name          Stick figure (person, role)
  file     Name          Dog-ear rectangle (document, file)
  queue    Name          Parallelogram (message queue, event bus)
  data     Name          Dashed cylinder (non-persistent data structure)

Edges:
  From -> To : \"label\"
  From -> To : data \"Name\"          (inline data node, no columns)
  From -> To : data Name {           (inline data node with columns)
    field1
    field2
  }

Groups:
  group \"Name\" { ... }

Example:
  actor User
  process WebApp
  queue EventBus
  datastore UserDB {
    id
    name
    email
  }

  User -> WebApp : \"HTTP\"
  WebApp -> EventBus : \"publish\"
  WebApp -> UserDB : \"query\"
  WebApp -> EventBus : data Payload {
    user_id
    action
  }
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => { eprintln!("mdd-system: {}", e); std::process::exit(1); }
    };

    print!("{}", render_svg(&diagram));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_kinds() {
        let input = "process A\nentity B\nactor C\nfile D\nqueue E\ndatastore F\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.nodes.len(), 6);
        assert_eq!(d.edges.len(), 1);
    }

    #[test]
    fn parse_datastore_with_columns() {
        let input = "datastore DB {\n  id\n  name\n}\n";
        let d = parse(input).unwrap();
        match &d.nodes[0].kind {
            NodeKind::DataStore { columns } => assert_eq!(columns.len(), 2),
            _ => panic!("Expected DataStore"),
        }
    }

    #[test]
    fn parse_group() {
        let input = "group \"Backend\" {\n  process API\n  datastore DB\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].label, "Backend");
    }

    #[test]
    fn parse_edge_with_label() {
        let input = "process A\nprocess B\nA -> B : \"data\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "data");
    }

    #[test]
    fn render_produces_svg() {
        let input = "process A\nqueue B\nA -> B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn parse_inline_data_simple() {
        let input = "process A\nprocess B\nA -> B : data \"中間結果\"\n";
        let d = parse(input).unwrap();
        // Should create: A, B, 中間結果 (3 nodes), A->中間結果, 中間結果->B (2 edges)
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
        assert!(matches!(d.nodes[2].kind, NodeKind::Data { .. }));
        assert_eq!(d.nodes[2].label, "中間結果");
        assert_eq!(d.edges[0].from, "A");
        assert_eq!(d.edges[0].to, "中間結果");
        assert_eq!(d.edges[1].from, "中間結果");
        assert_eq!(d.edges[1].to, "B");
    }

    #[test]
    fn parse_inline_data_with_columns() {
        let input = "process A\nprocess B\nA -> B : data Payload {\n  field1\n  field2\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
        match &d.nodes[2].kind {
            NodeKind::Data { columns } => assert_eq!(columns.len(), 2),
            _ => panic!("Expected Data"),
        }
    }
}
