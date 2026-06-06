use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Participant {
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ArrowStyle {
    Solid,
    Dashed,
}

#[derive(Debug)]
struct Message {
    from: usize,
    to: usize,
    label: String,
    style: ArrowStyle,
}

#[derive(Debug)]
struct Diagram {
    participants: Vec<Participant>,
    messages: Vec<Message>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut participants: Vec<Participant> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut messages: Vec<Message> = Vec::new();

    let ensure_participant =
        |name: &str, participants: &mut Vec<Participant>, map: &mut HashMap<String, usize>| -> usize {
            if let Some(&id) = map.get(name) {
                return id;
            }
            let id = participants.len();
            map.insert(name.to_string(), id);
            participants.push(Participant {
                name: name.to_string(),
            });
            id
        };

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("participant ") {
            let name = line.strip_prefix("participant ").unwrap().trim().to_string();
            ensure_participant(&name, &mut participants, &mut name_to_id);
            continue;
        }

        // Dashed arrow: from --> to : "label"
        if line.contains(" --> ") {
            let parts: Vec<&str> = line.splitn(2, " --> ").collect();
            let from_name = parts[0].trim();
            let rest = parts[1];
            let (to_name, label) = parse_message_rest(rest);
            let from_id = ensure_participant(from_name, &mut participants, &mut name_to_id);
            let to_id = ensure_participant(to_name, &mut participants, &mut name_to_id);
            messages.push(Message {
                from: from_id,
                to: to_id,
                label,
                style: ArrowStyle::Dashed,
            });
            continue;
        }

        // Solid arrow: from -> to : "label"
        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from_name = parts[0].trim();
            let rest = parts[1];
            let (to_name, label) = parse_message_rest(rest);
            let from_id = ensure_participant(from_name, &mut participants, &mut name_to_id);
            let to_id = ensure_participant(to_name, &mut participants, &mut name_to_id);
            messages.push(Message {
                from: from_id,
                to: to_id,
                label,
                style: ArrowStyle::Solid,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram {
        participants,
        messages,
    })
}

fn parse_message_rest(rest: &str) -> (&str, String) {
    if let Some((to_part, label_part)) = rest.split_once(" : ") {
        (
            to_part.trim(),
            label_part.trim().trim_matches('"').to_string(),
        )
    } else {
        (rest.trim(), String::new())
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 30.0;

const PARTICIPANT_H_PAD: f64 = 16.0;
const PARTICIPANT_HEIGHT: f64 = 36.0;
const PARTICIPANT_MIN_W: f64 = 80.0;
const PARTICIPANT_GAP: f64 = 60.0;

const MSG_HEIGHT: f64 = 40.0;
const SELF_MSG_WIDTH: f64 = 30.0;
const SELF_MSG_HEIGHT: f64 = 25.0;

const COLOR_DARK: &str = "#333";
const COLOR_LINE: &str = "#999";
const COLOR_ARROW: &str = "#555";
const COLOR_PARTICIPANT_FILL: &str = "#e3f2fd";
const COLOR_PARTICIPANT_STROKE: &str = "#1976d2";

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

fn participant_width(name: &str) -> f64 {
    (text_width(name) + PARTICIPANT_H_PAD * 2.0).max(PARTICIPANT_MIN_W)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    if diagram.participants.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"></svg>"
            .to_string();
    }

    // Compute participant X positions (center of each)
    let widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|p| participant_width(&p.name))
        .collect();

    // Also consider message label widths for spacing
    let mut col_min_gap = vec![PARTICIPANT_GAP; diagram.participants.len()];
    for msg in &diagram.messages {
        if msg.from == msg.to {
            continue;
        }
        let left = msg.from.min(msg.to);
        let right = msg.from.max(msg.to);
        if right == left + 1 {
            let label_w = text_width(&msg.label) + 20.0;
            col_min_gap[left] = col_min_gap[left].max(label_w);
        }
    }

    let mut x_centers: Vec<f64> = Vec::new();
    let mut x = PADDING + widths[0] / 2.0;
    x_centers.push(x);
    for i in 1..diagram.participants.len() {
        let gap = (widths[i - 1] / 2.0 + widths[i] / 2.0 + col_min_gap[i - 1]).max(
            widths[i - 1] / 2.0 + widths[i] / 2.0 + PARTICIPANT_GAP,
        );
        x += gap;
        x_centers.push(x);
    }

    let top_y = PADDING;
    let lifeline_start = top_y + PARTICIPANT_HEIGHT;

    // Count self-messages for extra height
    let total_msg_height: f64 = diagram
        .messages
        .iter()
        .map(|m| {
            if m.from == m.to {
                MSG_HEIGHT + SELF_MSG_HEIGHT
            } else {
                MSG_HEIGHT
            }
        })
        .sum();

    let lifeline_end = lifeline_start + total_msg_height + MSG_HEIGHT;
    let bottom_y = lifeline_end;

    let svg_width = x_centers.last().unwrap() + widths.last().unwrap() / 2.0 + PADDING;
    let svg_height = bottom_y + PARTICIPANT_HEIGHT + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Arrow markers
    svg.push_str(&format!(
        "<defs>\
         <marker id=\"arrow-solid\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/>\
         </marker>\
         <marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\">\
         <polyline points=\"0,1 10,5 0,9\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>\
         </marker>\
         </defs>",
        COLOR_ARROW, COLOR_ARROW
    ));

    // Lifelines (behind everything)
    for (i, _) in diagram.participants.iter().enumerate() {
        let cx = x_centers[i];
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5,5\"/>",
            cx, lifeline_start, cx, lifeline_end, COLOR_LINE
        ));
    }

    // Top participant boxes
    for (i, p) in diagram.participants.iter().enumerate() {
        let cx = x_centers[i];
        let w = widths[i];
        render_participant(&mut svg, cx - w / 2.0, top_y, w, &p.name);
    }

    // Bottom participant boxes
    for (i, p) in diagram.participants.iter().enumerate() {
        let cx = x_centers[i];
        let w = widths[i];
        render_participant(&mut svg, cx - w / 2.0, bottom_y, w, &p.name);
    }

    // Messages
    let mut msg_y = lifeline_start + MSG_HEIGHT;
    for msg in &diagram.messages {
        let from_x = x_centers[msg.from];
        let to_x = x_centers[msg.to];

        let (stroke_dash, marker) = match msg.style {
            ArrowStyle::Solid => ("", "url(#arrow-solid)"),
            ArrowStyle::Dashed => ("stroke-dasharray=\"6,4\" ", "url(#arrow-open)"),
        };

        if msg.from == msg.to {
            // Self message: loop to the right
            let loop_right = from_x + SELF_MSG_WIDTH;
            svg.push_str(&format!(
                "<polyline points=\"{},{} {},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" {}marker-end=\"{}\"/>",
                from_x, msg_y,
                loop_right, msg_y,
                loop_right, msg_y + SELF_MSG_HEIGHT,
                from_x, msg_y + SELF_MSG_HEIGHT,
                COLOR_ARROW, stroke_dash, marker
            ));

            // Label
            if !msg.label.is_empty() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                    loop_right + 6.0,
                    msg_y + SELF_MSG_HEIGHT / 2.0 + 4.0,
                    COLOR_ARROW,
                    escape_xml(&msg.label)
                ));
            }

            msg_y += MSG_HEIGHT + SELF_MSG_HEIGHT;
        } else {
            // Normal message
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" {}marker-end=\"{}\"/>",
                from_x, msg_y, to_x, msg_y, COLOR_ARROW, stroke_dash, marker
            ));

            // Label above the arrow
            if !msg.label.is_empty() {
                let label_x = (from_x + to_x) / 2.0;
                let label_y = msg_y - 6.0;
                let lw = text_width(&msg.label);
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.9\"/>",
                    label_x - lw / 2.0 - 4.0,
                    label_y - 12.0,
                    lw + 8.0
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                    label_x, label_y, COLOR_ARROW, escape_xml(&msg.label)
                ));
            }

            msg_y += MSG_HEIGHT;
        }
    }

    svg.push_str("</svg>");
    svg
}

