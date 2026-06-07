use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum NodeKind {
    Start,
    End,
    Process,
    Decision,
}

#[derive(Debug)]
struct Node {
    name: String,
    kind: NodeKind,
    lane: usize, // index into lanes
    order: usize, // global definition order (for vertical positioning)
}

#[derive(Debug)]
struct Edge {
    from: usize,
    to: usize,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    lanes: Vec<String>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut lanes: Vec<String> = Vec::new();
    let mut lane_to_id: HashMap<String, usize> = HashMap::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut order = 0_usize;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Lane definition
        if line.starts_with("lane ") {
            let name = line.strip_prefix("lane ").unwrap().trim().to_string();
            let id = lanes.len();
            lane_to_id.insert(name.clone(), id);
            lanes.push(name);
            continue;
        }

        // Edge: from -> to : "label"
        if line.contains(" -> ") && !line.contains(": ") ||
           (line.contains(" -> ") && !line.split(" -> ").next().unwrap().contains(": ")) {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from_name = parts[0].trim();
            let rest = parts[1];
            let (to_name, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (to_part.trim(), label_part.trim().trim_matches('"').to_string())
            } else {
                (rest.trim(), String::new())
            };

            let from_id = name_to_id
                .get(from_name)
                .ok_or_else(|| format!("Unknown node: {}", from_name))?;
            let to_id = name_to_id
                .get(to_name)
                .ok_or_else(|| format!("Unknown node: {}", to_name))?;
            edges.push(Edge { from: *from_id, to: *to_id, label });
            continue;
        }

        // Node definition: lane: kind name
        if let Some((lane_name, rest)) = line.split_once(": ") {
            let lane_name = lane_name.trim();
            let lane_id = *lane_to_id
                .get(lane_name)
                .ok_or_else(|| format!("Unknown lane: {}", lane_name))?;

            let (kind, name) = if let Some(n) = rest.strip_prefix("start ") {
                (NodeKind::Start, n.trim().to_string())
            } else if let Some(n) = rest.strip_prefix("end ") {
                (NodeKind::End, n.trim().to_string())
            } else if let Some(n) = rest.strip_prefix("process ") {
                (NodeKind::Process, n.trim().to_string())
            } else if let Some(n) = rest.strip_prefix("decision ") {
                (NodeKind::Decision, n.trim().to_string())
            } else {
                return Err(format!("Invalid node syntax: {}", rest));
            };

            let id = nodes.len();
            name_to_id.insert(name.clone(), id);
            nodes.push(Node { name, kind, lane: lane_id, order });
            order += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram { lanes, nodes, edges })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 30.0;

const LANE_MIN_W: f64 = 180.0;
const LANE_H_PAD: f64 = 20.0;
const LANE_HEADER_H: f64 = 36.0;

const NODE_H_PAD: f64 = 16.0;
const NODE_V_PAD: f64 = 10.0;
const NODE_MIN_W: f64 = 100.0;
const NODE_MIN_H: f64 = 36.0;
const NODE_V_GAP: f64 = 50.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_LANE_HEADER: &str = "#f0f0f0";
const COLOR_LANE_HEADER_TEXT: &str = "#333";
const LANE_COLORS: [&str; 4] = ["#ffffff", "#fcfcfc", "#ffffff", "#fcfcfc"];

fn node_colors(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Start => ("#f7fcf7", "#2e7d32"),
        NodeKind::End => ("#fcfcfc", "#616161"),
        NodeKind::Process => ("#f7faff", "#1565c0"),
        NodeKind::Decision => ("#fffef7", "#f57f17"),
    }
}

// ---------------------------------------------------------------------------
// Text & sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

fn node_rect_size(name: &str) -> (f64, f64) {
    let w = (text_width(name) + NODE_H_PAD * 2.0).max(NODE_MIN_W);
    let h = (LINE_HEIGHT + NODE_V_PAD * 2.0).max(NODE_MIN_H);
    (w, h)
}

fn node_size(node: &Node) -> (f64, f64) {
    let (w, h) = node_rect_size(&node.name);
    match node.kind {
        NodeKind::Decision => {
            let dw = w * 1.3 + 16.0;
            let dh = h * 1.1 + 8.0;
            (dw, dh)
        }
        _ => (w, h),
    }
}

