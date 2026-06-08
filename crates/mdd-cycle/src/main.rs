use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Step {
    name: String,
    description: Vec<String>,
}

#[derive(Debug)]
struct Diagram {
    steps: Vec<Step>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut steps: Vec<Step> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if let Some(brace_pos) = trimmed.find('{') {
            let name = trimmed[..brace_pos].trim().to_string();
            let after_brace = trimmed[brace_pos + 1..].trim();
            if let Some(end) = after_brace.strip_suffix('}') {
                steps.push(Step {
                    name,
                    description: vec![end.trim().to_string()],
                });
            } else {
                let mut desc_lines = Vec::new();
                if !after_brace.is_empty() {
                    desc_lines.push(after_brace.to_string());
                }
                i += 1;
                while i < lines.len() {
                    let bl = lines[i].trim();
                    if bl == "}" {
                        break;
                    }
                    desc_lines.push(bl.to_string());
                    i += 1;
                }
                steps.push(Step {
                    name,
                    description: desc_lines,
                });
            }
        } else {
            steps.push(Step {
                name: trimmed.to_string(),
                description: Vec::new(),
            });
        }
        i += 1;
    }

    if steps.len() < 2 {
        return Err("At least 2 steps are required for a cycle".to_string());
    }

    Ok(Diagram { steps })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 60.0;

const NODE_H_PAD: f64 = 24.0;
const NODE_V_PAD: f64 = 14.0;
const NODE_MIN_W: f64 = 100.0;
const NODE_MIN_H: f64 = 44.0;
const NODE_RADIUS: f64 = 8.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_DESC: &str = "#666";
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_LINE_HEIGHT: f64 = 14.0;
const DESC_OFFSET: f64 = 12.0;

// Cycle-specific color palette (pastel tones per AGENTS.md)
const STEP_COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"), // light blue
    ("#e8f5e9", "#2e7d32"), // light green
    ("#fff8e1", "#f57f17"), // light yellow
    ("#f3e5f5", "#7b1fa2"), // light purple
    ("#e0f2f1", "#00695c"), // light teal
    ("#fce4ec", "#c62828"), // light pink
    ("#e8eaf6", "#283593"), // light indigo
    ("#fff3e0", "#e65100"), // light orange
];

// ---------------------------------------------------------------------------
// Text & sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