fn render_participant(svg: &mut String, x: f64, y: f64, w: f64, name: &str) {
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, w, PARTICIPANT_HEIGHT, COLOR_PARTICIPANT_FILL, COLOR_PARTICIPANT_STROKE
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
        x + w / 2.0,
        y + PARTICIPANT_HEIGHT / 2.0 + LINE_HEIGHT * 0.35,
        escape_xml(name)
    ));
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
            eprintln!("mdd-sequence: {}", e);
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
    fn parse_participant() {
        let d = parse("participant Alice\n").unwrap();
        assert_eq!(d.participants.len(), 1);
        assert_eq!(d.participants[0].name, "Alice");
    }

    #[test]
    fn parse_message_solid() {
        let input = "participant A\nparticipant B\nA -> B : \"hello\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages.len(), 1);
        assert_eq!(d.messages[0].label, "hello");
        assert_eq!(d.messages[0].style, ArrowStyle::Solid);
    }

    #[test]
    fn parse_message_dashed() {
        let input = "participant A\nparticipant B\nA --> B : \"reply\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages[0].style, ArrowStyle::Dashed);
        assert_eq!(d.messages[0].label, "reply");
    }

    #[test]
    fn parse_self_message() {
        let input = "participant A\nA -> A : \"self\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages[0].from, d.messages[0].to);
    }

    #[test]
    fn auto_participant() {
        let input = "Alice -> Bob : \"hi\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].name, "Alice");
        assert_eq!(d.participants[1].name, "Bob");
    }

    #[test]
    fn parse_message_without_label() {
        let input = "A -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages[0].label, "");
    }

    #[test]
    fn render_produces_svg() {
        let input = "Alice -> Bob : \"hello\"\nBob --> Alice : \"hi\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("arrow-solid"));
        assert!(svg.contains("arrow-open"));
    }
}
