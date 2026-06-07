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
    title: Option<String>,
    steps: Vec<Step>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Process, String> {
    let mut title: Option<String> = None;
    let mut steps: Vec<Step> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if trimmed.starts_with("title ") {
            let rest = trimmed.strip_prefix("title ").unwrap().trim();
            title = Some(strip_quotes(rest).to_string());
            i += 1;
            continue;
        }

        if trimmed.starts_with("step ") {
            let rest = trimmed.strip_prefix("step ").unwrap().trim();
            if let Some((label, desc_part)) = rest.split_once(" : ") {
                let label = label.trim().to_string();
                let desc_part = desc_part.trim();
                let (desc, consumed) = parse_multiline_desc(desc_part, &lines, i)?;
                i += consumed;
                steps.push(Step { label, description: desc });
            } else {
                steps.push(Step {
                    label: rest.to_string(),
                    description: Vec::new(),
                });
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if steps.len() < 2 {
        return Err("At least 2 steps are required".to_string());
    }

    Ok(Process { title, steps })
}

fn parse_multiline_desc(start: &str, lines: &[&str], current: usize) -> Result<(Vec<String>, usize), String> {
    let content = start.strip_prefix('"').unwrap_or(start);
    if let Some(end) = content.find('"') {
        return Ok((vec![content[..end].to_string()], 0));
    }
    let mut desc_lines = vec![content.to_string()];
    let mut extra = 0;
    for j in (current + 1)..lines.len() {
        extra += 1;
        let line = lines[j].trim();
        if line.ends_with('"') {
            desc_lines.push(line[..line.len() - 1].to_string());
            return Ok((desc_lines, extra));
        }
        desc_lines.push(line.to_string());
    }
    Err("Unterminated description (missing closing \")".to_string())
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

    let title_space = if process.title.is_some() {
        TITLE_HEIGHT + TITLE_GAP
    } else {
        0.0
    };

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
    let total_h = PADDING * 2.0 + title_space + BOX_HEIGHT + desc_area_h;

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
        assert_eq!(p.steps[0].label, "A");
        assert_eq!(p.steps[1].label, "B");
        assert_eq!(p.steps[2].label, "C");
        assert!(p.steps[0].description.is_empty());
    }

    #[test]
    fn parse_with_title() {
        let input = "title \"My Process\"\nstep X\nstep Y\n";
        let p = parse(input).unwrap();
        assert_eq!(p.title.as_deref(), Some("My Process"));
        assert_eq!(p.steps.len(), 2);
    }

    #[test]
    fn parse_with_description() {
        let input = "step A : \"Do thing\"\nstep B : \"Do other\"\n";
        let p = parse(input).unwrap();
        assert_eq!(p.steps[0].label, "A");
        assert_eq!(p.steps[0].description, vec!["Do thing"]);
    }

    #[test]
    fn parse_multiline_description() {
        let input = "step A : \"Line one\nLine two\"\nstep B\n";
        let p = parse(input).unwrap();
        assert_eq!(p.steps[0].description, vec!["Line one", "Line two"]);
        assert!(p.steps[1].description.is_empty());
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
