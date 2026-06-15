use std::collections::HashMap;
use std::io::{self, Read};

#[derive(Debug, Clone)]
enum NodeKind {
    Actor,
    Usecase,
}

#[derive(Debug)]
struct Node {
    label: String,
    kind: NodeKind,
    package: Option<String>,
}

#[derive(Debug)]
struct Diagram {
    nodes: Vec<Node>,
    edges: Vec<(usize, usize)>,
    /// Unique package names in order of first appearance
    packages: Vec<String>,
}

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut current_package: Option<String> = None;
    let mut packages: Vec<String> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "}" {
            current_package = None;
            continue;
        }

        if line.starts_with("package ") {
            let rest = line.strip_prefix("package ").unwrap();
            let label = if let Some(rest) = rest.strip_suffix(" {") {
                rest.trim_matches('"').to_string()
            } else {
                return Err(format!("Invalid package syntax: {}", line));
            };
            if !packages.contains(&label) {
                packages.push(label.clone());
            }
            current_package = Some(label);
            continue;
        }

        if line.starts_with("actor ") {
            let label = line.strip_prefix("actor ").unwrap().trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::Actor,
                package: current_package.clone(),
            });
            continue;
        }

        if line.starts_with("usecase ") {
            let label = line.strip_prefix("usecase ").unwrap().trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::Usecase,
                package: current_package.clone(),
            });
            continue;
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from = parts[0].trim();
            let to = parts[1].trim();
            let from_id = name_to_id
                .get(from)
                .ok_or_else(|| format!("Unknown node: {}", from))?;
            let to_id = name_to_id
                .get(to)
                .ok_or_else(|| format!("Unknown node: {}", to))?;
            edges.push((*from_id, *to_id));
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram {
        nodes,
        edges,
        packages,
    })
}

const MIN_NODE_WIDTH: f64 = 120.0;
const LINE_HEIGHT: f64 = 18.0;
const MIN_NODE_HEIGHT: f64 = 50.0;
const ELLIPSE_H_PAD: f64 = 30.0;
const PADDING: f64 = 40.0;
const ACTOR_WIDTH: f64 = 60.0;
const ACTOR_HEIGHT: f64 = 80.0;
const MAX_LINE_CHARS: usize = 14;
const PKG_PADDING: f64 = 20.0;
const PKG_HEADER_H: f64 = 24.0;

const ACTOR_COL_WIDTH: f64 = 100.0;
const COL_GAP: f64 = 60.0;
const UC_GAP_Y: f64 = 20.0;
const PKG_GAP_Y: f64 = 30.0;

const COLOR_DARK: &str = "#333";
const COLOR_MID: &str = "#666";
const COLOR_FILL: &str = "#f0f8ff";

/// Split a CamelCase or space-separated label into words
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

/// Wrap words into lines that fit within MAX_LINE_CHARS
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

fn usecase_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let text_w = lines
        .iter()
        .map(|l| mdd_layout::text::text_width(l))
        .fold(0.0_f64, f64::max);
    let w = (text_w + ELLIPSE_H_PAD * 2.0).max(MIN_NODE_WIDTH);
    let text_height = lines.len() as f64 * LINE_HEIGHT;
    let h = (text_height + 24.0).max(MIN_NODE_HEIGHT);
    (w, h)
}

// ---------------------------------------------------------------------------
// Custom layout: actors on sides, usecases in center
// ---------------------------------------------------------------------------

struct Layout {
    positions: HashMap<String, (f64, f64, f64, f64)>,
}

