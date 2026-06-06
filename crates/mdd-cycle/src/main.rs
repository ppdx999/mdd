use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Step {
    name: String,
}

#[derive(Debug)]
struct Diagram {
    title: Option<String>,
    steps: Vec<Step>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut title: Option<String> = None;
    let mut steps: Vec<Step> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("title ") {
            title = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("step ") {
            steps.push(Step {
                name: rest.trim().to_string(),
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if steps.len() < 2 {
        return Err("At least 2 steps are required for a cycle".to_string());
    }

    Ok(Diagram { title, steps })
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

    // Compute node center positions (start from top, go clockwise)
    let positions: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect();

    // SVG dimensions
    let svg_width = cx * 2.0;
    let svg_height = cy * 2.0;

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

    // Render title in center
    if let Some(ref title) = diagram.title {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            cx,
            cy + 6.0,
            COLOR_DARK,
            escape_xml(title)
        ));
    }

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

fn main() {
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
        let input = "step Plan\nstep Do\nstep Check\nstep Act\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 4);
        assert_eq!(d.steps[0].name, "Plan");
        assert!(d.title.is_none());
    }

    #[test]
    fn parse_with_title() {
        let input = "title PDCA\nstep Plan\nstep Do\nstep Check\nstep Act\n";
        let d = parse(input).unwrap();
        assert_eq!(d.title.as_deref(), Some("PDCA"));
        assert_eq!(d.steps.len(), 4);
    }

    #[test]
    fn parse_rejects_single_step() {
        let input = "step Only\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "\nstep A\n\nstep B\n\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 2);
    }

    #[test]
    fn parse_unknown_syntax() {
        let input = "step A\nstep B\nfoo bar\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_japanese() {
        let input = "step 計画\nstep 実行\nstep 評価\nstep 改善\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].name, "計画");
    }

    #[test]
    fn render_produces_svg() {
        let input = "step Plan\nstep Do\nstep Check\nstep Act\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Plan"));
        assert!(svg.contains("Act"));
    }

    #[test]
    fn render_has_white_background() {
        let input = "step A\nstep B\nstep C\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_has_arrows() {
        let input = "step A\nstep B\nstep C\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("marker-end=\"url(#arrow)\""));
    }

    #[test]
    fn render_with_title() {
        let input = "title MyTitle\nstep A\nstep B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("MyTitle"));
    }

    #[test]
    fn render_node_count_matches() {
        let input = "step A\nstep B\nstep C\nstep D\nstep E\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        // 5 nodes = 5 rects (background rect uses width= not x=)
        assert_eq!(svg.matches("rx=\"8\"").count(), 5);
    }
}
