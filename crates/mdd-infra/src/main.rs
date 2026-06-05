use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Server,
    Db,
    Lb,
    Cache,
    Queue,
    Storage,
    Cdn,
    Network,
    User,
    Generic,
}

impl NodeType {
    fn from_str(s: &str) -> Self {
        match s {
            "server" => NodeType::Server,
            "db" | "database" => NodeType::Db,
            "lb" | "loadbalancer" => NodeType::Lb,
            "cache" => NodeType::Cache,
            "queue" => NodeType::Queue,
            "storage" => NodeType::Storage,
            "cdn" => NodeType::Cdn,
            "network" | "vpc" | "subnet" => NodeType::Network,
            "user" | "client" => NodeType::User,
            _ => NodeType::Generic,
        }
    }
}

#[derive(Debug)]
struct Node {
    name: String,
    node_type: NodeType,
}

#[derive(Debug)]
struct Group {
    name: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    NodeRef(usize),   // index into flat nodes vec
    GroupRef(usize),  // index into flat groups vec
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
    let mut edges: Vec<Edge> = Vec::new();
    let mut name_to_node: HashMap<String, usize> = HashMap::new();
    let mut name_to_group: HashMap<String, usize> = HashMap::new();

    // Stack for nested groups: (group_index, children_so_far)
    let mut group_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

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

        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let gidx = groups.len();
                name_to_group.insert(name.clone(), gidx);
                groups.push(Group {
                    name,
                    children: Vec::new(),
                });
                group_stack.push((gidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        if line.starts_with("node ") {
            let rest = line.strip_prefix("node ").unwrap();
            let (name, node_type) = if let Some((name_part, type_part)) = rest.split_once(" type=") {
                (name_part.trim().to_string(), NodeType::from_str(type_part.trim()))
            } else {
                (rest.trim().to_string(), NodeType::Generic)
            };

            let nidx = nodes.len();
            name_to_node.insert(name.clone(), nidx);
            nodes.push(Node { name, node_type });

            let elem = Element::NodeRef(nidx);
            if let Some(parent) = group_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from = parts[0].trim().trim_matches('"').to_string();
            let rest = parts[1];
            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim().trim_matches('"').to_string(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().trim_matches('"').to_string(), String::new())
            };
            edges.push(Edge { from, to, label });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if !group_stack.is_empty() {
        return Err("Unclosed group block".to_string());
    }

    Ok(Diagram {
        nodes,
        groups,
        top_level,
        edges,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const PADDING: f64 = 40.0;

const NODE_W: f64 = 100.0;
const NODE_H: f64 = 70.0;
const ICON_SIZE: f64 = 32.0;

const GROUP_H_PAD: f64 = 16.0;
const GROUP_V_PAD: f64 = 12.0;
const GROUP_HEADER_H: f64 = 28.0;
const GROUP_INNER_GAP: f64 = 16.0;
const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_GROUP_FILL: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#bbb";

// Node type colors
fn node_colors(nt: &NodeType) -> (&'static str, &'static str) {
    match nt {
        NodeType::Server => ("#e3f2fd", "#1565c0"),
        NodeType::Db => ("#e8f5e9", "#2e7d32"),
        NodeType::Lb => ("#fff3e0", "#e65100"),
        NodeType::Cache => ("#fce4ec", "#c62828"),
        NodeType::Queue => ("#f3e5f5", "#6a1b9a"),
        NodeType::Storage => ("#e0f2f1", "#00695c"),
        NodeType::Cdn => ("#fff8e1", "#f9a825"),
        NodeType::Network => ("#e8eaf6", "#283593"),
        NodeType::User => ("#fafafa", "#616161"),
        NodeType::Generic => ("#f5f5f5", "#757575"),
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
// Sizing & Layout (bottom-up Sugiyama)
// ---------------------------------------------------------------------------

/// Get the name of an element (node or group)
fn element_name(elem: &Element, nodes: &[Node], groups: &[Group]) -> String {
    match elem {
        Element::NodeRef(i) => nodes[*i].name.clone(),
        Element::GroupRef(i) => groups[*i].name.clone(),
    }
}

/// Collect all node/group names reachable from an element (recursively for groups)
fn element_all_names(elem: &Element, nodes: &[Node], groups: &[Group]) -> Vec<String> {
    match elem {
        Element::NodeRef(i) => vec![nodes[*i].name.clone()],
        Element::GroupRef(i) => {
            let mut names = vec![groups[*i].name.clone()];
            for child in &groups[*i].children {
                names.extend(element_all_names(child, nodes, groups));
            }
            names
        }
    }
}

/// Recursively layout elements using Sugiyama, bottom-up.
/// First layout children of each group, compute their sizes,
/// then layout this level's elements using edges between them.
fn layout_elements(
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    edges: &[Edge],
    start_x: f64,
    start_y: f64,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
) -> (f64, f64) {
    if elements.is_empty() {
        return (0.0, 0.0);
    }

    // Step 1: Recursively layout children of each group (bottom-up)
    // and compute the size of each element at this level.
    let mut elem_sizes: Vec<(f64, f64)> = Vec::new();
    // Temporarily store group internal sizes for later positioning
    let mut group_internal: HashMap<usize, (f64, f64)> = HashMap::new();

    for elem in elements {
        match elem {
            Element::NodeRef(_) => {
                elem_sizes.push((NODE_W, NODE_H));
            }
            Element::GroupRef(gi) => {
                let g = &groups[*gi];
                if g.children.is_empty() {
                    let w = (text_width(&g.name) + GROUP_H_PAD * 2.0).max(NODE_W);
                    let h = GROUP_HEADER_H + GROUP_V_PAD;
                    elem_sizes.push((w, h));
                } else {
                    // Layout children in a temporary coordinate space
                    let mut child_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
                    let (inner_w, inner_h) = layout_elements(
                        &g.children,
                        nodes,
                        groups,
                        edges,
                        0.0,
                        0.0,
                        &mut child_positions,
                    );
                    // Store child positions relative to (0,0) for later offset
                    for (name, pos) in &child_positions {
                        positions.insert(name.clone(), *pos);
                    }
                    group_internal.insert(*gi, (inner_w, inner_h));

                    let header_w = text_width(&g.name) + GROUP_H_PAD * 2.0;
                    let w = inner_w.max(header_w) + GROUP_H_PAD * 2.0;
                    let h = GROUP_HEADER_H + inner_h + GROUP_V_PAD * 2.0;
                    elem_sizes.push((w, h));
                }
            }
        }
    }

    // Step 2: Build name-to-local-index map for this level's elements
    let mut name_to_local: HashMap<String, usize> = HashMap::new();
    // Also track which names belong to which element (including nested)
    let mut name_to_elem_idx: HashMap<String, usize> = HashMap::new();
    for (i, elem) in elements.iter().enumerate() {
        for name in element_all_names(elem, nodes, groups) {
            name_to_elem_idx.insert(name, i);
        }
        name_to_local.insert(element_name(elem, nodes, groups), i);
    }

    // Step 3: Find edges between elements at this level
    let mut local_edges: Vec<(u32, u32)> = Vec::new();
    let mut seen_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for edge in edges {
        let from_idx = name_to_elem_idx.get(&edge.from);
        let to_idx = name_to_elem_idx.get(&edge.to);
        if let (Some(&fi), Some(&ti)) = (from_idx, to_idx) {
            if fi != ti && seen_edges.insert((fi, ti)) {
                local_edges.push((fi as u32, ti as u32));
            }
        }
    }

    // Step 4: Run Sugiyama on this level
    let vertices: Vec<(u32, (f64, f64))> = elem_sizes
        .iter()
        .enumerate()
        .map(|(i, (w, h))| (i as u32, (*h, *w))) // swap for LTR
        .collect();

    let config = Config {
        vertex_spacing: 15.0 + elements.len() as f64 * 3.0,
        ..Config::default()
    };

    let layouts = from_vertices_and_edges(&vertices, &local_edges, &config);

    // Collect and pack disconnected components
    let mut local_positions: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut x_cursor: f64 = 0.0;
    let component_gap = GROUP_INNER_GAP * 2.0;

    for (coords, _w, _h) in &layouts {
        let mut comp_min_x = f64::MAX;
        let mut comp_min_y = f64::MAX;
        let mut comp_max_x = f64::MIN;
        let mut comp_max_y = f64::MIN;

        for &(id, (sx, sy)) in coords {
            let final_x = sy; // swap back for LTR
            let final_y = sx;
            let (w, h) = elem_sizes[id];
            comp_min_x = comp_min_x.min(final_x);
            comp_min_y = comp_min_y.min(final_y);
            comp_max_x = comp_max_x.max(final_x + w);
            comp_max_y = comp_max_y.max(final_y + h);
        }

        for &(id, (sx, sy)) in coords {
            let final_x = sy - comp_min_x + x_cursor;
            let final_y = sx - comp_min_y;
            local_positions.insert(id, (final_x, final_y));
        }

        x_cursor += (comp_max_x - comp_min_x) + component_gap;
    }

    // Step 5: Compute total bounding box and place elements
    let mut total_max_x: f64 = 0.0;
    let mut total_max_y: f64 = 0.0;

    for (i, elem) in elements.iter().enumerate() {
        let (lx, ly) = local_positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (ew, eh) = elem_sizes[i];
        let ex = start_x + lx;
        let ey = start_y + ly;

        total_max_x = total_max_x.max(lx + ew);
        total_max_y = total_max_y.max(ly + eh);

        match elem {
            Element::NodeRef(ni) => {
                positions.insert(nodes[*ni].name.clone(), (ex, ey, ew, eh));
            }
            Element::GroupRef(gi) => {
                let g = &groups[*gi];
                positions.insert(g.name.clone(), (ex, ey, ew, eh));

                // Offset pre-computed child positions into this group's space
                let content_x = ex + GROUP_H_PAD;
                let content_y = ey + GROUP_HEADER_H + GROUP_V_PAD;
                // Center inner content
                let (inner_w, _inner_h) = group_internal.get(gi).copied().unwrap_or((0.0, 0.0));
                let offset_x = content_x + (ew - GROUP_H_PAD * 2.0 - inner_w).max(0.0) / 2.0;
                let offset_y = content_y;

                for child in &g.children {
                    offset_positions(child, nodes, groups, offset_x, offset_y, positions);
                }
            }
        }
    }

    (total_max_x, total_max_y)
}

/// Offset pre-computed positions (from 0,0-based child layout) by the group's absolute position
fn offset_positions(
    elem: &Element,
    nodes: &[Node],
    groups: &[Group],
    offset_x: f64,
    offset_y: f64,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
) {
    let name = element_name(elem, nodes, groups);
    if let Some(pos) = positions.get_mut(&name) {
        pos.0 += offset_x;
        pos.1 += offset_y;
    }
    if let Element::GroupRef(gi) = elem {
        for child in &groups[*gi].children {
            offset_positions(child, nodes, groups, offset_x, offset_y, positions);
        }
    }
}

// ---------------------------------------------------------------------------
// Edge routing
// ---------------------------------------------------------------------------

fn segment_intersects_rect(
    p1x: f64, p1y: f64, p2x: f64, p2y: f64,
    cx: f64, cy: f64, hw: f64, hh: f64,
) -> bool {
    let margin = 4.0;
    let left = cx - hw - margin;
    let right = cx + hw + margin;
    let top = cy - hh - margin;
    let bottom = cy + hh + margin;

    if (p1x < left && p2x < left) || (p1x > right && p2x > right) {
        return false;
    }
    if (p1y < top && p2y < top) || (p1y > bottom && p2y > bottom) {
        return false;
    }

    let dx = p2x - p1x;
    let dy = p2y - p1y;
    let edges: [(f64, f64, f64, f64); 4] = [
        (left, top, right, top),
        (left, bottom, right, bottom),
        (left, top, left, bottom),
        (right, top, right, bottom),
    ];

    for (ex1, ey1, ex2, ey2) in &edges {
        let edx = ex2 - ex1;
        let edy = ey2 - ey1;
        let denom = dx * edy - dy * edx;
        if denom.abs() < 1e-10 { continue; }
        let t = ((ex1 - p1x) * edy - (ey1 - p1y) * edx) / denom;
        let u = ((ex1 - p1x) * dy - (ey1 - p1y) * dx) / denom;
        if (0.01..=0.99).contains(&t) && (0.0..=1.0).contains(&u) {
            return true;
        }
    }
    false
}

fn route_around_nodes(
    sx: f64, sy: f64, ex: f64, ey: f64,
    from_name: &str, to_name: &str,
    all_bounds: &[(String, f64, f64, f64, f64)], // (name, cx, cy, hw, hh)
    offset: f64,
) -> Vec<(f64, f64)> {
    let (sx, sy, ex, ey) = if offset.abs() > 0.1 {
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / len * offset;
        let ny = dx / len * offset;
        (sx + nx, sy + ny, ex + nx, ey + ny)
    } else {
        (sx, sy, ex, ey)
    };

    let mut blockers: Vec<usize> = Vec::new();
    for (i, (name, cx, cy, hw, hh)) in all_bounds.iter().enumerate() {
        if name == from_name || name == to_name { continue; }
        if segment_intersects_rect(sx, sy, ex, ey, *cx, *cy, *hw, *hh) {
            blockers.push(i);
        }
    }

    if blockers.is_empty() {
        return vec![(sx, sy), (ex, ey)];
    }

    let margin = 20.0;
    let mut waypoints: Vec<(f64, f64)> = vec![(sx, sy)];

    blockers.sort_by(|a, b| {
        let (_, acx, acy, _, _) = all_bounds[*a];
        let (_, bcx, bcy, _, _) = all_bounds[*b];
        let da = (acx - sx).powi(2) + (acy - sy).powi(2);
        let db = (bcx - sx).powi(2) + (bcy - sy).powi(2);
        da.partial_cmp(&db).unwrap()
    });

    for &bi in &blockers {
        let (_, cx, cy, hw, hh) = all_bounds[bi];
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let last = waypoints.last().unwrap();
        let cross = (cx - last.0) * dy - (cy - last.1) * dx;

        if cross.abs() / len < hw + hh {
            if dy.abs() > dx.abs() {
                if cross > 0.0 { waypoints.push((cx + hw + margin, cy)); }
                else { waypoints.push((cx - hw - margin, cy)); }
            } else if cross > 0.0 { waypoints.push((cx, cy - hh - margin)); }
            else { waypoints.push((cx, cy + hh + margin)); }
        }
    }

    waypoints.push((ex, ey));
    waypoints
}

fn build_smooth_path(points: &[(f64, f64)]) -> String {
    if points.len() < 2 { return String::new(); }
    if points.len() == 2 {
        return format!("M{},{} L{},{}", points[0].0, points[0].1, points[1].0, points[1].1);
    }
    let mut d = format!("M{},{}", points[0].0, points[0].1);
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 { d.push_str(&format!(" L{},{}", mid_prev.0, mid_prev.1)); }
        d.push_str(&format!(" Q{},{} {},{}", curr.0, curr.1, mid_next.0, mid_next.1));
    }
    let last = points[points.len() - 1];
    d.push_str(&format!(" L{},{}", last.0, last.1));
    d
}

fn sample_smooth_path(points: &[(f64, f64)], n: usize) -> Vec<(f64, f64)> {
    if points.len() < 2 { return points.to_vec(); }
    if points.len() == 2 {
        return (0..=n).map(|i| {
            let t = i as f64 / n as f64;
            (points[0].0 + (points[1].0 - points[0].0) * t,
             points[0].1 + (points[1].1 - points[0].1) * t)
        }).collect();
    }
    let mut segments: Vec<((f64, f64), (f64, f64), (f64, f64))> = Vec::new();
    let mut cursor = points[0];
    for i in 1..points.len() - 1 {
        let prev = points[i - 1]; let curr = points[i]; let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 { segments.push((cursor, cursor, mid_prev)); cursor = mid_prev; }
        segments.push((cursor, curr, mid_next)); cursor = mid_next;
    }
    let last = *points.last().unwrap();
    segments.push((cursor, cursor, last));
    let per_seg = (n / segments.len()).max(2);
    let mut result = Vec::new();
    for (start, ctrl, end) in &segments {
        for j in 0..per_seg {
            let t = j as f64 / per_seg as f64;
            let mt = 1.0 - t;
            result.push((mt*mt*start.0 + 2.0*mt*t*ctrl.0 + t*t*end.0,
                         mt*mt*start.1 + 2.0*mt*t*ctrl.1 + t*t*end.1));
        }
    }
    result.push(last);
    result
}

fn midpoint_on_path(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() <= 1 { return points.first().copied().unwrap_or((0.0, 0.0)); }
    if points.len() == 2 {
        return ((points[0].0 + points[1].0) / 2.0, (points[0].1 + points[1].1) / 2.0);
    }
    let samples = sample_smooth_path(points, 64);
    let mut lengths = vec![0.0_f64];
    for i in 1..samples.len() {
        let dx = samples[i].0 - samples[i-1].0;
        let dy = samples[i].1 - samples[i-1].1;
        lengths.push(lengths[i-1] + (dx*dx + dy*dy).sqrt());
    }
    let half = *lengths.last().unwrap() / 2.0;
    for i in 1..lengths.len() {
        if lengths[i] >= half {
            let t = (half - lengths[i-1]) / (lengths[i] - lengths[i-1]).max(1e-10);
            return (samples[i-1].0 + (samples[i].0 - samples[i-1].0) * t,
                    samples[i-1].1 + (samples[i].1 - samples[i-1].1) * t);
        }
    }
    *samples.last().unwrap()
}

fn clip_to_rect(cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64) -> (f64, f64) {
    let dx = tx - cx; let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 { return (cx, cy); }
    let mut t = f64::MAX;
    if dx.abs() > 1e-9 { t = t.min(hw / dx.abs()); }
    if dy.abs() > 1e-9 { t = t.min(hh / dy.abs()); }
    (cx + dx * t, cy + dy * t)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    // Layout top-level elements
    layout_elements(
        &diagram.top_level,
        &diagram.nodes,
        &diagram.groups,
        &diagram.edges,
        PADDING,
        PADDING,
        &mut positions,
    );

    // SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (_, (x, y, w, h)) in &positions {
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
        "<style>text {{ font-family: sans-serif; font-size: 12px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render groups (back to front)
    render_groups_recursive(&mut svg, &diagram.top_level, &diagram.nodes, &diagram.groups, &positions);

    // Build node bounds for edge routing
    let all_bounds: Vec<(String, f64, f64, f64, f64)> = diagram
        .nodes
        .iter()
        .filter_map(|n| {
            positions.get(&n.name).map(|(x, y, w, h)| {
                (n.name.clone(), x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)
            })
        })
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

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        let pair_key = if edge.from <= edge.to { (edge.from.clone(), edge.to.clone()) } else { (edge.to.clone(), edge.from.clone()) };
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = { let seen = pair_seen.entry(pair_key).or_insert(0); let v = *seen; *seen += 1; v };
        let offset = if total > 1 { (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0 } else { 0.0 };

        let route = route_around_nodes(cx1, cy1, cx2, cy2, &edge.from, &edge.to, &all_bounds, offset);

        let start_target = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_target = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = clip_to_rect(cx1, cy1, start_target.0, start_target.1, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = clip_to_rect(cx2, cy2, end_target.0, end_target.1, tw / 2.0, th / 2.0);

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 { clipped.extend_from_slice(&route[1..route.len()-1]); }
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
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.9\"/>",
                mx - lw / 2.0 - 3.0, my - 18.0, lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                mx, my - 6.0, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_groups_recursive(
    svg: &mut String,
    elements: &[Element],
    nodes: &[Node],
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        match elem {
            Element::GroupRef(gi) => {
                let g = &groups[*gi];
                if let Some(&(x, y, w, h)) = positions.get(&g.name) {
                    // Group background
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6,4\"/>",
                        x, y, w, h, COLOR_GROUP_FILL, COLOR_GROUP_STROKE
                    ));
                    // Group label
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\">{}</text>",
                        x + 8.0, y + GROUP_HEADER_H * 0.7, escape_xml(&g.name)
                    ));
                }
                // Recurse into children
                render_groups_recursive(svg, &g.children, nodes, groups, positions);
            }
            Element::NodeRef(ni) => {
                let node = &nodes[*ni];
                if let Some(&(x, y, w, h)) = positions.get(&node.name) {
                    render_node(svg, x, y, w, h, node);
                }
            }
        }
    }
}

fn render_node(svg: &mut String, x: f64, y: f64, w: f64, h: f64, node: &Node) {
    let (fill, stroke) = node_colors(&node.node_type);

    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, h, fill, stroke
    ));

    // Draw icon shape
    let icon_cx = x + w / 2.0;
    let icon_cy = y + 22.0;
    render_icon(svg, icon_cx, icon_cy, &node.node_type, stroke);

    // Label
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\">{}</text>",
        x + w / 2.0,
        y + h - 8.0,
        escape_xml(&node.name)
    ));

