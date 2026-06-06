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

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("step ") {
            let rest = rest.trim();
            if let Some((title, desc)) = rest.split_once(" : ") {
                let title = title.trim().to_string();
                let desc = desc.trim().trim_matches('"').to_string();
                steps.push(Step {
                    title,
                    description: desc,
                });
            } else {
                steps.push(Step {
                    title: rest.to_string(),
                    description: String::new(),
                });
            }
        } else {
            return Err(format!("Unknown syntax: {}", line));
        }
    }

    if steps.is_empty() {
        return Err("No steps defined".to_string());
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
const STEP_OFFSET_X: f64 = 40.0;
const STEP_OFFSET_Y: f64 = 50.0;
const PADDING: f64 = 40.0;
const BADGE_RADIUS: f64 = 14.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#999";

// Step colors: cycle through these pastel backgrounds
const STEP_COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"), // light blue
    ("#e8f5e9", "#2e7d32"), // light green
    ("#fff8e1", "#f57f17"), // light yellow
    ("#f3e5f5", "#7b1fa2"), // light purple
    ("#e0f2f1", "#00695c"), // light teal
    ("#fce4ec", "#c62828"), // light pink
    ("#e8eaf6", "#283593"), // light indigo
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

    // Draw connector line behind steps (ascending path)
    if n > 1 {
        let mut path = String::from("M");
        for i in 0..n {
            let x = PADDING + i as f64 * STEP_OFFSET_X + step_w / 2.0;
            let y = PADDING + (n - 1 - i) as f64 * STEP_OFFSET_Y + step_h / 2.0;
            if i == 0 {
                path.push_str(&format!("{},{}", x, y));
            } else {
                path.push_str(&format!(" L{},{}", x, y));
            }
        }
        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"6,4\"/>",
            path, COLOR_EDGE
        ));
    }

    // Draw steps (bottom to top, i.e. step 0 at bottom-left)
    for (i, step) in diagram.steps.iter().enumerate() {
        let x = PADDING + i as f64 * STEP_OFFSET_X;
        let y = PADDING + (n - 1 - i) as f64 * STEP_OFFSET_Y;
        let (fill, stroke) = STEP_COLORS[i % STEP_COLORS.len()];

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
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"white\">{}</text>",
            badge_x,
            badge_y + FONT_SIZE * 0.35,
            FONT_SIZE,
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

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-steps: {}", e);
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
    fn parse_simple_steps() {
        let input = "step First\nstep Second\nstep Third\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 3);
        assert_eq!(d.steps[0].title, "First");
        assert_eq!(d.steps[1].title, "Second");
        assert_eq!(d.steps[2].title, "Third");
    }

    #[test]
    fn parse_step_with_description() {
        let input = "step Design : \"Create wireframes\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].title, "Design");
        assert_eq!(d.steps[0].description, "Create wireframes");
    }

    #[test]
    fn parse_empty_input() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unknown_syntax() {
        let result = parse("foo bar\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown syntax"));
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "\nstep A\n\nstep B\n\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps.len(), 2);
    }

    #[test]
    fn parse_japanese_titles() {
        let input = "step 要件定義\nstep 設計\nstep 実装\n";
        let d = parse(input).unwrap();
        assert_eq!(d.steps[0].title, "要件定義");
        assert_eq!(d.steps.len(), 3);
    }

    #[test]
    fn render_produces_svg() {
        let input = "step A\nstep B\nstep C\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn render_contains_white_background() {
        let input = "step A\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_step_elements() {
        let input = "step Alpha\nstep Beta\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
        assert!(svg.contains("<rect")); // step rectangles
        assert!(svg.contains("<circle")); // number badges
    }

    #[test]
    fn render_contains_description() {
        let input = "step Plan : \"Make a plan\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("Make a plan"));
    }

    #[test]
    fn render_connector_line() {
        let input = "step A\nstep B\nstep C\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("stroke-dasharray")); // dashed connector
    }
}
