use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

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
}

fn parse(input: &str) -> Result<Diagram, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut current_package: Option<String> = None;

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

    Ok(Diagram { nodes, edges })
}

const CHAR_WIDTH: f64 = 8.0;
const MIN_NODE_WIDTH: f64 = 120.0;
const LINE_HEIGHT: f64 = 18.0;
const MIN_NODE_HEIGHT: f64 = 50.0;
const ELLIPSE_H_PAD: f64 = 30.0;
const PADDING: f64 = 40.0;
const ACTOR_WIDTH: f64 = 60.0;
const ACTOR_HEIGHT: f64 = 80.0;
const MAX_LINE_CHARS: usize = 14;

struct SpacingConfig {
    nodesep: f64,
    ranksep: f64,
    component_gap: f64,
    pkg_gap: f64,
    pkg_padding: f64,
    actor_col_width: f64,
    actor_spacing: f64,
    vertex_spacing: f64,
}

fn compute_spacing(diagram: &Diagram) -> SpacingConfig {
    let complexity = diagram.nodes.len() + diagram.edges.len();
    let factor = (1.0 + (complexity as f64 / 10.0).sqrt() * 0.5).min(3.0);

    SpacingConfig {
        nodesep: 30.0 * factor,
        ranksep: 50.0 * factor,
        component_gap: 30.0 * factor,
        pkg_gap: 40.0 * factor,
        pkg_padding: 20.0 * factor,
        actor_col_width: 140.0 * factor.min(1.5),
        actor_spacing: (ACTOR_HEIGHT + 40.0) * factor,
        vertex_spacing: 10.0 + (diagram.nodes.len() as f64) * 3.0,
    }
}

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

