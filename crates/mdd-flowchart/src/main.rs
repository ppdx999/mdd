use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

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
}

#[derive(Debug)]
struct Edge {
    from: usize,
    to: usize,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let prefixes = [
            ("start ", NodeKind::Start),
            ("end ", NodeKind::End),
            ("process ", NodeKind::Process),
            ("decision ", NodeKind::Decision),
        ];

        let mut matched = false;
        for (prefix, kind) in &prefixes {
            if line.starts_with(prefix) {
                let name = line.strip_prefix(prefix).unwrap().trim().to_string();
                let id = nodes.len();
                name_to_id.insert(name.clone(), id);
                nodes.push(Node {
                    name,
                    kind: kind.clone(),
                });
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from_name = parts[0].trim();
            let rest = parts[1];

            let (to_name, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim(), String::new())
            };

            let from_id = name_to_id
                .get(from_name)
                .ok_or_else(|| format!("Unknown node: {}", from_name))?;
            let to_id = name_to_id
                .get(to_name)
                .ok_or_else(|| format!("Unknown node: {}", to_name))?;
            edges.push(Edge {
                from: *from_id,
                to: *to_id,
                label,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram { nodes, edges })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

const NODE_H_PAD: f64 = 20.0;
const NODE_V_PAD: f64 = 12.0;
const NODE_MIN_W: f64 = 100.0;
const NODE_MIN_H: f64 = 40.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";

// Node colors: (fill, stroke)
fn node_colors(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Start => ("#e8f5e9", "#2e7d32"),
        NodeKind::End => ("#f5f5f5", "#616161"),
        NodeKind::Process => ("#e3f2fd", "#1565c0"),
        NodeKind::Decision => ("#fff8e1", "#f57f17"),
    }
}

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
    let complexity = diagram.nodes.len() + diagram.edges.len();

    // Check if the graph is linear (no branching)
    let max_out_degree = {
        let mut out: HashMap<usize, usize> = HashMap::new();
        for e in &diagram.edges {
            *out.entry(e.from).or_insert(0) += 1;
        }
        out.values().copied().max().unwrap_or(1)
    };
    let is_linear = max_out_degree <= 1;

    let factor = if is_linear {
        // Compact spacing for linear flows
        0.6 + (complexity as f64 / 40.0).sqrt() * 0.3
    } else if complexity <= 10 {
        1.0 + (complexity as f64 / 20.0).sqrt() * 0.4
    } else if complexity <= 30 {
        1.0 + (complexity as f64 / 10.0).sqrt() * 0.6
    } else {
        2.0 + (complexity - 30) as f64 * 0.06
    }
    .min(5.0);

    let n = diagram.nodes.len() as f64;
    SpacingConfig {
        nodesep: 30.0 * factor,
        ranksep: 50.0 * factor,
        component_gap: 30.0 * factor,
        vertex_spacing: 8.0 + n * 3.0,
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

/// Node size for layout purposes. Decision diamonds need more space
/// because the text fits inside a rotated square.
fn node_size(node: &Node) -> (f64, f64) {
    let (w, h) = node_rect_size(&node.name);
    match node.kind {
        NodeKind::Decision => {
            // Diamond: text box rotated 45°, so bounding box is larger
            let diag = w.max(h) * 1.2 + 20.0;
            (diag, diag)
        }
        _ => (w, h),
    }
}

// ---------------------------------------------------------------------------
// Edge routing (from DFD/state pattern)
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

    if (p1x < left && p2x < left) || (p1x > right && p2x > right) { return false; }
    if (p1y < top && p2y < top) || (p1y > bottom && p2y > bottom) { return false; }

    let dx = p2x - p1x;
    let dy = p2y - p1y;
    let edges = [
        (left, top, right, top), (left, bottom, right, bottom),
        (left, top, left, bottom), (right, top, right, bottom),
    ];
    for (ex1, ey1, ex2, ey2) in &edges {
        let edx = ex2 - ex1;
        let edy = ey2 - ey1;
        let denom = dx * edy - dy * edx;
        if denom.abs() < 1e-10 { continue; }
        let t = ((ex1 - p1x) * edy - (ey1 - p1y) * edx) / denom;
        let u = ((ex1 - p1x) * dy - (ey1 - p1y) * dx) / denom;
        if (0.01..=0.99).contains(&t) && (0.0..=1.0).contains(&u) { return true; }
    }
    false
}

fn route_around_nodes(
    sx: f64, sy: f64, ex: f64, ey: f64,
    from_id: usize, to_id: usize,
    bounds: &[(f64, f64, f64, f64)], offset: f64,
) -> Vec<(f64, f64)> {
    let (sx, sy, ex, ey) = if offset.abs() > 0.1 {
        let dx = ex - sx; let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        (sx - dy / len * offset, sy + dx / len * offset,
         ex - dy / len * offset, ey + dx / len * offset)
    } else { (sx, sy, ex, ey) };

    let mut blockers = Vec::new();
    for (i, &(cx, cy, hw, hh)) in bounds.iter().enumerate() {
        if i == from_id || i == to_id { continue; }
        if segment_intersects_rect(sx, sy, ex, ey, cx, cy, hw, hh) { blockers.push(i); }
    }
    if blockers.is_empty() { return vec![(sx, sy), (ex, ey)]; }

    let margin = 20.0;
    let mut waypoints = vec![(sx, sy)];
    blockers.sort_by(|a, b| {
        let da = (bounds[*a].0 - sx).powi(2) + (bounds[*a].1 - sy).powi(2);
        let db = (bounds[*b].0 - sx).powi(2) + (bounds[*b].1 - sy).powi(2);
        da.partial_cmp(&db).unwrap()
    });
    for &bi in &blockers {
        let (cx, cy, hw, hh) = bounds[bi];
        let dx = ex - sx; let dy = ey - sy;
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
        let prev = points[i - 1]; let curr = points[i]; let next = points[i + 1];
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
            let t = j as f64 / per_seg as f64; let mt = 1.0 - t;
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
        let dx = samples[i].0 - samples[i-1].0; let dy = samples[i].1 - samples[i-1].1;
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

/// Clip to diamond boundary
fn clip_to_diamond(cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 { return (cx, cy); }
    // Diamond: |x/hw| + |y/hh| = 1
    let t = 1.0 / (dx.abs() / hw + dy.abs() / hh).max(1e-10);
    (cx + dx * t, cy + dy * t)
}

fn clip_to_node(cx: f64, cy: f64, tx: f64, ty: f64, node: &Node) -> (f64, f64) {
    let (w, h) = node_size(node);
    match node.kind {
        NodeKind::Decision => clip_to_diamond(cx, cy, tx, ty, w / 2.0, h / 2.0),
        NodeKind::Start | NodeKind::End => {
            // Ellipse clipping
            let (rw, rh) = (w / 2.0, h / 2.0);
            let dx = tx - cx;
            let dy = ty - cy;
            if dx.abs() < 1e-9 && dy.abs() < 1e-9 { return (cx, cy); }
            let angle = dy.atan2(dx);
            (cx + rw * angle.cos(), cy + rh * angle.sin())
        }
        _ => clip_to_rect(cx, cy, tx, ty, w / 2.0, h / 2.0),
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

    let vertices: Vec<(u32, (f64, f64))> = diagram.nodes.iter().enumerate()
        .map(|(i, n)| { let (w, h) = node_size(n); (i as u32, (h, w)) }) // swap for LTR
        .collect();

    let edges_for_layout: Vec<(u32, u32)> = diagram.edges.iter()
        .filter(|e| e.from != e.to)
        .map(|e| (e.from as u32, e.to as u32))
        .collect();

    let layouts = from_vertices_and_edges(&vertices, &edges_for_layout, &config);

    // Post-scale
    let base = sp.vertex_spacing.max(1.0);
    let nodesep_ratio = sp.nodesep / base;
    let ranksep_ratio = sp.ranksep / base;

    let scaled: Vec<(HashMap<usize, (f64, f64)>, f64, f64)> = layouts.iter()
        .map(|(coords, _w, _h)| {
            let n = coords.len() as f64;
            let cx = coords.iter().map(|(_, (x, _))| x).sum::<f64>() / n;
            let cy = coords.iter().map(|(_, (_, y))| y).sum::<f64>() / n;
            let mut sc = HashMap::new();
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &(id, (sx, sy)) in coords {
                let fx = cy + (sy - cy) * ranksep_ratio; // swap back
                let fy = cx + (sx - cx) * nodesep_ratio;
                let (w, h) = node_size(&diagram.nodes[id]);
                min_x = min_x.min(fx); min_y = min_y.min(fy);
                max_x = max_x.max(fx + w); max_y = max_y.max(fy + h);
                sc.insert(id, (fx, fy));
            }
            (sc, (max_x - min_x).max(0.0), (max_y - min_y).max(0.0))
        }).collect();

    // Pack components
    let total_area: f64 = scaled.iter().map(|(_, w, h)| (w + sp.component_gap) * (h + sp.component_gap)).sum();
    let target_w = total_area.sqrt() * 1.3;
    let mut comp_idx: Vec<usize> = (0..scaled.len()).collect();
    comp_idx.sort_by(|a, b| scaled[*b].2.partial_cmp(&scaled[*a].2).unwrap());

    let mut positions: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut row_x = 0.0_f64;
    let mut row_y = 0.0_f64;
    let mut row_h = 0.0_f64;

    for &ci in &comp_idx {
        let (ref coords, cw, ch) = scaled[ci];
        if row_x > 0.0 && row_x + cw > target_w {
            row_y += row_h + sp.component_gap; row_x = 0.0; row_h = 0.0;
        }
        let cmin_x = coords.values().map(|(x, _)| *x).fold(f64::MAX, f64::min);
        let cmin_y = coords.values().map(|(_, y)| *y).fold(f64::MAX, f64::min);
        for (&id, &(x, y)) in coords { positions.insert(id, (x - cmin_x + row_x, y - cmin_y + row_y)); }
        row_x += cw + sp.component_gap;
        row_h = row_h.max(ch);
    }

    // SVG dimensions
    let (mut max_x, mut max_y) = (0.0_f64, 0.0_f64);
    for (i, n) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = node_size(n);
        max_x = max_x.max(x + w); max_y = max_y.max(y + h);
    }

    let svg_w = max_x + PADDING * 2.0;
    let svg_h = max_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_w, svg_h, svg_w, svg_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render nodes
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let px = PADDING + x;
        let py = PADDING + y;
        let (w, h) = node_size(node);
        let (fill, stroke) = node_colors(&node.kind);

        match node.kind {
            NodeKind::Start | NodeKind::End => {
                let cx = px + w / 2.0;
                let cy = py + h / 2.0;
                svg.push_str(&format!(
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, w / 2.0, h / 2.0, fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                    cx, cy + LINE_HEIGHT * 0.35, escape_xml(&node.name)
                ));
            }
            NodeKind::Process => {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    px, py, w, h, fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
                    px + w / 2.0, py + h / 2.0 + LINE_HEIGHT * 0.35, escape_xml(&node.name)
                ));
            }
            NodeKind::Decision => {
                let cx = px + w / 2.0;
                let cy = py + h / 2.0;
                let hw = w / 2.0;
                let hh = h / 2.0;
                svg.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    cx, cy - hh,       // top
                    cx + hw, cy,       // right
                    cx, cy + hh,       // bottom
                    cx - hw, cy,       // left
                    fill, stroke
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" font-size=\"12\">{}</text>",
                    cx, cy + LINE_HEIGHT * 0.3, escape_xml(&node.name)
                ));
            }
        }
    }

    // Node bounds for routing
    let bounds: Vec<(f64, f64, f64, f64)> = diagram.nodes.iter().enumerate()
        .map(|(i, n)| {
            let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
            let (w, h) = node_size(n);
            (PADDING + x + w / 2.0, PADDING + y + h / 2.0, w / 2.0, h / 2.0)
        }).collect();

    // Reciprocal counting
    let mut pair_count: HashMap<(usize, usize), usize> = HashMap::new();
    for e in &diagram.edges {
        if e.from == e.to { continue; }
        let key = (e.from.min(e.to), e.from.max(e.to));
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(usize, usize), usize> = HashMap::new();

    // Render edges
    for edge in &diagram.edges {
        let (x1, y1) = positions.get(&edge.from).copied().unwrap_or((0.0, 0.0));
        let (x2, y2) = positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = node_size(&diagram.nodes[edge.from]);
        let (tw, th) = node_size(&diagram.nodes[edge.to]);

        let cx1 = PADDING + x1 + fw / 2.0;
        let cy1 = PADDING + y1 + fh / 2.0;
        let cx2 = PADDING + x2 + tw / 2.0;
        let cy2 = PADDING + y2 + th / 2.0;

        // Self-loop
        if edge.from == edge.to {
            let rx = PADDING + x1 + fw;
            let ry_top = PADDING + y1 + fh * 0.3;
            let ry_bot = PADDING + y1 + fh * 0.7;
            svg.push_str(&format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                rx, ry_top, rx + 35.0, ry_top - 15.0, rx + 35.0, ry_bot + 15.0, rx, ry_bot, COLOR_EDGE
            ));
            if !edge.label.is_empty() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                    rx + 40.0, (ry_top + ry_bot) / 2.0 + 4.0, COLOR_EDGE, escape_xml(&edge.label)
                ));
            }
            continue;
        }

        let pair_key = (edge.from.min(edge.to), edge.from.max(edge.to));
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = { let s = pair_seen.entry(pair_key).or_insert(0); let v = *s; *s += 1; v };
        let offset = if total > 1 { (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0 } else { 0.0 };

        let route = route_around_nodes(cx1, cy1, cx2, cy2, edge.from, edge.to, &bounds, offset);

        let st = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let et = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = clip_to_node(cx1, cy1, st.0, st.1, &diagram.nodes[edge.from]);
        let (ax2, ay2) = clip_to_node(cx2, cy2, et.0, et.1, &diagram.nodes[edge.to]);

        let mut clipped = vec![(ax1, ay1)];
        if route.len() > 2 { clipped.extend_from_slice(&route[1..route.len()-1]); }
        clipped.push((ax2, ay2));

        let path_d = if clipped.len() == 2 {
            format!("M{},{} L{},{}", clipped[0].0, clipped[0].1, clipped[1].0, clipped[1].1)
        } else { build_smooth_path(&clipped) };

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
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                mx, my - 6.0, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-flowchart - Render a flowchart as SVG

Usage: mdd-flowchart < input.flowchart

Define nodes with a type prefix (start, end, process, decision),
then connect them with \"->\" edges. Add labels with \" : \".

Example:
  start Begin
  process DoWork
  decision OK?
  end Done

  Begin -> DoWork
  DoWork -> OK?
  OK? -> Done : \"Yes\"
  OK? -> DoWork : \"No\"
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
        Err(e) => { eprintln!("mdd-flowchart: {}", e); std::process::exit(1); }
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
    fn parse_start() {
        let d = parse("start Begin\n").unwrap();
        assert_eq!(d.nodes[0].kind, NodeKind::Start);
    }

    #[test]
    fn parse_end() {
        let d = parse("end Done\n").unwrap();
        assert_eq!(d.nodes[0].kind, NodeKind::End);
    }

    #[test]
    fn parse_process() {
        let d = parse("process DoWork\n").unwrap();
        assert_eq!(d.nodes[0].kind, NodeKind::Process);
    }

    #[test]
    fn parse_decision() {
        let d = parse("decision IsValid\n").unwrap();
        assert_eq!(d.nodes[0].kind, NodeKind::Decision);
    }

    #[test]
    fn parse_edge_with_label() {
        let input = "start A\nend B\nA -> B : \"Yes\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "Yes");
    }

    #[test]
    fn parse_edge_without_label() {
        let input = "start A\nend B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "");
    }

    #[test]
    fn render_produces_svg() {
        let input = "start A\nprocess B\ndecision C\nend D\nA -> B\nB -> C\nC -> D : \"Yes\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("polygon")); // diamond
        assert!(svg.contains("ellipse")); // start/end
    }

    #[test]
    fn spacing_scales() {
        let small = parse("start A\nend B\nA -> B\n").unwrap();
        let big = parse("start A\nprocess B\nprocess C\nprocess D\nend E\nA -> B\nB -> C\nC -> D\nD -> E\n").unwrap();
        assert!(compute_spacing(&big).nodesep > compute_spacing(&small).nodesep);
    }
}
