use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Node {
    name: String,
}

#[derive(Debug)]
enum LinkKind {
    Directed,
    Undirected,
}

#[derive(Debug)]
struct Link {
    from: String,
    to: String,
    kind: LinkKind,
    label: String,
}

#[derive(Debug)]
struct Concept {
    nodes: Vec<Node>,
    links: Vec<Link>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Concept, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut links: Vec<Link> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // node Name
        if trimmed.starts_with("node ") {
            let name = trimmed.strip_prefix("node ").unwrap().trim().to_string();
            if name.is_empty() {
                return Err("Empty node name".to_string());
            }
            nodes.push(Node { name });
            continue;
        }

        // link A -> B : "label" or link A -- B : "label"
        if trimmed.starts_with("link ") {
            let rest = trimmed.strip_prefix("link ").unwrap().trim();
            let link = parse_link(rest)?;
            links.push(link);
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if nodes.len() < 2 {
        return Err("At least 2 nodes are required".to_string());
    }

    // Validate that all link references exist
    let node_names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    for link in &links {
        if !node_names.contains(&link.from.as_str()) {
            return Err(format!("Unknown node in link: {}", link.from));
        }
        if !node_names.contains(&link.to.as_str()) {
            return Err(format!("Unknown node in link: {}", link.to));
        }
    }

    Ok(Concept {
        nodes,
        links,
    })
}

fn parse_link(rest: &str) -> Result<Link, String> {
    // Try directed first: A -> B : "label"
    if let Some((before_arrow, after_arrow)) = rest.split_once(" -> ") {
        let from = before_arrow.trim().to_string();
        let (to, label) = parse_link_target(after_arrow.trim())?;
        return Ok(Link {
            from,
            to,
            kind: LinkKind::Directed,
            label,
        });
    }

    // Try undirected: A -- B : "label"
    if let Some((before_dash, after_dash)) = rest.split_once(" -- ") {
        let from = before_dash.trim().to_string();
        let (to, label) = parse_link_target(after_dash.trim())?;
        return Ok(Link {
            from,
            to,
            kind: LinkKind::Undirected,
            label,
        });
    }

    Err(format!("Invalid link syntax: {}", rest))
}

fn parse_link_target(s: &str) -> Result<(String, String), String> {
    if let Some((target, label_part)) = s.split_once(" : ") {
        let label = strip_quotes(label_part.trim()).to_string();
        Ok((target.trim().to_string(), label))
    } else {
        Ok((s.trim().to_string(), String::new()))
    }
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

const ORBIT_RADIUS: f64 = 150.0;
const NODE_H: f64 = 40.0;
const NODE_H_PAD: f64 = 16.0;
const MIN_NODE_W: f64 = 80.0;
const PADDING: f64 = 60.0;
const EDGE_COLOR: &str = "#666";
const EDGE_LABEL_FONT: f64 = 11.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"),
    ("#e8f5e9", "#2e7d32"),
    ("#fff8e1", "#f57f17"),
    ("#f3e5f5", "#7b1fa2"),
    ("#e0f2f1", "#00695c"),
    ("#fce4ec", "#c62828"),
    ("#e8eaf6", "#283593"),
    ("#fff3e0", "#e65100"),
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

fn render_svg(concept: &Concept) -> String {
    let n = concept.nodes.len();

    // Compute node widths
    let node_widths: Vec<f64> = concept
        .nodes
        .iter()
        .map(|node| (text_width(&node.name) + NODE_H_PAD * 2.0).max(MIN_NODE_W))
        .collect();

    // Find max node width for bounding box calculation
    let max_node_w = node_widths.iter().cloned().fold(0.0_f64, f64::max);

    // Center of the circular layout
    let cx = PADDING + ORBIT_RADIUS + max_node_w / 2.0;
    let cy = PADDING + ORBIT_RADIUS + NODE_H / 2.0;

    let total_w = cx * 2.0;
    let total_h = cy + ORBIT_RADIUS + NODE_H / 2.0 + PADDING;

    // Compute node positions (circular layout)
    let positions: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let angle = -std::f64::consts::FRAC_PI_2
                + 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            let x = cx + ORBIT_RADIUS * angle.cos();
            let y = cy + ORBIT_RADIUS * angle.sin();
            (x, y)
        })
        .collect();

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Arrow marker defs
    svg.push_str("<defs>");
    svg.push_str(&format!(
        "<marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/>\
         </marker>",
        EDGE_COLOR
    ));
    svg.push_str("</defs>");