fn compute_layout(diagram: &Diagram) -> Layout {
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    // Separate actors and usecases
    let actor_ids: Vec<usize> = diagram.nodes.iter().enumerate()
        .filter(|(_, n)| matches!(n.kind, NodeKind::Actor))
        .map(|(i, _)| i).collect();
    let usecase_ids: Vec<usize> = diagram.nodes.iter().enumerate()
        .filter(|(_, n)| matches!(n.kind, NodeKind::Usecase))
        .map(|(i, _)| i).collect();

    // --- Step 1: Layout usecases vertically by package (Sugiyama-like ordering) ---

    // Build usecase-to-usecase edges for ordering within packages
    let uc_set: std::collections::HashSet<usize> = usecase_ids.iter().copied().collect();
    let mut uc_edges: Vec<(usize, usize)> = Vec::new();
    for &(from, to) in &diagram.edges {
        if uc_set.contains(&from) && uc_set.contains(&to) {
            uc_edges.push((from, to));
        }
    }

    // Order usecases: group by package, topological sort within each group
    // For simplicity: use declaration order + push targets of uc edges after sources
    let mut uc_order: Vec<usize> = Vec::new();
    let mut placed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    // First, place by package order
    for pkg in &diagram.packages {
        let pkg_ucs: Vec<usize> = usecase_ids.iter()
            .filter(|&&i| diagram.nodes[i].package.as_deref() == Some(pkg))
            .copied().collect();

        // Simple topological sort within package
        let mut sorted = topo_sort_subset(&pkg_ucs, &uc_edges);
        // Fallback: if topo sort didn't include all, append remaining
        for &id in &pkg_ucs {
            if !sorted.contains(&id) {
                sorted.push(id);
            }
        }

        for id in sorted {
            if placed.insert(id) {
                uc_order.push(id);
            }
        }
    }

    // Unpackaged usecases
    for &id in &usecase_ids {
        if placed.insert(id) {
            uc_order.push(id);
        }
    }

    // --- Step 2: Position usecases in center column ---

    // Compute max usecase width for center column sizing
    let max_uc_w: f64 = uc_order.iter()
        .map(|&i| usecase_size(&diagram.nodes[i].label).0)
        .fold(0.0_f64, f64::max);

    let center_x = PADDING + ACTOR_COL_WIDTH + COL_GAP;
    let mut cur_y = PADDING;
    let mut current_pkg: Option<&str> = None;
    let mut pkg_start_y: HashMap<String, f64> = HashMap::new();

    for &uc_id in &uc_order {
        let node = &diagram.nodes[uc_id];
        let pkg = node.package.as_deref();

        // Check if entering a new package
        if pkg != current_pkg.as_deref() {
            if current_pkg.is_some() {
                cur_y += PKG_GAP_Y; // gap between packages
            }
            if let Some(p) = pkg {
                pkg_start_y.insert(p.to_string(), cur_y);
                cur_y += PKG_HEADER_H + PKG_PADDING; // header + padding
            }
            current_pkg = pkg;
        }

        let (w, h) = usecase_size(&node.label);
        let x = center_x + (max_uc_w - w) / 2.0; // center-align within column
        positions.insert(node.label.clone(), (x, cur_y, w, h));
        cur_y += h + UC_GAP_Y;
    }

    let center_bottom = cur_y;

    // --- Step 3: Position actors on left and right sides ---

    // Compute barycenter Y for each actor
    let mut actor_bary: Vec<(usize, f64)> = Vec::new();
    for &aid in &actor_ids {
        let connected_ys: Vec<f64> = diagram.edges.iter()
            .filter_map(|&(from, to)| {
                let target = if from == aid { to } else if to == aid { from } else { return None };
                positions.get(&diagram.nodes[target].label).map(|(_, y, _, h)| y + h / 2.0)
            })
            .collect();

        let bary = if connected_ys.is_empty() {
            center_bottom / 2.0 // default to middle
        } else {
            connected_ys.iter().sum::<f64>() / connected_ys.len() as f64
        };
        actor_bary.push((aid, bary));
    }

    // Count edges per actor
    let mut actor_edge_count: HashMap<usize, usize> = HashMap::new();
    for &aid in &actor_ids {
        let count = diagram.edges.iter()
            .filter(|&&(from, to)| from == aid || to == aid)
            .count();
        actor_edge_count.insert(aid, count);
    }

    // Sort actors by edge count descending, assign to side with fewer total edges
    let mut sorted_actors: Vec<(usize, f64)> = actor_bary.clone();
    sorted_actors.sort_by(|a, b| {
        actor_edge_count.get(&b.0).unwrap_or(&0)
            .cmp(actor_edge_count.get(&a.0).unwrap_or(&0))
    });

    let mut left_actors: Vec<(usize, f64)> = Vec::new();
    let mut right_actors: Vec<(usize, f64)> = Vec::new();
    let mut left_edges: usize = 0;
    let mut right_edges: usize = 0;

    for (aid, bary) in sorted_actors {
        let count = *actor_edge_count.get(&aid).unwrap_or(&0);
        if left_edges <= right_edges {
            left_actors.push((aid, bary));
            left_edges += count;
        } else {
            right_actors.push((aid, bary));
            right_edges += count;
        }
    }

    // Sort each side by barycenter Y for vertical positioning
    left_actors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    right_actors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let right_x = center_x + max_uc_w + COL_GAP;

    // Position left actors
    for &(aid, bary) in &left_actors {
        let y = bary - ACTOR_HEIGHT / 2.0;
        let x = PADDING + (ACTOR_COL_WIDTH - ACTOR_WIDTH) / 2.0;
        positions.insert(diagram.nodes[aid].label.clone(), (x, y, ACTOR_WIDTH, ACTOR_HEIGHT));
    }

    // Position right actors
    for &(aid, bary) in &right_actors {
        let y = bary - ACTOR_HEIGHT / 2.0;
        let x = right_x + (ACTOR_COL_WIDTH - ACTOR_WIDTH) / 2.0;
        positions.insert(diagram.nodes[aid].label.clone(), (x, y, ACTOR_WIDTH, ACTOR_HEIGHT));
    }

    // --- Step 4: Ensure non-negative positions ---
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    for (_, (x, y, _, _)) in &positions {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
    }
    if min_x < PADDING || min_y < PADDING {
        let dx = if min_x < PADDING { PADDING - min_x } else { 0.0 };
        let dy = if min_y < PADDING { PADDING - min_y } else { 0.0 };
        for (_, pos) in positions.iter_mut() {
            pos.0 += dx;
            pos.1 += dy;
        }
    }

    Layout { positions }
}

