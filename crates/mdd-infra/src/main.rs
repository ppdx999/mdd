use std::collections::HashMap;
use std::io::{self, Read};
use mdd_layout;

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
    Phone,
    Cloud,
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
            "phone" | "telephone" => NodeType::Phone,
            "cloud" | "internet" | "pstn" => NodeType::Cloud,
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
    show_type: bool,
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
    let mut show_type = false;

    // Stack for nested groups: (group_index, children_so_far)
    let mut group_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "show type" {
            show_type = true;
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
        show_type,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PADDING: f64 = 40.0;

const NODE_W: f64 = 100.0;
const NODE_H: f64 = 70.0;
const ICON_SIZE: f64 = 32.0;

const GROUP_H_PAD: f64 = 16.0;
const GROUP_V_PAD: f64 = 12.0;
const GROUP_HEADER_H: f64 = 28.0;
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
        NodeType::Phone => ("#e8eaf6", "#4527a0"),
        NodeType::Cloud => ("#e0f7fa", "#00838f"),
        NodeType::Generic => ("#f5f5f5", "#757575"),
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    mdd_layout::text::text_width(s)
}


// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> mdd_layout::LayoutGraph {
    let mut graph = mdd_layout::LayoutGraph::new();

    // Add nodes
    for node in &diagram.nodes {
        let w = (text_width(&node.name) + 24.0).max(NODE_W);
        graph.nodes.push(mdd_layout::LayoutNode {
            name: node.name.clone(),
            width: w,
            height: NODE_H,
        });
    }

    // Add groups (recursively convert children)
    for group in &diagram.groups {
        let children = group.children.iter().map(|e| match e {
            Element::NodeRef(i) => mdd_layout::LayoutElement::NodeRef(*i),
            Element::GroupRef(i) => mdd_layout::LayoutElement::GroupRef(*i),
        }).collect();
        graph.groups.push(mdd_layout::LayoutGroup {
            name: group.name.clone(),
            children,
        });
    }

    // Copy top_level elements
    graph.top_level = diagram.top_level.iter().map(|e| match e {
        Element::NodeRef(i) => mdd_layout::LayoutElement::NodeRef(*i),
        Element::GroupRef(i) => mdd_layout::LayoutElement::GroupRef(*i),
    }).collect();

    // Copy edges
    for edge in &diagram.edges {
        graph.edges.push(mdd_layout::LayoutEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
        });
    }

    graph
}

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);
    // Adaptive spacing based on complexity
    let n = diagram.nodes.len() as f64;
    let e = diagram.edges.len() as f64;
    let g = diagram.groups.len() as f64;
    let complexity = n + e + g * 3.0;
    let scale = 1.0 + (complexity / 10.0).sqrt() * 0.4;
    let node_sep = (40.0 * scale).max(50.0);
    let rank_sep = (50.0 * scale).max(60.0);
    let group_pad = (30.0 * scale).max(36.0);

    let config = mdd_layout::LayoutConfig {
        padding: PADDING,
        node_sep,
        rank_sep,
        group_h_pad: group_pad,
        group_v_pad: group_pad,
        group_header_h: GROUP_HEADER_H,
        default_node_w: NODE_W,
        default_node_h: NODE_H,
        ..mdd_layout::LayoutConfig::default()
    };
    let result = mdd_layout::layout(&graph, &config);
    let positions = result.positions;
    let edge_waypoints = result.edge_waypoints;

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
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 12px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render groups (back to front)
    render_groups_recursive(&mut svg, &diagram.top_level, &diagram.nodes, &diagram.groups, &positions, diagram.show_type);

    // Build node bounds for edge routing (nodes only, not groups —
    // edges are allowed to cross group borders since they're dashed)
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

        // Use virtual node waypoints if available, otherwise route around nodes
        let edge_key = format!("{}→{}", edge.from, edge.to);
        let route = if let Some(waypoints) = edge_waypoints.get(&edge_key) {
            // Build route: start → waypoints → end
            let mut r = vec![(cx1, cy1)];
            r.extend(waypoints);
            r.push((cx2, cy2));
            r
        } else {
            mdd_layout::edge::route_around_nodes(cx1, cy1, cx2, cy2, &edge.from, &edge.to, &all_bounds, offset)
        };

        let start_target = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_target = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = mdd_layout::edge::clip_to_rect(cx1, cy1, start_target.0, start_target.1, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = mdd_layout::edge::clip_to_rect(cx2, cy2, end_target.0, end_target.1, tw / 2.0, th / 2.0);

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 { clipped.extend_from_slice(&route[1..route.len()-1]); }
        clipped.push((ax2, ay2));

        let path_d = if clipped.len() == 2 {
            format!("M{},{} L{},{}", clipped[0].0, clipped[0].1, clipped[1].0, clipped[1].1)
        } else {
            mdd_layout::edge::build_smooth_path(&clipped)
        };

        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            path_d, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            let (mx, my) = mdd_layout::edge::midpoint_on_path(&clipped);
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
    show_type: bool,
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
                render_groups_recursive(svg, &g.children, nodes, groups, positions, show_type);
            }
            Element::NodeRef(ni) => {
                let node = &nodes[*ni];
                if let Some(&(x, y, w, h)) = positions.get(&node.name) {
                    render_node(svg, x, y, w, h, node, show_type);
                }
            }
        }
    }
}

fn render_node(svg: &mut String, x: f64, y: f64, w: f64, h: f64, node: &Node, show_type: bool) {
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
    if show_type {
        let type_label = format!("{:?}", node.node_type).to_lowercase();
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9\" fill=\"{}\">{}</text>",
            x + w / 2.0,
            y + h - 20.0,
            stroke,
            type_label
        ));
    }
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
        NodeType::Phone => {
            // Landline phone: base + handset
            // Base
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - r * 0.55, cy + 1.0, r * 1.1, r * 0.5, color
            ));
            // Handset (receiver arc)
            svg.push_str(&format!(
                "<path d=\"M{},{} Q{},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.5\" stroke-linecap=\"round\"/>",
                cx - r * 0.45, cy + 1.0,
                cx, cy - r * 0.7,
                cx + r * 0.45, cy + 1.0,
                color
            ));
        }
        NodeType::Cloud => {
            // Network cloud shape using arcs
            let w = r * 0.8;
            let h = r * 0.5;
            svg.push_str(&format!(
                "<path d=\"M{},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{} \
                 a{},{} 0 0,1 {},{}\" \
                 fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx - w * 0.6, cy + h * 0.4,
                // bottom-left bump
                h * 0.7, h * 0.7, w * 0.1, -(h * 0.8),
                // top-left bump
                h * 0.6, h * 0.6, w * 0.5, -(h * 0.3),
                // top-right bump
                h * 0.7, h * 0.7, w * 0.6, h * 0.2,
                // right bump
                h * 0.6, h * 0.6, w * 0.0, h * 0.9,
                // bottom line back
                h * 0.3, h * 0.3, -(w * 1.2), 0.0,
                color
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
    mdd_layout::text::escape_xml(s)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-infra - Render an infrastructure diagram as SVG

Usage: mdd-infra < input.infra

Define nodes with \"node Name [type=TYPE]\" where TYPE is one of:
server, db, lb, cache, queue, storage, cdn, network, user.
Group nodes with \"group \"Name\" { ... }\" (nesting allowed).
Connect nodes with \"A -> B\" or \"A -> B : \"label\"\".

Example:
  node Client type=user
  node WebServer type=server
  node Database type=db
  Client -> WebServer : \"HTTP\"
  WebServer -> Database : \"SQL\"
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
