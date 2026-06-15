use std::collections::HashMap;
use std::io::{self, Read};

use mdd_layout::edge::clip_to_rect;
use mdd_layout::text::{escape_xml, text_width};
use mdd_layout::{ForceConfig, LayoutEdge, LayoutElement, LayoutGraph, LayoutGroup, LayoutNode};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Screen {
    name: String,
    elements: Vec<String>,
}

#[derive(Debug)]
struct Group {
    label: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    ScreenRef(usize),
    GroupRef(usize),
}

#[derive(Debug)]
struct Edge {
    from: String,
    to: String,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    screens: Vec<Screen>,
    groups: Vec<Group>,
    top_level: Vec<Element>,
    edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut screens: Vec<Screen> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut top_level: Vec<Element> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();

    let mut in_screen = false;
    let mut screen_name = String::new();
    let mut screen_elements: Vec<String> = Vec::new();

    // Stack for nested groups: (group_index, children_so_far)
    let mut group_stack: Vec<(usize, Vec<Element>)> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Inside screen block
        if in_screen {
            if line == "}" {
                let id = screens.len();
                name_to_id.insert(screen_name.clone(), id);
                screens.push(Screen {
                    name: screen_name.clone(),
                    elements: screen_elements.clone(),
                });
                let elem = Element::ScreenRef(id);
                if let Some(parent) = group_stack.last_mut() {
                    parent.1.push(elem);
                } else {
                    top_level.push(elem);
                }
                in_screen = false;
                screen_name.clear();
                screen_elements.clear();
                continue;
            }
            screen_elements.push(line.to_string());
            continue;
        }

        // Close group block
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