// ---------------------------------------------------------------------------
// Layout & SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    if diagram.lanes.is_empty() || diagram.nodes.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"></svg>".to_string();
    }

    // Compute lane widths based on widest node in each lane
    let mut lane_widths: Vec<f64> = diagram.lanes.iter()
        .map(|name| (text_width(name) + LANE_H_PAD * 2.0).max(LANE_MIN_W))
        .collect();

    for node in &diagram.nodes {
        let (nw, _) = node_size(node);
        let needed = nw + LANE_H_PAD * 2.0;
        lane_widths[node.lane] = lane_widths[node.lane].max(needed);
    }

    // Compute lane X positions (left edge of each lane)
    let mut lane_x: Vec<f64> = Vec::new();
    let mut x = PADDING;
    for w in &lane_widths {
        lane_x.push(x);
        x += w;
    }
    let total_w = x + PADDING;

    // Assign vertical positions to nodes.
    // Nodes are positioned by their global order, but we need to ensure
    // that connected nodes respect edge ordering (from before to).
    // Simple approach: assign y based on "row" - each node gets the next
    // available row, considering that edges should flow downward.

    // Compute row for each node using topological-ish ordering
    let mut node_row: Vec<usize> = vec![0; diagram.nodes.len()];
    // Process nodes in definition order, but push down targets of edges
    for node in &diagram.nodes {
        let id = diagram.nodes.iter().position(|n| std::ptr::eq(n, node)).unwrap();
        // Ensure this node is at least at its order position
        node_row[id] = node_row[id].max(node.order);
    }
    // Push targets of edges down so they come after their sources.
    // Skip back-edges (where target was defined before source) to avoid
    // infinite row inflation from loops.
    for _ in 0..diagram.nodes.len() {
        for edge in &diagram.edges {
            let is_back_edge = diagram.nodes[edge.to].order < diagram.nodes[edge.from].order;
            if !is_back_edge && node_row[edge.to] <= node_row[edge.from] {
                node_row[edge.to] = node_row[edge.from] + 1;
            }
        }
    }

    // Compute Y position for each row
    let max_row = *node_row.iter().max().unwrap_or(&0);
    let mut row_y: Vec<f64> = Vec::new();
    let mut y = PADDING + LANE_HEADER_H + NODE_V_GAP / 2.0;
    for row in 0..=max_row {
        row_y.push(y);
        // Find max node height in this row
        let max_h: f64 = diagram.nodes.iter().enumerate()
            .filter(|(i, _)| node_row[*i] == row)
            .map(|(_, n)| node_size(n).1)
            .fold(NODE_MIN_H, f64::max);
        y += max_h + NODE_V_GAP;
    }
    let total_h = y + PADDING;

    // Compute node positions (center of lane, at row y)
    let mut node_positions: Vec<(f64, f64, f64, f64)> = Vec::new(); // (x, y, w, h)
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (nw, nh) = node_size(node);
        let lane_cx = lane_x[node.lane] + lane_widths[node.lane] / 2.0;
        let nx = lane_cx - nw / 2.0;
        let ny = row_y[node_row[i]];
        node_positions.push((nx, ny, nw, nh));
    }

    // Build SVG
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render lanes (background strips + headers)
    for (i, lane_name) in diagram.lanes.iter().enumerate() {
        let lx = lane_x[i];
        let lw = lane_widths[i];
        let bg = LANE_COLORS[i % LANE_COLORS.len()];

        // Lane background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#ddd\" stroke-width=\"0.5\"/>",
            lx, PADDING, lw, total_h - PADDING * 2.0, bg
        ));

        // Lane header
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            lx, PADDING, lw, LANE_HEADER_H, COLOR_LANE_HEADER
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\" font-size=\"14\">{}</text>",
            lx + lw / 2.0,
            PADDING + LANE_HEADER_H / 2.0 + LINE_HEIGHT * 0.35,
            COLOR_LANE_HEADER_TEXT,
            escape_xml(lane_name)
        ));
    }

    // Render nodes
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (nx, ny, nw, nh) = node_positions[i];
        let (fill, stroke) = node_colors(&node.kind);

        match node.kind {
            NodeKind::Start | NodeKind::End => {
                let cx = nx + nw / 2.0;
                let cy = ny + nh / 2.0;
                svg.push_str(&format!(
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, nw / 2.0, nh / 2.0, fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                    cx, cy + LINE_HEIGHT * 0.35, escape_xml(&node.name)
                ));
            }
            NodeKind::Process => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    nx, ny, nw, nh, fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                    nx + nw / 2.0, ny + nh / 2.0 + LINE_HEIGHT * 0.35, escape_xml(&node.name)
                ));
            }
            NodeKind::Decision => {
                let cx = nx + nw / 2.0;
                let cy = ny + nh / 2.0;
                let hw = nw / 2.0;
                let hh = nh / 2.0;
                svg.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx, cy - hh, cx + hw, cy, cx, cy + hh, cx - hw, cy, fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" font-size=\"12\">{}</text>",
                    cx, cy + LINE_HEIGHT * 0.3, escape_xml(&node.name)
                ));
            }
        }
    }

    // Render edges
    for edge in &diagram.edges {
        let (fx, fy, fw, fh) = node_positions[edge.from];
        let (tx, ty, tw, th) = node_positions[edge.to];

        let from_cx = fx + fw / 2.0;
        let from_cy = fy + fh / 2.0;
        let to_cx = tx + tw / 2.0;
        let to_cy = ty + th / 2.0;

        // Clip to node boundaries
        let (ax1, ay1) = clip_to_node(from_cx, from_cy, to_cx, to_cy, &diagram.nodes[edge.from]);
        let (ax2, ay2) = clip_to_node(to_cx, to_cy, from_cx, from_cy, &diagram.nodes[edge.to]);

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            ax1, ay1, ax2, ay2, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            let lx = (ax1 + ax2) / 2.0;
            let ly = (ay1 + ay2) / 2.0 - 6.0;
            let lw = text_width(&edge.label);
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.9\"/>",
                lx - lw / 2.0 - 3.0, ly - 12.0, lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                lx, ly, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn clip_to_node(cx: f64, cy: f64, tx: f64, ty: f64, node: &Node) -> (f64, f64) {
    let (w, h) = node_size(node);
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 { return (cx, cy); }

    match node.kind {
        NodeKind::Decision => {
            // Diamond: |x/hw| + |y/hh| = 1
            let hw = w / 2.0;
            let hh = h / 2.0;
            let t = 1.0 / (dx.abs() / hw + dy.abs() / hh).max(1e-10);
            (cx + dx * t, cy + dy * t)
        }
        NodeKind::Start | NodeKind::End => {
            // Ellipse
            let angle = dy.atan2(dx);
            (cx + (w / 2.0) * angle.cos(), cy + (h / 2.0) * angle.sin())
        }
        _ => {
            // Rectangle
            let hw = w / 2.0;
            let hh = h / 2.0;
            let mut t = f64::MAX;
            if dx.abs() > 1e-9 { t = t.min(hw / dx.abs()); }
            if dy.abs() > 1e-9 { t = t.min(hh / dy.abs()); }
            (cx + dx * t, cy + dy * t)
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => { eprintln!("mdd-swimlane: {}", e); std::process::exit(1); }
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
    fn parse_lane() {
        let d = parse("lane Sales\n").unwrap();
        assert_eq!(d.lanes.len(), 1);
        assert_eq!(d.lanes[0], "Sales");
    }

    #[test]
    fn parse_node_in_lane() {
        let input = "lane Dev\nDev: process Build\n";
        let d = parse(input).unwrap();
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].kind, NodeKind::Process);
        assert_eq!(d.nodes[0].lane, 0);
    }

    #[test]
    fn parse_all_node_types() {
        let input = "lane L\nL: start A\nL: process B\nL: decision C\nL: end D\n";
        let d = parse(input).unwrap();
        assert_eq!(d.nodes[0].kind, NodeKind::Start);
        assert_eq!(d.nodes[1].kind, NodeKind::Process);
        assert_eq!(d.nodes[2].kind, NodeKind::Decision);
        assert_eq!(d.nodes[3].kind, NodeKind::End);
    }

    #[test]
    fn parse_edge_with_label() {
        let input = "lane L\nL: start A\nL: end B\nA -> B : \"Yes\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "Yes");
    }

    #[test]
    fn parse_edge_without_label() {
        let input = "lane L\nL: start A\nL: end B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "");
    }

    #[test]
    fn parse_multi_lane() {
        let input = "lane A\nlane B\nA: start X\nB: end Y\nX -> Y\n";
        let d = parse(input).unwrap();
        assert_eq!(d.lanes.len(), 2);
        assert_eq!(d.nodes[0].lane, 0);
        assert_eq!(d.nodes[1].lane, 1);
    }

    #[test]
    fn render_produces_svg() {
        let input = "lane L\nL: start A\nL: process B\nL: end C\nA -> B\nB -> C\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
