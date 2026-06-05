use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

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

    let mut in_datastore = false;
    let mut ds_name = String::new();
    let mut ds_columns: Vec<String> = Vec::new();

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
                in_datastore = false;
                ds_name.clear();
                ds_columns.clear();
                continue;
            }
            ds_columns.push(line.to_string());
            continue;
        }

        if line.starts_with("entity ") {
            let label = line.strip_prefix("entity ").unwrap().trim().to_string();
            let id = nodes.len();
            name_to_id.insert(label.clone(), id);
            nodes.push(Node {
                label,
                kind: NodeKind::Entity,
            });
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
            continue;
        }

        if line.contains(" -> ") {
            // Parse: From -> To : "label"  or  From -> To
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from = parts[0].trim();
            let rest = parts[1];

            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim(), String::new())
            };

            let from_id = name_to_id
                .get(from)
                .ok_or_else(|| format!("Unknown node: {}", from))?;
            let to_id = name_to_id
                .get(to)
                .ok_or_else(|| format!("Unknown node: {}", to))?;
            edges.push(Edge {
                from: *from_id,
                to: *to_id,
                label,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_datastore {
        return Err(format!("Unclosed datastore block: {}", ds_name));
    }

    Ok(Diagram { nodes, edges })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
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
    // Three-tier scaling:
    // <=10:  factor ~1.0-1.3  (compact)
    // 10-30: factor ~1.3-2.5  (moderate)
    // 30+:   factor ~2.5-6.0  (spacious, linear growth)
    let factor = if complexity <= 10 {
        1.0 + (complexity as f64 / 20.0).sqrt() * 0.4
    } else if complexity <= 30 {
        1.0 + (complexity as f64 / 10.0).sqrt() * 0.6
    } else {
        2.0 + (complexity - 30) as f64 * 0.06
    }
    .min(5.0);

    let node_count = diagram.nodes.len() as f64;
    SpacingConfig {
        nodesep: 22.0 * factor,
        ranksep: 35.0 * factor,
        component_gap: 28.0 * factor,
        vertex_spacing: 5.0 + node_count * 3.5,
    }
}

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

fn text_width(s: &str) -> f64 {
    // Approximate: ASCII ~8px, CJK ~14px per char
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                CHAR_WIDTH
            } else {
                14.0
            }
        })
        .sum()
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
/// Returns (num_columns, col_widths, num_rows).
fn ds_column_layout(columns: &[String]) -> (usize, Vec<f64>, usize) {
    if columns.is_empty() {
        return (1, vec![0.0], 0);
    }

    // Determine number of display columns: ceil(len / DS_MAX_ROWS)
    let num_cols = ((columns.len() + DS_MAX_ROWS - 1) / DS_MAX_ROWS).max(1);
    let num_rows = (columns.len() + num_cols - 1) / num_cols;

    // Compute max width per display column
    let mut col_widths = vec![0.0_f64; num_cols];
    for (i, col) in columns.iter().enumerate() {
        let c = i / num_rows; // fill column-first
        col_widths[c] = col_widths[c].max(text_width(col));
    }

    (num_cols, col_widths, num_rows)
}

