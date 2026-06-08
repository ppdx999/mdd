use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Reaction {
    emoji: String,
    count: String,
}

#[derive(Debug)]
struct Message {
    name: String,
    body: Vec<String>,
    time: Option<String>,
    reactions: Vec<Reaction>,
    thread_count: Option<String>,
}

#[derive(Debug)]
struct Diagram {
    channel: Option<String>,
    messages: Vec<Message>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let lines: Vec<&str> = input.lines().collect();
    let mut channel: Option<String> = None;
    let mut messages: Vec<Message> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // channel #name
        if trimmed.starts_with("channel ") {
            let rest = trimmed.strip_prefix("channel ").unwrap().trim();
            channel = Some(rest.to_string());
            i += 1;
            continue;
        }

        // msg "Name" : "body"
        if trimmed.starts_with("msg ") {
            let rest = trimmed.strip_prefix("msg ").unwrap().trim();
            let (name, body_start) = parse_msg_header(rest)?;
            let (body, consumed) = if let Some(bs) = body_start {
                parse_multiline_desc(bs, &lines, i)?
            } else {
                (Vec::new(), 0)
            };
            i += consumed;

            messages.push(Message {
                name,
                body,
                time: None,
                reactions: Vec::new(),
                thread_count: None,
            });
            i += 1;
            continue;
        }

        // time "10:30 AM"
        if trimmed.starts_with("time ") {
            if let Some(msg) = messages.last_mut() {
                let rest = trimmed.strip_prefix("time ").unwrap().trim();
                msg.time = Some(strip_quotes(rest).to_string());
            }
            i += 1;
            continue;
        }

        // react :emoji: 3
        if trimmed.starts_with("react ") {
            if let Some(msg) = messages.last_mut() {
                let rest = trimmed.strip_prefix("react ").unwrap().trim();
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                let emoji = parts[0].to_string();
                let count = if parts.len() > 1 {
                    parts[1].trim().to_string()
                } else {
                    "1".to_string()
                };
                msg.reactions.push(Reaction { emoji, count });
            }
            i += 1;
            continue;
        }

