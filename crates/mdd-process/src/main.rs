use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Step {
    label: String,
    description: Vec<String>,
}

#[derive(Debug)]
struct Process {
    steps: Vec<Step>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Process, String> {
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
            let label = trimmed[..brace_pos].trim().to_string();
            let after_brace = trimmed[brace_pos + 1..].trim();
            if let Some(end) = after_brace.strip_suffix('}') {
                steps.push(Step {
                    label,
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
                steps.push(Step { label, description: desc_lines });
            }
        } else {
            steps.push(Step {
                label: trimmed.to_string(),
                description: Vec::new(),
            });
        }
        i += 1;
    }

    if steps.len() < 2 {
        return Err("At least 2 steps are required".to_string());
    }

    Ok(Process { steps })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const BOX_HEIGHT: f64 = 50.0;
const BOX_H_PAD: f64 = 20.0;
const ARROW_WIDTH: f64 = 30.0;
const PADDING: f64 = 40.0;
const MIN_BOX_WIDTH: f64 = 100.0;
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_LINE_HEIGHT: f64 = 15.0;
const DESC_GAP: f64 = 8.0;
const COLOR_DESC: &str = "#666";

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

// ---------------------------------------------------------------------------
// Text & sizing
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
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(process: &Process) -> String {
    let n = process.steps.len();

    // Compute box widths (consider description width too)
    let box_widths: Vec<f64> = process
        .steps
        .iter()
        .map(|s| {
            let label_w = text_width(&s.label) + BOX_H_PAD * 2.0;
            let desc_w = s.description.iter()
                .map(|d| text_width(d) * (DESC_FONT_SIZE / FONT_SIZE) + BOX_H_PAD)
                .fold(0.0_f64, f64::max);
            label_w.max(desc_w).max(MIN_BOX_WIDTH)
        })
        .collect();

    let total_boxes_w: f64 = box_widths.iter().sum();
    let total_arrows_w = ARROW_WIDTH * (n - 1) as f64;

    let max_desc_lines = process.steps.iter()
        .map(|s| s.description.len())
        .max()
        .unwrap_or(0);
    let desc_area_h = if max_desc_lines > 0 {
        DESC_GAP + max_desc_lines as f64 * DESC_LINE_HEIGHT
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + total_boxes_w + total_arrows_w;
    let total_h = PADDING * 2.0 + BOX_HEIGHT + desc_area_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let box_y = PADDING;
    let mut x = PADDING;

    for (i, step) in process.steps.iter().enumerate() {
        let bw = box_widths[i];
        let (bg, border) = COLORS[i % COLORS.len()];

        // Box
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            x, box_y, bw, BOX_HEIGHT, bg, border
        ));

        // Label text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" font-weight=\"bold\">{}</text>",
            x + bw / 2.0,
            box_y + BOX_HEIGHT / 2.0 + 5.0,
            border,
            escape_xml(&step.label)
        ));

        // Description below the box
        if !step.description.is_empty() {
            let desc_start_y = box_y + BOX_HEIGHT + DESC_GAP;
            for (j, desc_line) in step.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    x + bw / 2.0,
                    desc_start_y + j as f64 * DESC_LINE_HEIGHT + DESC_FONT_SIZE * 0.8,
                    DESC_FONT_SIZE,
                    COLOR_DESC,
                    escape_xml(desc_line)
                ));
            }
        }

        x += bw;

        // Arrow (except after the last box)
        if i < n - 1 {
            let arrow_y = box_y + BOX_HEIGHT / 2.0;
            let ax1 = x + 4.0;
            let ax2 = x + ARROW_WIDTH - 4.0;

            // Arrow line
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                ax1, arrow_y, ax2 - 6.0, arrow_y, COLOR_DARK
            ));

            // Arrowhead (triangle)
            svg.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                ax2, arrow_y,
                ax2 - 8.0, arrow_y - 5.0,
                ax2 - 8.0, arrow_y + 5.0,
                COLOR_DARK
            ));

            x += ARROW_WIDTH;
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-process - Render a process flow as SVG

Usage: mdd-process < input.process

Each line is a step in a left-to-right flow connected by arrows.
Add a description with braces: Name { description }
Multi-line descriptions use a block: Name {\\n  line1\\n  line2\\n}
At least 2 steps are required.

Example:
  Plan
  Build
  Test
  Deploy
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

    let process = match parse(&input) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("mdd-process: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&process));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "A\nB\nC\n";
        let p = parse(input).unwrap();
        assert_eq!(p.steps.len(), 3);
        assert_eq!(p.steps[0].label, "A");
        assert_eq!(p.steps[1].label, "B");
        assert_eq!(p.steps[2].label, "C");
        assert!(p.steps[0].description.is_empty());
    }

    #[test]
    fn parse_with_description() {
        let input = "A { Do thing }\nB { Do other }\n";
        let p = parse(input).unwrap();
        assert_eq!(p.steps[0].label, "A");
        assert_eq!(p.steps[0].description, vec!["Do thing"]);
    }

    #[test]
    fn parse_multiline_description() {
        let input = "A {\n  Line one\n  Line two\n}\nB\n";
        let p = parse(input).unwrap();
        assert_eq!(p.steps[0].description, vec!["Line one", "Line two"]);
        assert!(p.steps[1].description.is_empty());
    }

    #[test]
    fn parse_too_few_steps() {
        let input = "A\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "A\nB\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