/// Calculate the node size based on the label
fn usecase_size(label: &str) -> (f64, f64) {
    let lines = wrap_lines(label);
    let text_width = lines
        .iter()
        .map(|l| {
            l.chars()
                .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
                .sum::<f64>()
        })
        .fold(0.0_f64, f64::max);
    let w = (text_width + ELLIPSE_H_PAD * 2.0).max(MIN_NODE_WIDTH);
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

fn render_svg(diagram: &Diagram) -> String {
    let sp = compute_spacing(diagram);
    let mut positions: HashMap<usize, (f64, f64)> = HashMap::new();

    // Collect unique package names in order of first appearance
    let mut package_names: Vec<String> = Vec::new();
    for node in &diagram.nodes {
        if let Some(ref pkg) = node.package {
            if !package_names.contains(pkg) {
                package_names.push(pkg.clone());
            }
        }
    }

    // Collect actors (nodes without package)
    let actor_ids: Vec<usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.package.is_none())
        .map(|(i, _)| i)
        .collect();

    // Place actors in the left column, evenly spaced vertically
    for (idx, &actor_id) in actor_ids.iter().enumerate() {
        positions.insert(actor_id, (0.0, idx as f64 * sp.actor_spacing));
    }

    // Layout each package independently, then stack them vertically
    let pkg_x_offset = sp.actor_col_width;
    let mut pkg_y_cursor: f64 = 0.0;
    let mut pkg_bounds: Vec<(String, f64, f64, f64, f64)> = Vec::new(); // (name, x, y, w, h)

    for pkg_name in &package_names {
        // Gather nodes in this package with local indices
        let pkg_nodes: Vec<(usize, &Node)> = diagram
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.package.as_deref() == Some(pkg_name))
            .collect();

        if pkg_nodes.is_empty() {
            continue;
        }

        let config = Config {
            vertex_spacing: sp.vertex_spacing,
            ..Config::default()
        };

        // Map global IDs to local (0-based) IDs for Sugiyama
        let global_to_local: HashMap<usize, u32> = pkg_nodes
            .iter()
            .enumerate()
            .map(|(local, (global, _))| (*global, local as u32))
            .collect();
        let local_to_global: Vec<usize> = pkg_nodes.iter().map(|(g, _)| *g).collect();

        // Vertices with swapped dimensions for LTR layout
        let vertices: Vec<(u32, (f64, f64))> = pkg_nodes
            .iter()
            .enumerate()
            .map(|(local, (_, node))| {
                let (w, h) = node_size(node);
                (local as u32, (h, w)) // swap for LTR
            })
            .collect();

        // Intra-package edges only
        let edges: Vec<(u32, u32)> = diagram
            .edges
            .iter()
            .filter_map(|(from, to)| {
                let lf = global_to_local.get(from)?;
                let lt = global_to_local.get(to)?;
                Some((*lf, *lt))
            })
            .collect();

        if vertices.len() == 1 {
            // Single node, just place it directly
            let global_id = local_to_global[0];
            let (w, h) = node_size(&diagram.nodes[global_id]);
            positions.insert(
                global_id,
                (pkg_x_offset + sp.pkg_padding, pkg_y_cursor + sp.pkg_padding + 20.0),
            );
            let pkg_w = w + sp.pkg_padding * 2.0;
            let pkg_h = h + sp.pkg_padding * 2.0 + 20.0;
            pkg_bounds.push((pkg_name.clone(), pkg_x_offset, pkg_y_cursor, pkg_w, pkg_h));
            pkg_y_cursor += pkg_h + sp.pkg_gap;
        } else {
            let layouts = from_vertices_and_edges(&vertices, &edges, &config);

            // Post-scale coordinates for asymmetric nodesep/ranksep.
            // Sugiyama applies vertex_spacing equally to both axes.
            // We scale each axis independently relative to centroid.
            let base = sp.vertex_spacing.max(1.0);
            let nodesep_ratio = sp.nodesep / base;
            let ranksep_ratio = sp.ranksep / base;

            // Build scaled components with bounding boxes
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
                        // sx = sugiyama rank axis (becomes final Y after LTR swap)
                        // sy = sugiyama within-rank axis (becomes final X after LTR swap)
                        let new_sx = cx + (sx - cx) * nodesep_ratio;
                        let new_sy = cy + (sy - cy) * ranksep_ratio;
                        let final_x = new_sy; // swap back for LTR
                        let final_y = new_sx;

                        let (w, h) = node_size(&diagram.nodes[local_to_global[id]]);
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

            // Row-based shelf packing for disconnected components.
            // Target roughly square aspect ratio.
            let total_area: f64 = scaled_components
                .iter()
                .map(|(_, w, h)| (w + sp.component_gap) * (h + sp.component_gap))
                .sum();
            let target_width = total_area.sqrt() * 1.3;

            // Sort components by height descending for better packing
            let mut comp_indices: Vec<usize> = (0..scaled_components.len()).collect();
            comp_indices.sort_by(|a, b| {
                scaled_components[*b]
                    .2
                    .partial_cmp(&scaled_components[*a].2)
                    .unwrap()
            });

            let mut local_positions: HashMap<usize, (f64, f64)> = HashMap::new();
            let mut row_x: f64 = 0.0;
            let mut row_y: f64 = 0.0;
            let mut row_max_height: f64 = 0.0;

            for &ci in &comp_indices {
                let (ref coords, comp_w, comp_h) = scaled_components[ci];

                // Start new row if this component would exceed target width
                if row_x > 0.0 && row_x + comp_w > target_width {
                    row_y += row_max_height + sp.component_gap;
                    row_x = 0.0;
                    row_max_height = 0.0;
                }

                let cmin_x = coords.values().map(|(x, _)| *x).fold(f64::MAX, f64::min);
                let cmin_y = coords.values().map(|(_, y)| *y).fold(f64::MAX, f64::min);

                for (&id, &(x, y)) in coords {
                    let lx = x - cmin_x + row_x;
                    let ly = y - cmin_y + row_y;
                    local_positions.insert(id, (lx, ly));
                }

                row_x += comp_w + sp.component_gap;
                row_max_height = row_max_height.max(comp_h);
            }

            // Compute bounding box and place with package offset
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x: f64 = 0.0;
            let mut max_y: f64 = 0.0;
            for (local, (lx, ly)) in &local_positions {
                let (w, h) = node_size(&diagram.nodes[local_to_global[*local]]);
                min_x = min_x.min(*lx);
                min_y = min_y.min(*ly);
                max_x = max_x.max(*lx + w);
                max_y = max_y.max(*ly + h);
            }

            let ox = pkg_x_offset + sp.pkg_padding - min_x;
            let oy = pkg_y_cursor + sp.pkg_padding + 20.0 - min_y;
            for (local, (lx, ly)) in &local_positions {
                let global_id = local_to_global[*local];
                positions.insert(global_id, (lx + ox, ly + oy));
            }

            let pkg_w = (max_x - min_x) + sp.pkg_padding * 2.0;
            let pkg_h = (max_y - min_y) + sp.pkg_padding * 2.0 + 20.0;
            pkg_bounds.push((pkg_name.clone(), pkg_x_offset, pkg_y_cursor, pkg_w, pkg_h));
            pkg_y_cursor += pkg_h + sp.pkg_gap;
        }
    }

    // Center actors vertically relative to packages
    let total_pkg_height = if pkg_y_cursor > sp.pkg_gap {
        pkg_y_cursor - sp.pkg_gap
    } else {
        0.0
    };
    let total_actor_height = if actor_ids.is_empty() {
        0.0
    } else {
        (actor_ids.len() - 1) as f64 * sp.actor_spacing + ACTOR_HEIGHT
    };
    let actor_y_offset = (total_pkg_height - total_actor_height).max(0.0) / 2.0;
    for &actor_id in &actor_ids {
        if let Some(pos) = positions.get_mut(&actor_id) {
            pos.1 += actor_y_offset;
        }
    }

    // Calculate SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = node_size(node);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }
    for (_, px, py, pw, ph) in &pkg_bounds {
        max_x = max_x.max(px + pw);
        max_y = max_y.max(py + ph);
    }

    let svg_width = max_x + PADDING * 2.0;
    let svg_height = max_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 14px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Render packages
    for (name, px, py, pw, ph) in &pkg_bounds {
        let rx = PADDING + px;
        let ry = PADDING + py;
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" stroke-dasharray=\"5,5\" rx=\"5\"/>",
            rx, ry, pw, ph, COLOR_MID
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
            rx + 5.0,
            ry + 15.0,
            escape_xml(name)
        ));
    }

    // Render edges
    for (from, to) in &diagram.edges {
        let (x1, y1) = positions.get(from).copied().unwrap_or((0.0, 0.0));
        let (x2, y2) = positions.get(to).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = node_size(&diagram.nodes[*from]);
        let (tw, th) = node_size(&diagram.nodes[*to]);
        let cx1 = PADDING + x1 + fw / 2.0;
        let cy1 = PADDING + y1 + fh / 2.0;
        let cx2 = PADDING + x2 + tw / 2.0;
        let cy2 = PADDING + y2 + th / 2.0;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            cx1, cy1, cx2, cy2, COLOR_MID
        ));
    }

    // Render nodes
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let px = PADDING + x;
        let py = PADDING + y;

        match node.kind {
            NodeKind::Actor => render_actor(&mut svg, px, py, &node.label),
            NodeKind::Usecase => render_usecase(&mut svg, px, py, &node.label),
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
        cx, y + 22.0, cx, y + 45.0, COLOR_DARK
    ));
    // Arms
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx - 15.0, y + 32.0, cx + 15.0, y + 32.0, COLOR_DARK
    ));
    // Legs
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 45.0, cx - 12.0, y + 60.0, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
        cx, y + 45.0, cx + 12.0, y + 60.0, COLOR_DARK
    ));
    // Label (also wrap for actors)
    let lines = wrap_lines(label);
    let start_y = y + 75.0;
    for (i, line) in lines.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            cx,
            start_y + i as f64 * LINE_HEIGHT,
            escape_xml(line)
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
            cx,
            text_start_y + i as f64 * LINE_HEIGHT,
            escape_xml(line)
        ));
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

    #[test]
    fn spacing_scales_with_complexity() {
        let small = parse("actor A\nusecase B\nA -> B\n").unwrap();
        let small_sp = compute_spacing(&small);

        let big_input = "actor A\nusecase B\nusecase C\nusecase D\nusecase E\n\
                         A -> B\nA -> C\nA -> D\nA -> E\nB -> C\nC -> D\n";
        let big = parse(big_input).unwrap();
        let big_sp = compute_spacing(&big);

        assert!(big_sp.nodesep > small_sp.nodesep);
        assert!(big_sp.ranksep > small_sp.ranksep);
        assert!(big_sp.component_gap > small_sp.component_gap);
        assert!(big_sp.pkg_gap > small_sp.pkg_gap);
    }
}
