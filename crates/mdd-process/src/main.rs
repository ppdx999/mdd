use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Process {
    title: Option<String>,
    steps: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Process, String> {
    let mut title: Option<String> = None;
    let mut steps: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("title ") {
            let rest = trimmed.strip_prefix("title ").unwrap().trim();
            title = Some(strip_quotes(rest).to_string());
            continue;
        }

        if trimmed.starts_with("step ") {
            let rest = trimmed.strip_prefix("step ").unwrap().trim();
            steps.push(rest.to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if steps.len() < 2 {
        return Err("At least 2 steps are required".to_string());
    }

    Ok(Process { title, steps })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const TITLE_FONT_SIZE: f64 = 16.0;
const COLOR_DARK: &str = "#333";

const BOX_HEIGHT: f64 = 50.0;
const BOX_H_PAD: f64 = 20.0;
const ARROW_WIDTH: f64 = 30.0;
const PADDING: f64 = 40.0;
const MIN_BOX_WIDTH: f64 = 100.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;

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

    // Compute box widths
    let box_widths: Vec<f64> = process
        .steps
        .iter()
        .map(|s| (text_width(s) + BOX_H_PAD * 2.0).max(MIN_BOX_WIDTH))
        .collect();

    let total_boxes_w: f64 = box_widths.iter().sum();
    let total_arrows_w = ARROW_WIDTH * (n - 1) as f64;

    let title_space = if process.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

    let total_w = PADDING * 2.0 + total_boxes_w + total_arrows_w;
    let total_h = PADDING * 2.0 + title_space + BOX_HEIGHT;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Title
    let content_y = if let Some(ref title) = process.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            TITLE_FONT_SIZE,
            escape_xml(title)
        ));
        PADDING + TITLE_HEIGHT + TITLE_GAP
    } else {
        PADDING
    };

    let box_y = content_y;
    let mut x = PADDING;

    for (i, step) in process.steps.iter().enumerate() {
        let bw = box_widths[i];
        let (bg, border) = COLORS[i % COLORS.len()];

        // Box
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            x, box_y, bw, BOX_HEIGHT, bg, border
        ));

        // Text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" font-weight=\"bold\">{}</text>",
            x + bw / 2.0,
            box_y + BOX_HEIGHT / 2.0 + 5.0,
            border,
            escape_xml(step)
        ));

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

fn main() {
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
        let input = "step A\nstep B\nstep C\n";
        let p = parse(input).unwrap();
        assert!(p.title.is_none());
        assert_eq!(p.steps.len(), 3);
        assert_eq!(p.steps[0], "A");
        assert_eq!(p.steps[1], "B");
        assert_eq!(p.steps[2], "C");
    }

    #[test]
    fn parse_with_title() {
        let input = "title \"My Process\"\nstep X\nstep Y\n";
        let p = parse(input).unwrap();
        assert_eq!(p.title.as_deref(), Some("My Process"));
        assert_eq!(p.steps.len(), 2);
    }

    #[test]
    fn parse_too_few_steps() {
        let input = "step A\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "step A\nstep B\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }
}