        // group "Name" {
        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let gidx = groups.len();
                groups.push(Group {
                    label: name,
                    children: Vec::new(),
                });
                group_stack.push((gidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        // screen Name { ... } or screen Name
        if line.starts_with("screen ") {
            let rest = line.strip_prefix("screen ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                screen_name = name.trim().trim_matches('"').to_string();
                screen_elements.clear();
                in_screen = true;
                continue;
            }
            // Single-line screen without elements
            let name = rest.trim().trim_matches('"').to_string();
            let id = screens.len();
            name_to_id.insert(name.clone(), id);
            screens.push(Screen {
                name,
                elements: Vec::new(),
            });
            let elem = Element::ScreenRef(id);
            if let Some(parent) = group_stack.last_mut() {
                parent.1.push(elem);
            } else {
                top_level.push(elem);
            }
            continue;
        }

        // Edge: From -> To : "label"
        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            if parts.len() < 2 {
                return Err(format!("Invalid edge syntax: {}", line));
            }
            let from = parts[0].trim().to_string();
            let rest = parts[1];

            let (to, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim().to_string(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };

            if !name_to_id.contains_key(&from) {
                return Err(format!("Unknown screen: {}", from));
            }
            if !name_to_id.contains_key(&to) {
                return Err(format!("Unknown screen: {}", to));
            }

            edges.push(Edge { from, to, label });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_screen {
        return Err(format!("Unclosed screen block: {}", screen_name));
    }
    if !group_stack.is_empty() {
        return Err("Unclosed group block".to_string());
    }

    Ok(Diagram {
        screens,
        groups,
        top_level,
        edges,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

// Screen card
const SCREEN_H_PAD: f64 = 16.0;
const SCREEN_MIN_W: f64 = 140.0;
const SCREEN_TITLE_H: f64 = 28.0;
const SCREEN_BODY_PAD: f64 = 8.0;
const SCREEN_RADIUS: f64 = 6.0;

// Colors
const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_SCREEN_FILL: &str = "#ffffff";
const COLOR_SCREEN_STROKE: &str = "#1565c0";
const COLOR_TITLE_BG: &str = "#e3f2fd";
const COLOR_TITLE_DOT: &str = "#90caf9";
const COLOR_ELEMENT_TEXT: &str = "#555";
const COLOR_GROUP_STROKE: &str = "#7b1fa2";
const COLOR_GROUP_FILL: &str = "#f3e5f5";

// ---------------------------------------------------------------------------
// Screen sizing
// ---------------------------------------------------------------------------

fn screen_size(screen: &Screen) -> (f64, f64) {
    let title_w = text_width(&screen.name) + SCREEN_H_PAD * 2.0 + 40.0; // extra for dots
    let max_elem_w = screen
        .elements
        .iter()
        .map(|e| text_width(e))
        .fold(0.0_f64, f64::max);
    let body_w = max_elem_w + SCREEN_H_PAD * 2.0;

    let w = title_w.max(body_w).max(SCREEN_MIN_W);

    let body_h = if screen.elements.is_empty() {
        20.0
    } else {
        SCREEN_BODY_PAD + screen.elements.len() as f64 * LINE_HEIGHT + SCREEN_BODY_PAD
    };
    let h = SCREEN_TITLE_H + body_h;

    (w, h)
}

// ---------------------------------------------------------------------------
// Build LayoutGraph
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> LayoutGraph {
    let mut graph = LayoutGraph::new();

    for screen in &diagram.screens {
        let (w, h) = screen_size(screen);
        graph.nodes.push(LayoutNode {
            name: screen.name.clone(),
            width: w,
            height: h,
        });
    }

    for edge in &diagram.edges {
        graph.edges.push(LayoutEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            label: edge.label.clone(),
        });
    }

    for group in &diagram.groups {
        graph.groups.push(LayoutGroup {
            name: group.label.clone(),
            children: convert_elements(&group.children),
        });
    }

    graph.top_level = convert_elements(&diagram.top_level);

    graph
}

fn convert_elements(elements: &[Element]) -> Vec<LayoutElement> {
    elements
        .iter()
        .map(|e| match e {
            Element::ScreenRef(i) => LayoutElement::NodeRef(*i),
            Element::GroupRef(i) => LayoutElement::GroupRef(*i),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);

    let config = ForceConfig {
        padding: PADDING,
        iterations: 500,
        repulsion_strength: 1.5,
        group_padding: 30.0,
        group_header_h: 28.0,
        ..ForceConfig::default()
    };
    let result = mdd_layout::force_layout(&graph, &config);
    let positions = &result.positions;

    // Compute SVG dimensions
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

    // Styles & arrow marker
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs>\
         <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker>\
         </defs>",
        COLOR_EDGE
    ));

    // Render groups and screens
    render_elements_recursive(
        &mut svg,
        &diagram.top_level,
        &diagram.screens,
        &diagram.groups,
        positions,
    );

    // Reciprocal edge counting
    let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
    for e in &diagram.edges {
        let key = if e.from <= e.to {
            (e.from.clone(), e.to.clone())
        } else {
            (e.to.clone(), e.from.clone())
        };
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(String, String), usize> = HashMap::new();

    // Render edges
    for edge in &diagram.edges {
        let from_pos = positions.get(&edge.from);
        let to_pos = positions.get(&edge.to);
        if from_pos.is_none() || to_pos.is_none() {
            continue;
        }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        // Self-transition
        if edge.from == edge.to {
            let rx = fx + fw;
            let ry_top = fy + fh * 0.3;
            let ry_bot = fy + fh * 0.7;
            let bulge = 35.0;
            svg.push_str(&format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" \
                 stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                rx, ry_top,
                rx + bulge, ry_top - 15.0,
                rx + bulge, ry_bot + 15.0,
                rx, ry_bot,
                COLOR_EDGE
            ));
            if !edge.label.is_empty() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                    rx + bulge + 6.0,
                    (ry_top + ry_bot) / 2.0 + 4.0,
                    COLOR_EDGE,
                    escape_xml(&edge.label)
                ));
            }
            continue;
        }

        let pair_key = if edge.from <= edge.to {
            (edge.from.clone(), edge.to.clone())
        } else {
            (edge.to.clone(), edge.from.clone())
        };
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = {
            let seen = pair_seen.entry(pair_key).or_insert(0);
            let v = *seen;
            *seen += 1;
            v
        };

        // Curve bulge: always curve edges. The perpendicular direction is
        // derived from (cx1,cy1)->(cx2,cy2), so swapping from/to naturally
        // flips the curve to the opposite side. For reciprocal pairs we
        // increase the bulge so they separate further.
        let base_bulge = 40.0;
        let bulge = if total > 1 {
            base_bulge + idx as f64 * 25.0
        } else {
            base_bulge
        };

        // Compute control point: offset perpendicular to the line between centers
        let dx = cx2 - cx1;
        let dy = cy2 - cy1;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / len; // perpendicular unit vector
        let ny = dx / len;
        let ctrl_x = (cx1 + cx2) / 2.0 + nx * bulge;
        let ctrl_y = (cy1 + cy2) / 2.0 + ny * bulge;

        // Clip start/end to node boundaries, aiming at the control point
        let (ax1, ay1) = clip_to_rect(cx1, cy1, ctrl_x, ctrl_y, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = clip_to_rect(cx2, cy2, ctrl_x, ctrl_y, tw / 2.0, th / 2.0);

        let path_d = format!(
            "M{},{} Q{},{} {},{}",
            ax1, ay1, ctrl_x, ctrl_y, ax2, ay2
        );

        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" \
             marker-end=\"url(#arrow)\"/>",
            path_d, COLOR_EDGE
        ));

        if !edge.label.is_empty() {
            // Label at the midpoint of the quadratic bezier: B(0.5) = 0.25*P0 + 0.5*ctrl + 0.25*P1
            let mx = 0.25 * ax1 + 0.5 * ctrl_x + 0.25 * ax2;
            let my = 0.25 * ay1 + 0.5 * ctrl_y + 0.25 * ay2;
            let lw = text_width(&edge.label);
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" \
                 fill=\"white\" opacity=\"0.85\"/>",
                mx - lw / 2.0 - 3.0,
                my - 12.0,
                lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" \
                 fill=\"{}\">{}</text>",
                mx,
                my,
                COLOR_EDGE,
                escape_xml(&edge.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_elements_recursive(
    svg: &mut String,
    elements: &[Element],
    screens: &[Screen],
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        match elem {
            Element::GroupRef(gi) => {
                let group = &groups[*gi];
                if let Some(&(x, y, w, h)) = positions.get(&group.label) {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" \
                         stroke-dasharray=\"8,4\"/>",
                        x, y, w, h, COLOR_GROUP_FILL, COLOR_GROUP_STROKE
                    ));
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" \
                         fill=\"{}\">{}</text>",
                        x + 10.0,
                        y + 20.0,
                        COLOR_GROUP_STROKE,
                        escape_xml(&group.label)
                    ));
                }
                render_elements_recursive(svg, &group.children, screens, groups, positions);
            }
            Element::ScreenRef(si) => {
                let screen = &screens[*si];
                if let Some(&(x, y, _w, _h)) = positions.get(&screen.name) {
                    render_screen(svg, x, y, screen);
                }
            }
        }
    }
}