    // Build node index lookup
    let node_index = |name: &str| -> Option<usize> {
        concept.nodes.iter().position(|n| n.name == name)
    };

    // Draw edges (before nodes so nodes appear on top)
    for link in &concept.links {
        let from_idx = match node_index(&link.from) {
            Some(i) => i,
            None => continue,
        };
        let to_idx = match node_index(&link.to) {
            Some(i) => i,
            None => continue,
        };

        let (x1, y1) = positions[from_idx];
        let (x2, y2) = positions[to_idx];

        let marker = match link.kind {
            LinkKind::Directed => " marker-end=\"url(#arrow)\"",
            LinkKind::Undirected => "",
        };

        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>",
            x1, y1, x2, y2, EDGE_COLOR, marker
        ));

        // Edge label at midpoint
        if !link.label.is_empty() {
            let mx = (x1 + x2) / 2.0;
            let my = (y1 + y2) / 2.0;
            let lw = text_width(&link.label) + 8.0;
            let lh = EDGE_LABEL_FONT + 8.0;

            // White background rect
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"white\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                mx - lw / 2.0,
                my - lh / 2.0,
                lw,
                lh,
                EDGE_COLOR
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\">{}</text>",
                mx,
                my + EDGE_LABEL_FONT / 2.0 - 1.5,
                EDGE_LABEL_FONT,
                escape_xml(&link.label)
            ));
        }
    }

    // Draw nodes (on top of edges)
    for (i, node) in concept.nodes.iter().enumerate() {
        let (x, y) = positions[i];
        let w = node_widths[i];
        let (bg, border) = COLORS[i % COLORS.len()];

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x - w / 2.0,
            y - NODE_H / 2.0,
            w,
            NODE_H,
            bg,
            border
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            x,
            y + 5.0,
            escape_xml(&node.name)
        ));
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-concept - Render a concept map as SVG

Usage: mdd-concept < input.concept

Define nodes and links between them. Nodes are laid out in a
circle. Links can be directed (->) or undirected (--) with
an optional label.

Example:
  node Design
  node Code
  node Test
  link Design -> Code : \"spec\"
  link Code -> Test : \"build\"
  link Test -> Design : \"feedback\"
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

    let concept = match parse(&input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mdd-concept: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&concept));
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
link A -- B : "関係"
"#;
        let c = parse(input).unwrap();
        assert_eq!(c.nodes.len(), 2);
        assert_eq!(c.nodes[0].name, "A");
        assert_eq!(c.nodes[1].name, "B");
        assert_eq!(c.links.len(), 1);
        assert_eq!(c.links[0].label, "関係");
    }

    #[test]
    fn parse_directed() {
        let input = r#"
node X
node Y
link X -> Y : "arrow"
"#;
        let c = parse(input).unwrap();
        assert_eq!(c.links.len(), 1);
        assert!(matches!(c.links[0].kind, LinkKind::Directed));
        assert_eq!(c.links[0].from, "X");
        assert_eq!(c.links[0].to, "Y");
        assert_eq!(c.links[0].label, "arrow");
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
node A
node B
link A -> B : "test"
"#;
        let c = parse(input).unwrap();
        let svg = render_svg(&c);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn parse_requires_two_nodes() {
        let input = "node A\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_unknown_node_in_link() {
        let input = r#"
node A
node B
link A -> C : "bad"
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_no_label() {
        let input = r#"
node A
node B
link A -> B
"#;
        let c = parse(input).unwrap();
        assert_eq!(c.links[0].label, "");
    }
}