        // thread 5
        if trimmed.starts_with("thread ") {
            if let Some(msg) = messages.last_mut() {
                let rest = trimmed.strip_prefix("thread ").unwrap().trim();
                msg.thread_count = Some(rest.to_string());
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if messages.is_empty() {
        return Err("At least 1 message is required".to_string());
    }

    Ok(Diagram { channel, messages })
}

fn parse_msg_header(rest: &str) -> Result<(String, Option<&str>), String> {
    if let Some(colon_pos) = rest.find(" : ") {
        let name = strip_quotes(rest[..colon_pos].trim()).to_string();
        let body_start = rest[colon_pos + 3..].trim();
        Ok((name, Some(body_start)))
    } else {
        let name = strip_quotes(rest.trim()).to_string();
        Ok((name, None))
    }
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
    Err("Unterminated message body (missing closing \")".to_string())
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
const FONT_SIZE: f64 = 14.0;
const NAME_FONT_SIZE: f64 = 14.0;
const BODY_FONT_SIZE: f64 = 14.0;
const TIME_FONT_SIZE: f64 = 11.0;
const REACTION_FONT_SIZE: f64 = 11.0;
const CHANNEL_FONT_SIZE: f64 = 15.0;
const THREAD_FONT_SIZE: f64 = 12.0;
const BODY_LINE_HEIGHT: f64 = 21.0;

const CARD_WIDTH: f64 = 440.0;
const CARD_H_PAD: f64 = 20.0;
const MSG_GAP: f64 = 4.0;
const PADDING: f64 = 0.0;

const AVATAR_SIZE: f64 = 36.0;
const AVATAR_RADIUS: f64 = 6.0;
const AVATAR_GAP: f64 = 10.0;

const CHANNEL_BAR_H: f64 = 44.0;
const REACTION_H: f64 = 26.0;
const REACTION_GAP: f64 = 6.0;
const REACTION_H_PAD: f64 = 8.0;
const REACTION_RADIUS: f64 = 12.0;
const THREAD_H: f64 = 22.0;

const COLOR_BG: &str = "#fff";
const COLOR_CHANNEL_BG: &str = "#fff";
const COLOR_CHANNEL_BORDER: &str = "#e0e0e0";
const COLOR_CHANNEL_TEXT: &str = "#1d1c1d";
const COLOR_NAME: &str = "#1d1c1d";
const COLOR_BODY: &str = "#1d1c1d";
const COLOR_TIME: &str = "#616061";
const COLOR_REACTION_BG: &str = "#f0f4ff";
const COLOR_REACTION_BORDER: &str = "#d0d8e8";
const COLOR_REACTION_TEXT: &str = "#1d1c1d";
const COLOR_THREAD: &str = "#1264a3";
const COLOR_HOVER_BG: &str = "#f8f8f8";

const AVATAR_COLORS: &[(&str, &str)] = &[
    ("#e8f5e9", "#2e7d32"),
    ("#e3f2fd", "#1565c0"),
    ("#fff8e1", "#f57f17"),
    ("#f3e5f5", "#7b1fa2"),
    ("#fce4ec", "#c62828"),
    ("#e0f2f1", "#00695c"),
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

fn wrap_text(s: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let scale = font_size / FONT_SIZE;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0;

    for c in s.chars() {
        let cw = if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH };
        let w = cw * scale;
        if current_w + w > max_width && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
            current_w = 0.0;
        }
        current.push(c);
        current_w += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn emoji_to_unicode(code: &str) -> &str {
    match code {
        ":+1:" | ":thumbsup:" => "\u{1F44D}",
        ":-1:" | ":thumbsdown:" => "\u{1F44E}",
        ":heart:" => "\u{2764}\u{FE0F}",
        ":tada:" => "\u{1F389}",
        ":rocket:" => "\u{1F680}",
        ":fire:" => "\u{1F525}",
        ":eyes:" => "\u{1F440}",
        ":wave:" => "\u{1F44B}",
        ":clap:" => "\u{1F44F}",
        ":100:" => "\u{1F4AF}",
        ":star:" => "\u{2B50}",
        ":sparkles:" => "\u{2728}",
        ":check:" | ":white_check_mark:" => "\u{2705}",
        ":x:" | ":cross_mark:" => "\u{274C}",
        ":warning:" => "\u{26A0}\u{FE0F}",
        ":bulb:" => "\u{1F4A1}",
        ":memo:" => "\u{1F4DD}",
        ":bug:" => "\u{1F41B}",
        ":wrench:" => "\u{1F527}",
        ":gear:" => "\u{2699}\u{FE0F}",
        ":lock:" => "\u{1F512}",
        ":key:" => "\u{1F511}",
        ":bell:" => "\u{1F514}",
        ":mega:" => "\u{1F4E3}",
        ":pray:" => "\u{1F64F}",
        ":muscle:" => "\u{1F4AA}",
        ":thinking:" | ":thinking_face:" => "\u{1F914}",
        ":smile:" | ":smiley:" => "\u{1F604}",
        ":laugh:" | ":laughing:" => "\u{1F606}",
        ":cry:" | ":sob:" => "\u{1F62D}",
        ":angry:" => "\u{1F620}",
        ":sunglasses:" => "\u{1F60E}",
        ":raised_hands:" => "\u{1F64C}",
        ":point_up:" => "\u{261D}\u{FE0F}",
        ":ok_hand:" => "\u{1F44C}",
        ":handshake:" => "\u{1F91D}",
        ":coffee:" => "\u{2615}",
        ":beer:" => "\u{1F37A}",
        ":pizza:" => "\u{1F355}",
        ":party_popper:" => "\u{1F389}",
        ":confetti_ball:" => "\u{1F38A}",
        ":trophy:" => "\u{1F3C6}",
        ":medal:" => "\u{1F3C5}",
        ":chart_with_upwards_trend:" | ":chart_up:" => "\u{1F4C8}",
        ":chart_with_downwards_trend:" | ":chart_down:" => "\u{1F4C9}",
        _ => code, // fallback: show as-is
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn wrap_body_lines(body: &[String]) -> Vec<String> {
    let max_w = CARD_WIDTH - CARD_H_PAD * 2.0 - AVATAR_SIZE - AVATAR_GAP - 20.0;
    body.iter()
        .flat_map(|line| wrap_text(line, max_w, BODY_FONT_SIZE))
        .collect()
}

fn msg_height(msg: &Message) -> f64 {
    let wrapped = wrap_body_lines(&msg.body);
    let body_h = if wrapped.is_empty() {
        0.0
    } else {
        wrapped.len() as f64 * BODY_LINE_HEIGHT
    };

    let reactions_h = if msg.reactions.is_empty() {
        0.0
    } else {
        REACTION_H + 6.0
    };

    let thread_h = if msg.thread_count.is_some() {
        THREAD_H + 4.0
    } else {
        0.0
    };

    let header_h = NAME_FONT_SIZE + 4.0;
    // top pad + header + body + reactions + thread + bottom pad
    8.0 + header_h + body_h + reactions_h + thread_h + 8.0
}

fn render_svg(diagram: &Diagram) -> String {
    let n = diagram.messages.len();

    // Card width is capped at CARD_WIDTH
    let card_w = CARD_WIDTH;

    let channel_h = if diagram.channel.is_some() { CHANNEL_BAR_H } else { 0.0 };

    let total_msgs_h: f64 = diagram.messages.iter()
        .map(|m| msg_height(m))
        .sum::<f64>()
        + (n.saturating_sub(1)) as f64 * MSG_GAP;

    let total_w = PADDING * 2.0 + card_w;
    let total_h = PADDING * 2.0 + channel_h + total_msgs_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        COLOR_BG
    ));
    svg.push_str(&format!(
        "<style>text {{ font-family: -apple-system, BlinkMacSystemFont, 'Slack-Lato', sans-serif; font-size: {}px; }}</style>",
        FONT_SIZE
    ));

    let mut y = PADDING;

    // Channel header
    if let Some(ref channel) = diagram.channel {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            PADDING, y, card_w, CHANNEL_BAR_H, COLOR_CHANNEL_BG
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            PADDING + CARD_H_PAD,
            y + CHANNEL_BAR_H / 2.0 + CHANNEL_FONT_SIZE * 0.35,
            CHANNEL_FONT_SIZE,
            COLOR_CHANNEL_TEXT,
            escape_xml(channel)
        ));
        // Bottom border
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            PADDING, y + CHANNEL_BAR_H, PADDING + card_w, y + CHANNEL_BAR_H, COLOR_CHANNEL_BORDER
        ));
        y += CHANNEL_BAR_H;
    }