fn render_screen(svg: &mut String, x: f64, y: f64, screen: &Screen) {
    let (w, h) = screen_size(screen);

    // Card outline with shadow
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" \
         filter=\"drop-shadow(2px 2px 3px rgba(0,0,0,0.15))\"/>",
        x, y, w, h, SCREEN_RADIUS, COLOR_SCREEN_FILL, COLOR_SCREEN_STROKE
    ));

    // Title bar background
    svg.push_str(&format!(
        "<clipPath id=\"clip-{x}-{y}\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" \
         height=\"{th}\" rx=\"{r}\" /></clipPath>\
         <rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{th}\" \
         fill=\"{bg}\" clip-path=\"url(#clip-{x}-{y})\"/>",
        x = x,
        y = y,
        w = w,
        th = SCREEN_TITLE_H,
        r = SCREEN_RADIUS,
        bg = COLOR_TITLE_BG,
    ));

    // Title bar separator
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
        x,
        y + SCREEN_TITLE_H,
        x + w,
        y + SCREEN_TITLE_H,
        COLOR_SCREEN_STROKE
    ));

    // Window dots (browser-like)
    let dot_y = y + SCREEN_TITLE_H / 2.0;
    for (i, _) in [0, 1, 2].iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"{}\"/>",
            x + 14.0 + i as f64 * 14.0,
            dot_y,
            COLOR_TITLE_DOT
        ));
    }

    // Title text
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        x + 56.0,
        y + SCREEN_TITLE_H / 2.0 + 5.0,
        COLOR_SCREEN_STROKE,
        escape_xml(&screen.name)
    ));

    // Body elements
    for (i, elem) in screen.elements.iter().enumerate() {
        let ey = y + SCREEN_TITLE_H + SCREEN_BODY_PAD + (i as f64 + 0.75) * LINE_HEIGHT;
        // Bullet point
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\"/>",
            x + SCREEN_H_PAD,
            ey - 4.0,
            COLOR_ELEMENT_TEXT
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            x + SCREEN_H_PAD + 10.0,
            ey,
            COLOR_ELEMENT_TEXT,
            escape_xml(elem)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-screen-flow - Render a screen transition diagram as SVG

Usage: mdd-screen-flow < input.txt

Define screens and transitions between them.

Syntax:
  screen Name                     Simple screen (no details)
  screen Name {                   Screen with UI elements
    element1
    element2
  }
  From -> To : \"action\"           Transition with label
  From -> To                      Transition without label
  group \"Section\" {               Group screens together
    screen A
    screen B
  }