fn datastore_size(label: &str, columns: &[String]) -> (f64, f64) {
    let header_w = text_width(label) + DS_H_PAD * 2.0;
    let (num_cols, col_widths, num_rows) = ds_column_layout(columns);

    let inner_w: f64 = col_widths.iter().sum::<f64>()
        + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let w = header_w
        .max(inner_w + DS_H_PAD * 2.0)
        .max(DS_MIN_W);
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
// Layout & SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let sp = compute_spacing(diagram);

    let config = Config {
        vertex_spacing: sp.vertex_spacing,
        ..Config::default()
    };

    // Build vertices with swapped dimensions for LTR layout
    let vertices: Vec<(u32, (f64, f64))> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (w, h) = node_size(node);
            (i as u32, (h, w)) // swap for LTR
        })
        .collect();

    let edges: Vec<(u32, u32)> = diagram
        .edges
        .iter()
        .map(|e| (e.from as u32, e.to as u32))
        .collect();

    let layouts = from_vertices_and_edges(&vertices, &edges, &config);

    // Post-scale for asymmetric nodesep/ranksep
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
                let new_sx = cx + (sx - cx) * nodesep_ratio;
                let new_sy = cy + (sy - cy) * ranksep_ratio;
                let final_x = new_sy; // swap back for LTR
                let final_y = new_sx;

                let (w, h) = node_size(&diagram.nodes[id]);
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
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = node_size(node);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING * 2.0;
    let svg_height = max_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );

    // Styles & arrow marker
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render nodes first (behind edges)
    for (i, node) in diagram.nodes.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let px = PADDING + x;
        let py = PADDING + y;

        match &node.kind {
            NodeKind::Entity => render_entity(&mut svg, px, py, &node.label),
            NodeKind::Process => render_process(&mut svg, px, py, &node.label),
            NodeKind::DataStore { columns } => {
                render_datastore(&mut svg, px, py, &node.label, columns)
            }
        }
    }

    // Render edges on top of nodes so they remain visible
    for edge in &diagram.edges {
        let (x1, y1) = positions.get(&edge.from).copied().unwrap_or((0.0, 0.0));
        let (x2, y2) = positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = node_size(&diagram.nodes[edge.from]);
        let (tw, th) = node_size(&diagram.nodes[edge.to]);

        let cx1 = PADDING + x1 + fw / 2.0;
        let cy1 = PADDING + y1 + fh / 2.0;
        let cx2 = PADDING + x2 + tw / 2.0;
        let cy2 = PADDING + y2 + th / 2.0;

        let (ax1, ay1) = clip_to_boundary(cx1, cy1, cx2, cy2, &diagram.nodes[edge.from]);
        let (ax2, ay2) = clip_to_boundary(cx2, cy2, cx1, cy1, &diagram.nodes[edge.to]);

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            ax1, ay1, ax2, ay2, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            let lx = (ax1 + ax2) / 2.0;
            let ly = (ay1 + ay2) / 2.0 - 6.0;
            // White background behind label for readability
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.85\"/>",
                lx - text_width(&edge.label) / 2.0 - 3.0,
                ly - 12.0,
                text_width(&edge.label) + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                lx, ly, COLOR_EDGE, escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Clip a line from (cx, cy) toward (tx, ty) to the boundary of the node shape.
fn clip_to_boundary(cx: f64, cy: f64, tx: f64, ty: f64, node: &Node) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;

    match &node.kind {
        NodeKind::Process => {
            let (w, _) = process_size(&node.label);
            let r = w / 2.0;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist == 0.0 {
                (cx, cy)
            } else {
                (cx + dx / dist * r, cy + dy / dist * r)
            }
        }
        _ => {
            // Rectangle clipping for Entity and DataStore
            let (w, h) = node_size(node);
            let hw = w / 2.0;
            let hh = h / 2.0;
            if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
                return (cx, cy);
            }
            // Find intersection with rectangle edges
            let mut t = f64::MAX;
            if dx.abs() > 1e-9 {
                let t_right = hw / dx.abs();
                t = t.min(t_right);
            }
            if dy.abs() > 1e-9 {
                let t_bottom = hh / dy.abs();
                t = t.min(t_bottom);
            }
            (cx + dx * t, cy + dy * t)
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
    let inner_w: f64 = col_widths.iter().sum::<f64>()
        + (num_cols as f64 - 1.0).max(0.0) * DS_COL_GAP;
    let grid_start_x = x + (w - inner_w) / 2.0;

    for (i, col) in columns.iter().enumerate() {
        let display_col = i / num_rows; // fill column-first
        let display_row = i % num_rows;

        let col_x: f64 = col_widths[..display_col].iter().sum::<f64>()
            + display_col as f64 * DS_COL_GAP;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
            grid_start_x + col_x,
            y + DS_HEADER_H + (display_row as f64 + 0.75) * LINE_HEIGHT,
            "#555",
            escape_xml(col)
        ));
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
    fn render_produces_svg() {
        let input = "entity A\nprocess B\nA -> B : \"test\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("marker"));
    }

    #[test]
    fn spacing_scales_with_complexity() {
        let small = parse("entity A\nprocess B\nA -> B\n").unwrap();
        let small_sp = compute_spacing(&small);

        let big_input = "entity A\nentity B\nprocess C\nprocess D\nprocess E\n\
                         A -> C\nA -> D\nB -> E\nC -> D\nD -> E\n";
        let big = parse(big_input).unwrap();
        let big_sp = compute_spacing(&big);

        assert!(big_sp.nodesep > small_sp.nodesep);
        assert!(big_sp.ranksep > small_sp.ranksep);
    }
}
