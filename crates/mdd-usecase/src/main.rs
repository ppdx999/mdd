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

/// Calculate the usecase ellipse size based on the label
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

fn node_size(node: &Node) -> (f64, f64) {
    match node.kind {
        NodeKind::Actor => (ACTOR_WIDTH, ACTOR_HEIGHT),
        NodeKind::Usecase => usecase_size(&node.label),
    }
}

// ---------------------------------------------------------------------------
// Build LayoutGraph from parsed Diagram
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> mdd_layout::LayoutGraph {
    let mut graph = mdd_layout::LayoutGraph::new();

    // Add all nodes
    for node in &diagram.nodes {
        let (w, h) = node_size(node);
        graph.nodes.push(mdd_layout::LayoutNode {
            name: node.label.clone(),
            width: w,
            height: h,
        });
    }

    // Build package name -> group index mapping
    // Each package becomes a LayoutGroup
    let mut pkg_to_group: HashMap<String, usize> = HashMap::new();
    for pkg_name in &diagram.packages {
        let gidx = graph.groups.len();
        pkg_to_group.insert(pkg_name.clone(), gidx);
        graph.groups.push(mdd_layout::LayoutGroup {
            name: pkg_name.clone(),
            children: Vec::new(),
        });
    }

    // Assign nodes to groups or top_level
    for (i, node) in diagram.nodes.iter().enumerate() {
        if let Some(ref pkg) = node.package {
            if let Some(&gidx) = pkg_to_group.get(pkg) {
                graph.groups[gidx]
                    .children
                    .push(mdd_layout::LayoutElement::NodeRef(i));
            }
        } else {
            graph.top_level.push(mdd_layout::LayoutElement::NodeRef(i));
        }
    }

    // Add groups to top_level
    for pkg_name in &diagram.packages {
        if let Some(&gidx) = pkg_to_group.get(pkg_name) {
            graph
                .top_level
                .push(mdd_layout::LayoutElement::GroupRef(gidx));
        }
    }

    // Add edges (using node labels as names)
    for (from, to) in &diagram.edges {
        graph.edges.push(mdd_layout::LayoutEdge {
            from: diagram.nodes[*from].label.clone(),
            to: diagram.nodes[*to].label.clone(),
            label: String::new(),
        });
    }

    graph
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);
    let config = mdd_layout::LayoutConfig {
        padding: PADDING,
        group_h_pad: 20.0,
        group_v_pad: 16.0,
        group_header_h: 24.0,
        default_node_w: MIN_NODE_WIDTH,
        default_node_h: MIN_NODE_HEIGHT,
        ..mdd_layout::LayoutConfig::default()
    };
    let result = mdd_layout::layout(&graph, &config);
    let positions = &result.positions;

    // SVG dimensions
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
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 14px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Render package rectangles (groups)
    for pkg_name in &diagram.packages {
        if let Some(&(x, y, w, h)) = positions.get(pkg_name) {
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-dasharray=\"5,5\" rx=\"5\"/>",
                x, y, w, h, COLOR_MID
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
                x + 5.0,
                y + 15.0,
                mdd_layout::text::escape_xml(pkg_name)
            ));
        }
    }

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

        // Use edge waypoints if available, else route around nodes
        let edge_key = format!("{}→{}", from_node.label, to_node.label);
        let route = if let Some(waypoints) = result.edge_waypoints.get(&edge_key) {
            let mut r = vec![(cx1, cy1)];
            r.extend(waypoints);
            r.push((cx2, cy2));
            r
        } else {
            mdd_layout::edge::route_around_nodes(
                cx1,
                cy1,
                cx2,
                cy2,
                &from_node.label,
                &to_node.label,
                &all_bounds,
                0.0,
            )
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
        let (ax1, ay1) = mdd_layout::edge::clip_to_rect(
            cx1,
            cy1,
            start_target.0,
            start_target.1,
            fw / 2.0,
            fh / 2.0,
        );
        let (ax2, ay2) = mdd_layout::edge::clip_to_rect(
            cx2,
            cy2,
            end_target.0,
            end_target.1,
            tw / 2.0,
            th / 2.0,
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
            mdd_layout::edge::build_smooth_path(&clipped)
        };

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            ax1, ay1, ax2, ay2, COLOR_MID
        ));
        // Also render smooth path for multi-segment edges
        if clipped.len() > 2 {
            svg.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                path_d, COLOR_MID
            ));
        }
    }

    // Render nodes
    for (i, node) in diagram.nodes.iter().enumerate() {
        if let Some(&(x, y, _w, _h)) = positions.get(&node.label) {
            match node.kind {
                NodeKind::Actor => render_actor(&mut svg, x, y, &node.label),
                NodeKind::Usecase => render_usecase(&mut svg, x, y, &node.label),
            }
        } else {
            // fallback: should not happen with layout
            let _ = i;
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_actor(svg: &mut String, x: f64, y: f64, label: &str) {
    let cx = x + ACTOR_WIDTH / 2.0;
    // Head
    svg.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx,
        y + 12.0,
        COLOR_DARK
    ));
    // Body
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx,
        y + 22.0,
        cx,
        y + 45.0,
        COLOR_DARK
    ));
    // Arms
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx - 15.0,
        y + 32.0,
        cx + 15.0,
        y + 32.0,
        COLOR_DARK
    ));
    // Legs
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx,
        y + 45.0,
        cx - 12.0,
        y + 60.0,
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx,
        y + 45.0,
        cx + 12.0,
        y + 60.0,
        COLOR_DARK
    ));
    // Label
    let lines = wrap_lines(label);
    let start_y = y + 75.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx,
            start_y + i as f64 * LINE_HEIGHT,
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
        cx,
        cy,
        w / 2.0,
        h / 2.0,
        COLOR_FILL,
        COLOR_DARK
    ));

    let lines = wrap_lines(label);
    let total_text_height = lines.len() as f64 * LINE_HEIGHT;
    let text_start_y = cy - total_text_height / 2.0 + LINE_HEIGHT * 0.7;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx,
            text_start_y + i as f64 * LINE_HEIGHT,
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
        // "Submit Insurance" (16 chars) > MAX_LINE_CHARS, so splits into 3
        assert_eq!(lines, vec!["Submit", "Insurance", "Claim"]);
    }
}