fn node_size(name: &str) -> (f64, f64) {
    let w = (text_width(name) + NODE_H_PAD * 2.0).max(NODE_MIN_W);
    let h = (LINE_HEIGHT + NODE_V_PAD * 2.0).max(NODE_MIN_H);
    (w, h)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let n = diagram.steps.len();
    let angle_step = std::f64::consts::TAU / n as f64;

    // Compute max node size to determine circle radius
    let max_node_w: f64 = diagram
        .steps
        .iter()
        .map(|s| node_size(&s.name).0)
        .fold(0.0, f64::max);
    let max_node_h: f64 = diagram
        .steps
        .iter()
        .map(|s| node_size(&s.name).1)
        .fold(0.0, f64::max);

    // Circle radius: ensure nodes don't overlap
    let node_diagonal = (max_node_w * max_node_w + max_node_h * max_node_h).sqrt();
    let min_radius_for_spacing = if n <= 2 {
        node_diagonal * 1.2
    } else {
        // Nodes are spaced angle_step apart; ensure arc distance > node size
        let chord_needed = node_diagonal * 1.3;
        chord_needed / (2.0 * (angle_step / 2.0).sin())
    };
    let radius = min_radius_for_spacing.max(120.0);

    // Center of the cycle
    let cx = radius + PADDING + max_node_w / 2.0;
    let cy = radius + PADDING + max_node_h / 2.0;

    // Extra padding for descriptions
    let has_desc = diagram.steps.iter().any(|s| !s.description.is_empty());
    let desc_extra = if has_desc {
        let max_desc_lines = diagram.steps.iter()
            .map(|s| s.description.len())
            .max()
            .unwrap_or(0);
        let max_desc_w = diagram.steps.iter()
            .flat_map(|s| s.description.iter())
            .map(|d| text_width(d) * (DESC_FONT_SIZE / 13.0))
            .fold(0.0_f64, f64::max);
        max_desc_w.max(max_desc_lines as f64 * DESC_LINE_HEIGHT) + DESC_OFFSET + 16.0
    } else {
        0.0
    };

    // SVG dimensions
    let svg_width = cx * 2.0 + desc_extra;
    let svg_height = cy * 2.0 + desc_extra;
    let cx = cx + desc_extra / 2.0;
    let cy = cy + desc_extra / 2.0;

    // Recompute positions with adjusted center
    let positions: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect();

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs>\
         <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/>\
         </marker>\
         </defs>",
        COLOR_EDGE
    ));

    // Render arrows (curved arcs between nodes)
    for i in 0..n {
        let next = (i + 1) % n;
        let (x1, y1) = positions[i];
        let (x2, y2) = positions[next];
        let (w1, h1) = node_size(&diagram.steps[i].name);
        let (w2, h2) = node_size(&diagram.steps[next].name);

        // Clip to node boundaries (elliptical approximation for rounded rects)
        let (sx, sy) = clip_to_rounded_rect(x1, y1, x2, y2, w1 / 2.0, h1 / 2.0);
        let (ex, ey) = clip_to_rounded_rect(x2, y2, x1, y1, w2 / 2.0, h2 / 2.0);

        // Curved arrow: use a quadratic bezier with control point pulled toward center
        let mid_x = (sx + ex) / 2.0;
        let mid_y = (sy + ey) / 2.0;
        let pull = 0.15; // how much to pull toward center
        let ctrl_x = mid_x + (cx - mid_x) * pull;
        let ctrl_y = mid_y + (cy - mid_y) * pull;

        svg.push_str(&format!(
            "<path d=\"M{},{} Q{},{} {},{}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"2\" marker-end=\"url(#arrow)\"/>",
            sx, sy, ctrl_x, ctrl_y, ex, ey, COLOR_EDGE
        ));
    }

    // Render nodes (on top of arrows)
    for (i, step) in diagram.steps.iter().enumerate() {
        let (nx, ny) = positions[i];
        let (w, h) = node_size(&step.name);
        let color_idx = i % STEP_COLORS.len();
        let (fill, stroke) = STEP_COLORS[color_idx];

        let rx = nx - w / 2.0;
        let ry = ny - h / 2.0;

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            rx, ry, w, h, NODE_RADIUS, fill, stroke
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            nx,
            ny + LINE_HEIGHT * 0.35,
            escape_xml(&step.name)
        ));

        // Description outside the node with radial relation line
        if !step.description.is_empty() {
            let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
            let dir_x = angle.cos();
            let dir_y = angle.sin();

            // Line length scales with description width
            let max_line_w = step.description.iter()
                .map(|d| text_width(d) * (DESC_FONT_SIZE / 13.0))
                .fold(0.0_f64, f64::max);
            let line_len = DESC_OFFSET + max_line_w * 0.3 + 20.0;

            // Line start: node edge
            let line_start_x = nx + dir_x * (w / 2.0);
            let line_start_y = ny + dir_y * (h / 2.0);
            // Line end: outward
            let line_end_x = nx + dir_x * (w / 2.0 + line_len);
            let line_end_y = ny + dir_y * (h / 2.0 + line_len);

            // Radial line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                line_start_x, line_start_y, line_end_x, line_end_y, "#ccc"
            ));
            // Dot at node edge
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"2.5\" fill=\"{}\"/>",
                line_start_x, line_start_y, stroke
            ));

            // Description text at end of line
            let desc_base_x = line_end_x + dir_x * 8.0;
            let desc_base_y = line_end_y + dir_y * 8.0;

            let anchor = if dir_x.abs() < 0.3 {
                "middle"
            } else if dir_x > 0.0 {
                "start"
            } else {
                "end"
            };

            let text_offset_y = -(step.description.len() as f64 - 1.0) * DESC_LINE_HEIGHT * 0.5;
            for (j, desc_line) in step.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    desc_base_x,
                    desc_base_y + text_offset_y + j as f64 * DESC_LINE_HEIGHT + DESC_FONT_SIZE * 0.35,
                    anchor,
                    DESC_FONT_SIZE,
                    COLOR_DESC,
                    escape_xml(desc_line)
                ));
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn clip_to_rounded_rect(
    cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64,
) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    // Use ellipse clipping for smooth results with rounded rects
    let angle = dy.atan2(dx);
    (cx + hw * angle.cos(), cy + hh * angle.sin())
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

const HELP: &str = "\
mdd-cycle - Render a cycle diagram as SVG

Usage: mdd-cycle < input.cycle

Each line is a step in the cycle. Steps are connected in order,
with the last step looping back to the first.

Add a description with braces: Name { description }
Multi-line descriptions use a block: Name {\\n  line1\\n  line2\\n}

Example:
  Plan
  Do
  Check
  Act
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
            eprintln!("mdd-cycle: {}", e);
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
    fn parse_simple() {
        let input = "Plan\nDo\nCheck\nAct\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 4);
        assert_eq!(d.steps[0].name, "Plan");
    }

    #[test]
    fn parse_rejects_single_step() {
        let input = "Only\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "\nA\n\nB\n\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 2);
    }

    #[test]
    fn parse_japanese() {
        let input = "計画\n実行\n評価\n改善\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].name, "計画");
    }

    #[test]
    fn render_produces_svg() {
        let input = "Plan\nDo\nCheck\nAct\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Plan"));
        assert!(svg.contains("Act"));
    }

    #[test]
    fn render_has_white_background() {
        let input = "A\nB\nC\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_has_arrows() {
        let input = "A\nB\nC\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("marker-end=\"url(#arrow)\""));
    }

    #[test]
    fn parse_with_description() {
        let input = "A { Do thing }\nB\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].description, vec!["Do thing"]);
        assert!(d.steps[1].description.is_empty());
    }

    #[test]
    fn parse_multiline_description() {
        let input = "A {\n  Line one\n  Line two\n}\nB\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].description, vec!["Line one", "Line two"]);
    }

    #[test]
    fn render_node_count_matches() {
        let input = "A\nB\nC\nD\nE\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        // 5 nodes = 5 rects (background rect uses width= not x=)
        assert_eq!(svg.matches("rx=\"8\"").count(), 5);
    }
}