    // Type badge
    let type_label = format!("{:?}", node.node_type).to_lowercase();
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9\" fill=\"{}\">{}</text>",
        x + w / 2.0,
        y + h - 20.0,
        stroke,
        type_label
    ));
}

fn render_icon(svg: &mut String, cx: f64, cy: f64, nt: &NodeType, color: &str) {
    let r = ICON_SIZE / 2.0;
    match nt {
        NodeType::Server => {
            // Server rack: stacked rectangles
            for i in 0..3 {
                let ry = cy - r + 2.0 + i as f64 * 10.0;
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" rx=\"1\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx - r * 0.6, ry, r * 1.2, color
                ));
            }
        }
        NodeType::Db => {
            // Cylinder
            let rw = r * 0.6;
            let rh = r * 0.8;
            let ell_h = 5.0;
            svg.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - rh + ell_h, rw, ell_h, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - rw, cy - rh + ell_h, cx - rw, cy + rh - ell_h, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx + rw, cy - rh + ell_h, cx + rw, cy + rh - ell_h, color
            ));
            svg.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy + rh - ell_h, rw, ell_h, color
            ));
        }
        NodeType::Lb => {
            // Load balancer: circle with arrows
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, r * 0.5, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy, cx - r * 0.9, cy - 6.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy, cx - r * 0.9, cy + 6.0, color
            ));
        }
        NodeType::Cache => {
            // Lightning bolt
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - 4.0, cy - r * 0.6,
                cx + 2.0, cy - 2.0,
                cx - 2.0, cy + 2.0,
                cx + 4.0, cy + r * 0.6,
                color
            ));
        }
        NodeType::Queue => {
            // Arrow right
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - r * 0.5, cy, cx + r * 0.3, cy, color
            ));
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx + r * 0.1, cy - 5.0, cx + r * 0.5, cy, cx + r * 0.1, cy + 5.0, color
            ));
        }
        NodeType::Storage => {
            // Bucket shape
            svg.push_str(&format!(
                "<path d=\"M{},{} L{},{} L{},{} L{},{} Z\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy - r * 0.5,
                cx + r * 0.5, cy - r * 0.5,
                cx + r * 0.4, cy + r * 0.5,
                cx - r * 0.4, cy + r * 0.5,
                color
            ));
        }
        NodeType::Cdn => {
            // Cloud shape
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - 5.0, cy + 2.0, color
            ));
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 3.0, color
            ));
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx + 5.0, cy + 2.0, color
            ));
        }
        NodeType::Network => {
            // Hexagon
            let s = r * 0.5;
            svg.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - s,
                cx + s * 0.87, cy - s * 0.5,
                cx + s * 0.87, cy + s * 0.5,
                cx, cy + s,
                cx - s * 0.87, cy + s * 0.5,
                cx - s * 0.87, cy - s * 0.5,
                color
            ));
        }
        NodeType::User => {
            // Stick figure
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 8.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy - 3.0, cx, cy + 6.0, color
            ));
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - 7.0, cy + 1.0, cx + 7.0, cy + 1.0, color
            ));
        }
        NodeType::Generic => {
            // Simple box
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.5, cy - r * 0.4, r, r * 0.8, color
            ));
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

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-infra: {}", e);
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
    fn parse_node_with_type() {
        let d = parse("node App type=server\n").unwrap();
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].node_type, NodeType::Server);
    }

    #[test]
    fn parse_node_generic() {
        let d = parse("node Foo\n").unwrap();
        assert_eq!(d.nodes[0].node_type, NodeType::Generic);
    }

    #[test]
    fn parse_group() {
        let input = "group \"VPC\" {\n  node App type=server\n  node DB type=db\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].children.len(), 2);
    }

    #[test]
    fn parse_nested_groups() {
        let input = "group \"AWS\" {\n  group \"VPC\" {\n    node App type=server\n  }\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 2);
    }

    #[test]
    fn parse_edge() {
        let input = "node A\nnode B\nA -> B : \"HTTP\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "HTTP");
    }

    #[test]
    fn render_produces_svg() {
        let input = "node A type=server\nnode B type=db\nA -> B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
