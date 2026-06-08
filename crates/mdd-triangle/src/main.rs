use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Edge {
    from: usize,
    to: usize,
    label: Option<String>,
}

#[derive(Debug)]
struct Triangle {
    nodes: Vec<String>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Triangle, String> {
    let mut nodes: Vec<String> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // node Label
        if trimmed.starts_with("node ") {
            let rest = trimmed.strip_prefix("node ").unwrap().trim();
            nodes.push(rest.to_string());
            continue;
        }

        // edge 0 -- 1 : "Label"
        if trimmed.starts_with("edge ") {
            let rest = trimmed.strip_prefix("edge ").unwrap().trim();
            let edge = parse_edge(rest)?;
            edges.push(edge);
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if nodes.len() != 3 {
        return Err(format!(
            "Expected exactly 3 nodes, got {}",
            nodes.len()
        ));
    }

    // Validate edge indices
    for edge in &edges {
        if edge.from >= 3 || edge.to >= 3 {
            return Err(format!(
                "Edge index out of range: {} -- {}",
                edge.from, edge.to
            ));
        }
    }

    Ok(Triangle {
        nodes,
        edges,
    })
}

fn parse_edge(s: &str) -> Result<Edge, String> {
    // Format: 0 -- 1 : "Label"  or  0 -- 1
    let colon_pos = s.find(" : ");
    let (indices_part, label) = if let Some(pos) = colon_pos {
        let label_part = s[pos + 3..].trim();
        let label = strip_quotes(label_part).to_string();
        (&s[..pos], Some(label))
    } else {
        (s, None)
    };

    let dash_pos = indices_part
        .find(" -- ")
        .ok_or_else(|| format!("Expected '--' in edge: {}", s))?;

    let from: usize = indices_part[..dash_pos]
        .trim()
        .parse()
        .map_err(|_| format!("Invalid edge index in: {}", s))?;
    let to: usize = indices_part[dash_pos + 4..]
        .trim()
        .parse()
        .map_err(|_| format!("Invalid edge index in: {}", s))?;

    Ok(Edge { from, to, label })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const TRI_SIZE: f64 = 300.0;
const NODE_WIDTH: f64 = 120.0;
const NODE_HEIGHT: f64 = 44.0;
const NODE_H_PAD: f64 = 16.0;
const PADDING: f64 = 60.0;
const EDGE_LABEL_FONT: f64 = 11.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"),
    ("#e8f5e9", "#2e7d32"),
    ("#fff8e1", "#f57f17"),
];

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(tri: &Triangle) -> String {
    // Compute node widths based on text
    let node_widths: Vec<f64> = tri
        .nodes
        .iter()
        .map(|n| (text_width(n) + NODE_H_PAD * 2.0).max(NODE_WIDTH))
        .collect();

    let max_node_w = node_widths.iter().cloned().fold(NODE_WIDTH, f64::max);

    // Triangle vertices (equilateral, pointing up)
    let tri_h = TRI_SIZE * (3.0_f64).sqrt() / 2.0;
    let cx = PADDING + TRI_SIZE / 2.0 + max_node_w / 2.0;
    let cy = PADDING + tri_h / 2.0 + NODE_HEIGHT / 2.0;

    // Top vertex
    let top_x = cx;
    let top_y = cy - tri_h / 2.0;

    // Bottom-left vertex
    let bl_x = cx - TRI_SIZE / 2.0;
    let bl_y = cy + tri_h / 2.0;

    // Bottom-right vertex
    let br_x = cx + TRI_SIZE / 2.0;
    let br_y = cy + tri_h / 2.0;

    let positions = [(top_x, top_y), (bl_x, bl_y), (br_x, br_y)];

    let total_w = cx + TRI_SIZE / 2.0 + max_node_w / 2.0 + PADDING;
    let total_h = cy + tri_h / 2.0 + NODE_HEIGHT / 2.0 + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Edges (draw before nodes so nodes appear on top)
    for edge in &tri.edges {
        let (x1, y1) = positions[edge.from];
        let (x2, y2) = positions[edge.to];

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            x1, y1, x2, y2, "#999"
        ));

        // Edge label at midpoint
        if let Some(ref label) = edge.label {
            let mid_x = (x1 + x2) / 2.0;
            let mid_y = (y1 + y2) / 2.0;

            // Offset label slightly away from the center of the triangle
            let offset_x = mid_x - cx;
            let offset_y = mid_y - cy;
            let dist = (offset_x * offset_x + offset_y * offset_y).sqrt();
            let norm = if dist > 0.0 { 14.0 / dist } else { 0.0 };
            let label_x = mid_x + offset_x * norm;
            let label_y = mid_y + offset_y * norm;

            let lw = text_width(label) + 8.0;
            let lh = EDGE_LABEL_FONT + 8.0;
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"white\" fill-opacity=\"0.85\"/>",
                label_x - lw / 2.0,
                label_y - lh / 2.0,
                lw,
                lh
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                label_x,
                label_y + EDGE_LABEL_FONT / 2.0 - 2.0,
                EDGE_LABEL_FONT,
                "#666",
                escape_xml(label)
            ));
        }
    }

    // Nodes
    for (i, node_label) in tri.nodes.iter().enumerate() {
        let (nx, ny) = positions[i];
        let nw = node_widths[i];
        let (bg, fg) = COLORS[i % COLORS.len()];

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            nx - nw / 2.0,
            ny - NODE_HEIGHT / 2.0,
            nw,
            NODE_HEIGHT,
            bg,
            fg
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            nx,
            ny + FONT_SIZE / 2.0 - 2.0,
            fg,
            escape_xml(node_label)
        ));
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let tri = match parse(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("mdd-triangle: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&tri));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = r#"
node A
node B
node C
"#;
        let t = parse(input).unwrap();
        assert_eq!(t.nodes.len(), 3);
        assert_eq!(t.nodes[0], "A");
        assert_eq!(t.nodes[1], "B");
        assert_eq!(t.nodes[2], "C");
        assert!(t.edges.is_empty());
    }

    #[test]
    fn parse_with_edges() {
        let input = r#"
node X
node Y
node Z
edge 0 -- 1 : "alpha"
edge 1 -- 2 : "beta"
edge 0 -- 2 : "gamma"
"#;
        let t = parse(input).unwrap();
        assert_eq!(t.edges.len(), 3);
        assert_eq!(t.edges[0].from, 0);
        assert_eq!(t.edges[0].to, 1);
        assert_eq!(t.edges[0].label.as_deref(), Some("alpha"));
        assert_eq!(t.edges[2].label.as_deref(), Some("gamma"));
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
node A
node B
node C
edge 0 -- 1 : "test"
"#;
        let t = parse(input).unwrap();
        let svg = render_svg(&t);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_rejects_wrong_node_count() {
        let input = "node A\nnode B\n";
        assert!(parse(input).is_err());

        let input = "node A\nnode B\nnode C\nnode D\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_rejects_invalid_edge_index() {
        let input = "node A\nnode B\nnode C\nedge 0 -- 5 : \"x\"\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn edge_without_label() {
        let input = "node A\nnode B\nnode C\nedge 0 -- 1\n";
        let t = parse(input).unwrap();
        assert_eq!(t.edges.len(), 1);
        assert!(t.edges[0].label.is_none());
    }
}