    // Messages
    for (i, msg) in diagram.messages.iter().enumerate() {
        let mh = msg_height(msg);

        // Hover background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            PADDING, y, card_w, mh, if i % 2 == 0 { COLOR_BG } else { COLOR_HOVER_BG }
        ));

        let mut cy = y + 8.0;

        // Avatar (rounded square)
        let avatar_x = PADDING + CARD_H_PAD;
        let avatar_y = cy;
        let (avatar_bg, avatar_fg) = AVATAR_COLORS[i % AVATAR_COLORS.len()];
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
            avatar_x, avatar_y, AVATAR_SIZE, AVATAR_SIZE, AVATAR_RADIUS, avatar_bg
        ));
        let initial = msg.name.chars().next().unwrap_or('?');
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            avatar_x + AVATAR_SIZE / 2.0,
            avatar_y + AVATAR_SIZE / 2.0 + 5.5,
            avatar_fg,
            escape_xml(&initial.to_string())
        ));

        let text_x = avatar_x + AVATAR_SIZE + AVATAR_GAP;

        // Name + time
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            text_x, cy + NAME_FONT_SIZE, NAME_FONT_SIZE, COLOR_NAME, escape_xml(&msg.name)
        ));

        if let Some(ref time) = msg.time {
            let name_w = text_width(&msg.name) * (NAME_FONT_SIZE / FONT_SIZE);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                text_x + name_w + 8.0,
                cy + NAME_FONT_SIZE,
                TIME_FONT_SIZE,
                COLOR_TIME,
                escape_xml(time)
            ));
        }

        cy += NAME_FONT_SIZE + 4.0;

        // Body (with wrapping)
        let wrapped_body = wrap_body_lines(&msg.body);
        for line in &wrapped_body {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                text_x, cy + BODY_FONT_SIZE, BODY_FONT_SIZE, COLOR_BODY, escape_xml(line)
            ));
            cy += BODY_LINE_HEIGHT;
        }

        // Reactions
        if !msg.reactions.is_empty() {
            cy += 6.0;
            let mut rx = text_x;
            for reaction in &msg.reactions {
                let emoji = emoji_to_unicode(&reaction.emoji);
                let label = format!("{} {}", emoji, reaction.count);
                let rw = text_width(&label) * (REACTION_FONT_SIZE / FONT_SIZE) + REACTION_H_PAD * 2.0;

                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                    rx, cy, rw, REACTION_H, REACTION_RADIUS, COLOR_REACTION_BG, COLOR_REACTION_BORDER
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    rx + rw / 2.0,
                    cy + REACTION_H / 2.0 + REACTION_FONT_SIZE * 0.35,
                    REACTION_FONT_SIZE,
                    COLOR_REACTION_TEXT,
                    escape_xml(&label)
                ));
                rx += rw + REACTION_GAP;
            }
            cy += REACTION_H;
        }

        // Thread replies
        if let Some(ref count) = msg.thread_count {
            cy += 4.0;
            let thread_text = format!("{} replies", count);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                text_x,
                cy + THREAD_FONT_SIZE,
                THREAD_FONT_SIZE,
                COLOR_THREAD,
                escape_xml(&thread_text)
            ));
        }

        y += mh + MSG_GAP;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-slack - Render a Slack-style chat mockup as SVG

