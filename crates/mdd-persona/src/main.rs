use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Actor {
    #[allow(dead_code)]
    name: String,
    label: String,
    speech: Vec<String>,
}

#[derive(Debug)]
struct Diagram {
    actors: Vec<Actor>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut actors: Vec<Actor> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // actor "Name" : "Label" : "Speech bubble"
        // actor "Name" : "Label"
        // actor "Name"
        if trimmed.starts_with("actor ") {
            let rest = trimmed.strip_prefix("actor ").unwrap().trim();
            let parts: Vec<&str> = rest.splitn(3, " : ").collect();

            match parts.len() {
                1 => {
                    let name = strip_quotes(parts[0].trim()).to_string();
                    actors.push(Actor {
                        label: name.clone(),
                        name,
                        speech: Vec::new(),
                    });
                }
                2 => {
                    let name = strip_quotes(parts[0].trim()).to_string();
                    let second = parts[1].trim();
                    if second.starts_with('"') {
                        let (speech, consumed) = parse_multiline_desc(second, &lines, i)?;
                        i += consumed;
                        actors.push(Actor {
                            label: name.clone(),
                            name,
                            speech,
                        });
                    } else {
                        actors.push(Actor {
                            name,
                            label: strip_quotes(second).to_string(),
                            speech: Vec::new(),
                        });
                    }
                }
                3 => {
                    let name = strip_quotes(parts[0].trim()).to_string();
                    let label = strip_quotes(parts[1].trim()).to_string();
                    let third = parts[2].trim();
                    let (speech, consumed) = parse_multiline_desc(third, &lines, i)?;
                    i += consumed;
                    actors.push(Actor {
                        name,
                        label,
                        speech,
                    });
                }
                _ => unreachable!(),
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if actors.is_empty() {
        return Err("At least 1 actor is required".to_string());
    }

    Ok(Diagram { actors })
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
    Err("Unterminated speech (missing closing \")".to_string())
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
const LABEL_FONT_SIZE: f64 = 12.0;
const SPEECH_FONT_SIZE: f64 = 12.0;

const PADDING: f64 = 40.0;
const ACTOR_GAP: f64 = 40.0;

// Stick figure dimensions
const HEAD_RADIUS: f64 = 14.0;
const BODY_LENGTH: f64 = 30.0;
const ARM_LENGTH: f64 = 18.0;
const ARM_Y_OFFSET: f64 = 8.0;
const LEG_LENGTH: f64 = 22.0;
const LEG_SPREAD: f64 = 14.0;
const FIGURE_HEIGHT: f64 = HEAD_RADIUS * 2.0 + BODY_LENGTH + LEG_LENGTH;

// Speech bubble
const BUBBLE_H_PAD: f64 = 14.0;
const BUBBLE_V_PAD: f64 = 10.0;
const BUBBLE_LINE_HEIGHT: f64 = 16.0;
const BUBBLE_TAIL_SIZE: f64 = 8.0;
const BUBBLE_GAP: f64 = 10.0;
const BUBBLE_RADIUS: f64 = 8.0;

const LABEL_GAP: f64 = 8.0;

const COLOR_DARK: &str = "#333";
const COLOR_BUBBLE_BG: &str = "#f7f8fc";
const COLOR_BUBBLE_BORDER: &str = "#ccc";
const COLOR_SPEECH: &str = "#444";
const COLOR_LABEL: &str = "#666";

const COLORS: &[&str] = &[
    "#1565c0", "#2e7d32", "#f57f17", "#7b1fa2",
    "#00695c", "#c62828", "#283593", "#e65100",
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
// SVG rendering
// ---------------------------------------------------------------------------

fn bubble_size(speech: &[String]) -> (f64, f64) {
    if speech.is_empty() {
        return (0.0, 0.0);
    }
    let max_w = speech.iter()
        .map(|s| text_width(s) * (SPEECH_FONT_SIZE / FONT_SIZE))
        .fold(0.0_f64, f64::max);
    let w = max_w + BUBBLE_H_PAD * 2.0;
    let h = speech.len() as f64 * BUBBLE_LINE_HEIGHT + BUBBLE_V_PAD * 2.0;
    (w.max(60.0), h)
}

fn actor_column_width(actor: &Actor) -> f64 {
    let label_w = text_width(&actor.label) * (LABEL_FONT_SIZE / FONT_SIZE);
    let figure_w = ARM_LENGTH * 2.0;
    let (bubble_w, _) = bubble_size(&actor.speech);
    label_w.max(figure_w).max(bubble_w)
}

fn render_svg(diagram: &Diagram) -> String {
    let n = diagram.actors.len();

    let col_widths: Vec<f64> = diagram.actors.iter()
        .map(|a| actor_column_width(a))
        .collect();

    let total_cols_w: f64 = col_widths.iter().sum::<f64>()
        + (n.saturating_sub(1)) as f64 * ACTOR_GAP;

    // Max bubble height
    let max_bubble_h = diagram.actors.iter()
        .map(|a| {
            let (_, bh) = bubble_size(&a.speech);
            if bh > 0.0 { bh + BUBBLE_TAIL_SIZE + BUBBLE_GAP } else { 0.0 }
        })
        .fold(0.0_f64, f64::max);

    let content_h = max_bubble_h + FIGURE_HEIGHT + LABEL_GAP + LABEL_FONT_SIZE;

    let total_w = PADDING * 2.0 + total_cols_w;
    let total_h = PADDING * 2.0 + content_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let content_y = PADDING;

    // Figure top starts after bubble area
    let figure_top = content_y + max_bubble_h;

    // Render each actor
    let mut x = PADDING;
    for (i, actor) in diagram.actors.iter().enumerate() {
        let col_w = col_widths[i];
        let cx = x + col_w / 2.0;
        let color = COLORS[i % COLORS.len()];

        // --- Stick figure ---
        let head_cy = figure_top + HEAD_RADIUS;
        let neck_y = head_cy + HEAD_RADIUS;
        let body_bottom = neck_y + BODY_LENGTH;
        let arm_y = neck_y + ARM_Y_OFFSET;

        // Head
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, head_cy, HEAD_RADIUS, color
        ));

        // Body
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, neck_y, cx, body_bottom, color
        ));

        // Arms
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx - ARM_LENGTH, arm_y + 6.0, cx + ARM_LENGTH, arm_y + 6.0, color
        ));

        // Legs
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, body_bottom, cx - LEG_SPREAD, body_bottom + LEG_LENGTH, color
        ));
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, body_bottom, cx + LEG_SPREAD, body_bottom + LEG_LENGTH, color
        ));

        // --- Label below figure ---
        let label_y = body_bottom + LEG_LENGTH + LABEL_GAP + LABEL_FONT_SIZE;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            cx, label_y, LABEL_FONT_SIZE, COLOR_LABEL, escape_xml(&actor.label)
        ));

        // --- Speech bubble above figure ---
        if !actor.speech.is_empty() {
            let (bw, bh) = bubble_size(&actor.speech);
            let bx = cx - bw / 2.0;
            let by = figure_top - BUBBLE_TAIL_SIZE - bh - BUBBLE_GAP;

            // Bubble rect
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                bx, by, bw, bh, BUBBLE_RADIUS, COLOR_BUBBLE_BG, COLOR_BUBBLE_BORDER
            ));

            // Tail (triangle pointing down to head)
            let tail_x = cx;
            let tail_top = by + bh;
            svg.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                tail_x - 6.0, tail_top,
                tail_x + 6.0, tail_top,
                tail_x, tail_top + BUBBLE_TAIL_SIZE,
                COLOR_BUBBLE_BG, COLOR_BUBBLE_BORDER
            ));
            // Cover the border at the base of the tail
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                tail_x - 5.0, tail_top, tail_x + 5.0, tail_top, COLOR_BUBBLE_BG
            ));

            // Speech text
            for (j, line) in actor.speech.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    cx,
                    by + BUBBLE_V_PAD + (j as f64 + 0.8) * BUBBLE_LINE_HEIGHT,
                    SPEECH_FONT_SIZE,
                    COLOR_SPEECH,
                    escape_xml(line)
                ));
            }
        }

        x += col_w + ACTOR_GAP;
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
            eprintln!("mdd-persona: {}", e);
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
    fn parse_basic() {
        let input = "actor Alice\nactor Bob\n";
        let d = parse(input).unwrap();
        assert_eq!(d.actors.len(), 2);
        assert_eq!(d.actors[0].name, "Alice");
        assert_eq!(d.actors[0].label, "Alice");
        assert!(d.actors[0].speech.is_empty());
    }

    #[test]
    fn parse_with_label() {
        let input = "actor Alice : Developer\n";
        let d = parse(input).unwrap();
        assert_eq!(d.actors[0].name, "Alice");
        assert_eq!(d.actors[0].label, "Developer");
    }

    #[test]
    fn parse_with_speech() {
        let input = "actor Alice : \"I need this feature!\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.actors[0].name, "Alice");
        assert_eq!(d.actors[0].label, "Alice");
        assert_eq!(d.actors[0].speech, vec!["I need this feature!"]);
    }

    #[test]
    fn parse_with_label_and_speech() {
        let input = "actor Alice : Developer : \"Fix the bug\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.actors[0].name, "Alice");
        assert_eq!(d.actors[0].label, "Developer");
        assert_eq!(d.actors[0].speech, vec!["Fix the bug"]);
    }

    #[test]
    fn parse_multiline_speech() {
        let input = "actor Alice : \"Line one\nLine two\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.actors[0].speech, vec!["Line one", "Line two"]);
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "actor Alice : \"Hello!\"\nactor Bob\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Hello!"));
    }

    #[test]
    fn render_has_stick_figure() {
        let input = "actor Alice\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("<circle")); // head
        assert!(svg.contains("<line")); // body/arms/legs
    }
}