/// Simple topological sort for a subset of nodes
fn topo_sort_subset(nodes: &[usize], edges: &[(usize, usize)]) -> Vec<usize> {
    let node_set: std::collections::HashSet<usize> = nodes.iter().copied().collect();
    let mut in_degree: HashMap<usize, usize> = HashMap::new();
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();

    for &n in nodes {
        in_degree.insert(n, 0);
    }

    for &(from, to) in edges {
        if node_set.contains(&from) && node_set.contains(&to) {
            adj.entry(from).or_default().push(to);
            *in_degree.entry(to).or_insert(0) += 1;
        }
    }

    let mut queue: Vec<usize> = nodes.iter()
        .filter(|&&n| *in_degree.get(&n).unwrap_or(&0) == 0)
        .copied().collect();
    // Stable sort: process in declaration order
    queue.sort_by_key(|n| nodes.iter().position(|&x| x == *n).unwrap_or(0));

    let mut result = Vec::new();
    while let Some(n) = queue.first().copied() {
        queue.remove(0);
        result.push(n);
        if let Some(neighbors) = adj.get(&n) {
            for &next in neighbors {
                if let Some(deg) = in_degree.get_mut(&next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(next);
                    }
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Package bounds from positioned usecases
// ---------------------------------------------------------------------------

fn compute_package_bounds(
    diagram: &Diagram,
    positions: &HashMap<String, (f64, f64, f64, f64)>,
) -> Vec<(String, f64, f64, f64, f64)> {
    let mut bounds = Vec::new();

    for pkg_name in &diagram.packages {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut has_children = false;

        for node in &diagram.nodes {
            if node.package.as_deref() == Some(pkg_name) {
                if let Some(&(x, y, w, h)) = positions.get(&node.label) {
                    has_children = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + w);
                    max_y = max_y.max(y + h);
                }
            }
        }

        if has_children {
            let bx = min_x - PKG_PADDING;
            let by = min_y - PKG_PADDING - PKG_HEADER_H;
            let bw = (max_x - min_x) + PKG_PADDING * 2.0;
            let bh = (max_y - min_y) + PKG_PADDING * 2.0 + PKG_HEADER_H;
            bounds.push((pkg_name.clone(), bx, by, bw, bh));
        }
    }

    bounds
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let layout = compute_layout(diagram);
    let positions = &layout.positions;

    let pkg_bounds = compute_package_bounds(diagram, positions);

    // SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (_, (x, y, w, h)) in positions {
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }
    for (_, bx, by, bw, bh) in &pkg_bounds {
        max_x = max_x.max(bx + bw);
        max_y = max_y.max(by + bh);
    }
    let svg_width = max_x + PADDING;
    let svg_height = max_y + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 14px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Render package rectangles
    for (name, bx, by, bw, bh) in &pkg_bounds {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-dasharray=\"5,5\" rx=\"5\"/>",
            bx, by, bw, bh, COLOR_MID
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
            bx + 5.0, by + 15.0,
            mdd_layout::text::escape_xml(name)
        ));
    }

    // Render edges
    for (from, to) in &diagram.edges {
        let from_node = &diagram.nodes[*from];
        let to_node = &diagram.nodes[*to];
        let from_pos = positions.get(&from_node.label);
        let to_pos = positions.get(&to_node.label);
        if from_pos.is_none() || to_pos.is_none() {
            continue;
        }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        // Clip to ellipse for usecases, rect for actors
        let (ax1, ay1) = clip_to_node(cx1, cy1, cx2, cy2, fw, fh, &from_node.kind);
        let (ax2, ay2) = clip_to_node(cx2, cy2, cx1, cy1, tw, th, &to_node.kind);

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            ax1, ay1, ax2, ay2, COLOR_MID
        ));
    }

    // Render nodes
    for node in &diagram.nodes {
        if let Some(&(x, y, _w, _h)) = positions.get(&node.label) {
            match node.kind {
                NodeKind::Actor => render_actor(&mut svg, x, y, &node.label),
                NodeKind::Usecase => render_usecase(&mut svg, x, y, &node.label),
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn clip_to_node(cx: f64, cy: f64, tx: f64, ty: f64, w: f64, h: f64, kind: &NodeKind) -> (f64, f64) {
    match kind {
        NodeKind::Actor => {
            // Clip to rect
            mdd_layout::edge::clip_to_rect(cx, cy, tx, ty, w / 2.0, h / 2.0)
        }
        NodeKind::Usecase => {
            // Clip to ellipse
            let dx = tx - cx;
            let dy = ty - cy;
            if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
                return (cx, cy + h / 2.0);
            }
            let rx = w / 2.0;
            let ry = h / 2.0;
            let angle = dy.atan2(dx);
            (cx + rx * angle.cos(), cy + ry * angle.sin())
        }
    }
}

fn render_actor(svg: &mut String, x: f64, y: f64, label: &str) {
    let cx = x + ACTOR_WIDTH / 2.0;
    svg.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 12.0, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 22.0, cx, y + 45.0, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx - 15.0, y + 32.0, cx + 15.0, y + 32.0, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 45.0, cx - 12.0, y + 60.0, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 45.0, cx + 12.0, y + 60.0, COLOR_DARK
    ));
    let lines = wrap_lines(label);
    let start_y = y + 75.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx, start_y + i as f64 * LINE_HEIGHT,
            mdd_layout::text::escape_xml(line)
        ));
    }
}