Usage: mdd-slack < input.slack

Use \"channel #name\" to set an optional channel header.
Define messages with: msg \"Name\" : \"body text\"
After a message, add optional metadata:
  time \"10:30 AM\"
  react :emoji: count
  thread N

Example:
  channel #general
  msg \"Alice\" : \"Good morning!\"
  time \"9:00 AM\"
  react :wave: 3
  msg \"Bob\" : \"Morning!\"
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
            eprintln!("mdd-slack: {}", e);
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
        let input = "msg \"Alice\" : \"Hello team!\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages.len(), 1);
        assert_eq!(d.messages[0].name, "Alice");
        assert_eq!(d.messages[0].body, vec!["Hello team!"]);
    }

    #[test]
    fn parse_with_channel() {
        let input = "channel #general\nmsg \"Alice\" : \"Hi\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.channel.as_deref(), Some("#general"));
    }

    #[test]
    fn parse_multiline_body() {
        let input = "msg \"Alice\" : \"Line one\nLine two\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages[0].body, vec!["Line one", "Line two"]);
    }

    #[test]
    fn parse_with_metadata() {
        let input = "msg \"Alice\" : \"Hello\"\ntime \"10:30 AM\"\nreact :+1: 3\nreact :heart: 2\nthread 5\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages[0].time.as_deref(), Some("10:30 AM"));
        assert_eq!(d.messages[0].reactions.len(), 2);
        assert_eq!(d.messages[0].reactions[0].emoji, ":+1:");
        assert_eq!(d.messages[0].reactions[0].count, "3");
        assert_eq!(d.messages[0].thread_count.as_deref(), Some("5"));
    }

    #[test]
    fn parse_multiple_messages() {
        let input = "msg \"Alice\" : \"Hi\"\nmsg \"Bob\" : \"Hey\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.messages.len(), 2);
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "channel #general\nmsg \"Alice\" : \"Hello!\"\nreact :+1: 3\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("#general"));
        assert!(svg.contains("Hello!"));
    }
}