Example:
  screen Login {
    Email input
    Password input
    Login button
  }

  screen Home {
    Dashboard
    Menu
  }

  screen Settings

  Login -> Home : \"Login success\"
  Home -> Settings : \"Tap settings icon\"
  Login -> Login : \"Validation error\"
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
            eprintln!("mdd-screen-flow: {}", e);
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
    fn parse_simple_screen() {
        let d = parse("screen Home\n").unwrap();
        assert_eq!(d.screens.len(), 1);
        assert_eq!(d.screens[0].name, "Home");
        assert!(d.screens[0].elements.is_empty());
    }

    #[test]
    fn parse_screen_with_elements() {
        let input = "screen Login {\n  Email input\n  Password input\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.screens.len(), 1);
        assert_eq!(d.screens[0].name, "Login");
        assert_eq!(d.screens[0].elements.len(), 2);
        assert_eq!(d.screens[0].elements[0], "Email input");
    }

    #[test]
    fn parse_edge_with_label() {
        let input = "screen A\nscreen B\nA -> B : \"click\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "click");
    }

    #[test]
    fn parse_edge_without_label() {
        let input = "screen A\nscreen B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].label, "");
    }

    #[test]
    fn parse_self_transition() {
        let input = "screen A\nA -> A : \"retry\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.edges[0].from, d.edges[0].to);
    }

    #[test]
    fn parse_group() {
        let input = "group \"Auth\" {\n  screen Login\n  screen Register\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].label, "Auth");
        assert_eq!(d.groups[0].children.len(), 2);
        assert_eq!(d.top_level.len(), 1);
    }

    #[test]
    fn parse_unknown_screen_in_edge() {
        let result = parse("screen A\nA -> B\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unclosed_screen() {
        let result = parse("screen A {\n  element\n");
        assert!(result.is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "screen A\nscreen B\nA -> B : \"go\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("marker"));
    }

    #[test]
    fn render_screen_with_elements() {
        let input = "screen Login {\n  Email\n  Password\n}\nscreen Home\nLogin -> Home : \"submit\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Login"));
        assert!(svg.contains("Email"));
        assert!(svg.contains("Password"));
    }

    #[test]
    fn render_with_group() {
        let input = "group \"Auth\" {\n  screen Login\n  screen Register\n}\nscreen Home\nLogin -> Home\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Auth"));
        assert!(svg.contains("stroke-dasharray"));
    }
}
