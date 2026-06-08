use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Step {
    title: String,
    description: String,
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
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Parse Name or Name { desc } (no prefix keyword)
        if let Some(brace_pos) = line.find('{') {
            let title = line[..brace_pos].trim().to_string();
            let after_brace = line[brace_pos + 1..].trim();
            // Single-line: Name { desc }
            if let Some(end) = after_brace.strip_suffix('}') {
                steps.push(Step {
                    title,
                    description: end.trim().to_string(),
                });
            } else {
                // Multiline block
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
                    title,
                    description: desc_lines.join("\n"),
                });
            }
        } else {
            // Plain name, no description
            steps.push(Step {
                title: line.to_string(),
                description: String::new(),
            });
        }
        i += 1;
    }

    if steps.is_empty() {
        return Err("No milestones defined".to_string());
    }

    Ok(Diagram { steps })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const DESC_FONT_SIZE: f64 = 11.0;
const LINE_HEIGHT: f64 = 18.0;

const STEP_WIDTH: f64 = 180.0;
const STEP_HEIGHT: f64 = 60.0;
const STEP_OFFSET_X: f64 = 60.0;
const STEP_OFFSET_Y: f64 = 70.0;
const PADDING: f64 = 50.0;
const BADGE_RADIUS: f64 = 14.0;

const COLOR_DARK: &str = "#333";

// Step colors: (desc_bg, title_bg, accent)
const STEP_COLORS: &[(&str, &str, &str)] = &[
    ("#f0f7ff", "#90caf9", "#1565c0"), // blue
    ("#f0f9f1", "#a5d6a7", "#2e7d32"), // green
    ("#fffef2", "#ffe082", "#f57f17"), // yellow
    ("#faf2fc", "#ce93d8", "#7b1fa2"), // purple
    ("#f0faf9", "#80cbc4", "#00695c"), // teal
    ("#fdf2f4", "#ef9a9a", "#c62828"), // pink
    ("#f2f3fa", "#9fa8da", "#283593"), // indigo
];

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let n = diagram.steps.len();

    // Compute step width based on content
    let step_w = diagram
        .steps
        .iter()
        .map(|s| {
            let title_w = text_width(&s.title) + 50.0; // room for badge
            let desc_w = if s.description.is_empty() {
                0.0
            } else {
                text_width(&s.description) + 20.0
            };
            title_w.max(desc_w).max(STEP_WIDTH)
        })
        .fold(0.0_f64, f64::max);

    let step_h = STEP_HEIGHT;

    // Staircase layout: step i is at position (i * offset_x, (n-1-i) * offset_y)
    // So step 0 is at bottom-left, step n-1 is at top-right
    let total_w = step_w + (n - 1) as f64 * STEP_OFFSET_X;
    let total_h = step_h + (n - 1) as f64 * STEP_OFFSET_Y;

    let svg_w = total_w + PADDING * 2.0;
    let svg_h = total_h + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_w, svg_h, svg_w, svg_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Draw background arrow (bottom-left to top-right, upward trend)
    {
        let arrow_margin = 20.0;
        let ax1 = arrow_margin;
        let ay1 = svg_h - arrow_margin;
        let ax2 = svg_w - arrow_margin;
        let ay2 = arrow_margin;
        let head_len = 14.0;
        // Arrowhead pointing upper-right (angle = atan2(ay2-ay1, ax2-ax1))
        let dx = ax2 - ax1;
        let dy = ay2 - ay1;
        let angle = dy.atan2(dx);
        let lx = ax2 - head_len * (angle - 0.35).cos();
        let ly = ay2 - head_len * (angle - 0.35).sin();
        let rx = ax2 - head_len * (angle + 0.35).cos();
        let ry = ay2 - head_len * (angle + 0.35).sin();
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"3\" stroke-linecap=\"round\"/>",
            ax1, ay1, ax2, ay2
        ));
        svg.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"#e0e0e0\"/>",
            ax2, ay2, lx, ly, rx, ry
        ));
    }

    // Draw steps (bottom to top, i.e. step 0 at bottom-left)
    for (i, step) in diagram.steps.iter().enumerate() {
        let x = PADDING + i as f64 * STEP_OFFSET_X;
        let y = PADDING + (n - 1 - i) as f64 * STEP_OFFSET_Y;
        let (fill, stroke, accent) = STEP_COLORS[i % STEP_COLORS.len()];

        // Step rectangle with rounded corners
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x, y, step_w, step_h, fill, stroke
        ));

        // Step number badge (circle)
        let badge_x = x + BADGE_RADIUS + 10.0;
        let badge_y = if step.description.is_empty() {
            y + step_h / 2.0
        } else {
            y + step_h / 2.0 - 4.0
        };
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" />",
            badge_x, badge_y, BADGE_RADIUS, stroke
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            badge_x,
            badge_y + FONT_SIZE * 0.35,
            FONT_SIZE,
            accent,
            i + 1
        ));

        // Title text
        let text_x = badge_x + BADGE_RADIUS + 10.0;
        let text_y = if step.description.is_empty() {
            y + step_h / 2.0 + LINE_HEIGHT * 0.35
        } else {
            y + step_h / 2.0 - 2.0
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\">{}</text>",
            text_x,
            text_y,
            FONT_SIZE,
            escape_xml(&step.title)
        ));

        // Description text (if present)
        if !step.description.is_empty() {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#666\">{}</text>",
                text_x,
                text_y + LINE_HEIGHT - 2.0,
                DESC_FONT_SIZE,
                escape_xml(&step.description)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-roadmap - Render a roadmap (staircase milestones) as SVG

Usage: mdd-roadmap < input.roadmap

Each line is a milestone name. Milestones are displayed as ascending
steps from bottom-left to top-right.

Add a description with braces: Name { description }
Multi-line descriptions use a block: Name {\\n  line1\\n  line2\\n}

Example:
  Plan
  Design { Create wireframes }
  Build
  Launch
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
            eprintln!("mdd-roadmap: {}", e);
            std::process::exit(1);
        }
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
    fn parse_simple_milestones() {
        let input = "First\nSecond\nThird\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 3);
        assert_eq!(d.steps[0].title, "First");
        assert_eq!(d.steps[1].title, "Second");
        assert_eq!(d.steps[2].title, "Third");
    }

    #[test]
    fn parse_milestone_with_description() {
        let input = "Design { Create wireframes }\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].title, "Design");
        assert_eq!(d.steps[0].description, "Create wireframes");
    }

    #[test]
    fn parse_milestone_multiline_description() {
        let input = "Design {\n  Line one\n  Line two\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].title, "Design");
        assert_eq!(d.steps[0].description, "Line one\nLine two");
    }

    #[test]
    fn parse_empty_input() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "\nA\n\nB\n\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 2);
    }

    #[test]
    fn parse_japanese_titles() {
        let input = "要件定義\n設計\n実装\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].title, "要件定義");
        assert_eq!(d.steps.len(), 3);
    }

    #[test]
    fn render_produces_svg() {
        let input = "A\nB\nC\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn render_contains_white_background() {
        let input = "A\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_step_elements() {
        let input = "Alpha\nBeta\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
        assert!(svg.contains("<rect")); // step rectangles
        assert!(svg.contains("<circle")); // number badges
    }

    #[test]
    fn render_contains_description() {
        let input = "Plan { Make a plan }\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Make a plan"));
    }

    #[test]
    fn render_background_arrow() {
        let input = "A\nB\nC\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("<line")); // arrow shaft
        assert!(svg.contains("<polygon")); // arrowhead
    }
}