fn render_usecase(svg: &mut String, x: f64, y: f64, label: &str) {
    let (w, h) = usecase_size(label);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    svg.push_str(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx, cy, w / 2.0, h / 2.0, COLOR_FILL, COLOR_DARK
    ));

    let lines = wrap_lines(label);
    let total_text_height = lines.len() as f64 * LINE_HEIGHT;
    let text_start_y = cy - total_text_height / 2.0 + LINE_HEIGHT * 0.7;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx, text_start_y + i as f64 * LINE_HEIGHT,
            mdd_layout::text::escape_xml(line)
        ));
    }
}

const HELP: &str = "\
mdd-usecase - Render a use-case diagram as SVG

Usage: mdd-usecase < input.usecase

Define actors with \"actor Name\" and use cases with \"usecase Name\".
Connect them with \"Name -> Name\". Group use cases in a package
with \"package Name { ... }\".

Example:
  actor Customer
  usecase Login
  Customer -> Login
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
            eprintln!("mdd-usecase: {}", e);
            std::process::exit(1);
        }
    };

    let svg = render_svg(&diagram);
    print!("{}", svg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_diagram() {
        let input = r#"
actor Customer
usecase Login
Customer -> Login
"#;
        let diagram = parse(input).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn parse_with_package() {
        let input = r#"
actor Admin

package "Auth" {
  usecase Login
  usecase Logout
}

Admin -> Login
Admin -> Logout
"#;
        let diagram = parse(input).unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        assert!(diagram.nodes[1].package.is_some());
        assert_eq!(diagram.nodes[1].package.as_deref(), Some("Auth"));
        assert_eq!(diagram.edges.len(), 2);
    }

    #[test]
    fn render_produces_svg() {
        let input = "actor A\nusecase B\nA -> B\n";
        let diagram = parse(input).unwrap();
        let svg = render_svg(&diagram);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn wrap_camel_case() {
        let lines = wrap_lines("RescheduleAppointment");
        assert_eq!(lines, vec!["Reschedule", "Appointment"]);
    }

    #[test]
    fn wrap_short_label() {
        let lines = wrap_lines("Login");
        assert_eq!(lines, vec!["Login"]);
    }

    #[test]
    fn wrap_multiple_words() {
        let lines = wrap_lines("SubmitInsuranceClaim");
        assert_eq!(lines, vec!["Submit", "Insurance", "Claim"]);
    }

    #[test]
    fn actors_split_left_right() {
        let input = "actor A\nactor B\nactor C\nactor D\nusecase U1\nusecase U2\nA -> U1\nB -> U1\nC -> U2\nD -> U2\n";
        let diagram = parse(input).unwrap();
        let layout = compute_layout(&diagram);
        // All actors should have positions
        assert!(layout.positions.contains_key("A"));
        assert!(layout.positions.contains_key("D"));
        // At least one actor should be on each side of usecases
        let u1_x = layout.positions.get("U1").unwrap().0;
        let has_left = ["A", "B", "C", "D"].iter().any(|&a| layout.positions.get(a).unwrap().0 < u1_x);
        let has_right = ["A", "B", "C", "D"].iter().any(|&a| layout.positions.get(a).unwrap().0 > u1_x);
        assert!(has_left);
        assert!(has_right);
    }
}
