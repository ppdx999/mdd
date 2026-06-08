use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Tweet {
    name: String,
    handle: String,
    body: Vec<String>,
    likes: Option<String>,
    retweets: Option<String>,
    time: Option<String>,
}

#[derive(Debug)]
struct Diagram {
    tweets: Vec<Tweet>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let lines: Vec<&str> = input.lines().collect();
    let mut tweets: Vec<Tweet> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // post "Name" @handle : "body"
        if trimmed.starts_with("post ") {
            let rest = trimmed.strip_prefix("post ").unwrap().trim();
            let (name, handle, body_start) = parse_post_header(rest)?;
            let (body, consumed) = if let Some(bs) = body_start {
                parse_multiline_desc(bs, &lines, i)?
            } else {
                (Vec::new(), 0)
            };
            i += consumed;

            tweets.push(Tweet {
                name,
                handle,
                body,
                likes: None,
                retweets: None,
                time: None,
            });
            i += 1;
            continue;
        }

        // Metadata for the last tweet: likes, retweets, time
        if trimmed.starts_with("likes ") {
            if let Some(tweet) = tweets.last_mut() {
                tweet.likes = Some(trimmed.strip_prefix("likes ").unwrap().trim().to_string());
            }
            i += 1;
            continue;
        }
        if trimmed.starts_with("retweets ") {
            if let Some(tweet) = tweets.last_mut() {
                tweet.retweets = Some(trimmed.strip_prefix("retweets ").unwrap().trim().to_string());
            }
            i += 1;
            continue;
        }
        if trimmed.starts_with("time ") {
            if let Some(tweet) = tweets.last_mut() {
                let rest = trimmed.strip_prefix("time ").unwrap().trim();
                tweet.time = Some(strip_quotes(rest).to_string());
            }
            i += 1;
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if tweets.is_empty() {
        return Err("At least 1 post is required".to_string());
    }

    Ok(Diagram { tweets })
}

fn parse_post_header(rest: &str) -> Result<(String, String, Option<&str>), String> {
    // "Name" @handle : "body..."
    // "Name" @handle
    // Find the @handle part
    let at_pos = rest.find('@').ok_or("Missing @handle in post")?;
    let name = strip_quotes(rest[..at_pos].trim()).to_string();

    let after_at = &rest[at_pos + 1..];
    if let Some(colon_pos) = after_at.find(" : ") {
        let handle = format!("@{}", after_at[..colon_pos].trim());
        let body_start = after_at[colon_pos + 3..].trim();
        Ok((name, handle, Some(body_start)))
    } else {
        let handle = format!("@{}", after_at.trim());
        Ok((name, handle, None))
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
    Err("Unterminated post body (missing closing \")".to_string())
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
const HANDLE_FONT_SIZE: f64 = 13.0;
const NAME_FONT_SIZE: f64 = 14.0;
const BODY_FONT_SIZE: f64 = 14.0;
const META_FONT_SIZE: f64 = 12.0;
const BODY_LINE_HEIGHT: f64 = 20.0;

const CARD_WIDTH: f64 = 400.0;
const CARD_H_PAD: f64 = 20.0;
const CARD_V_PAD: f64 = 16.0;
const CARD_GAP: f64 = 16.0;
const CARD_RADIUS: f64 = 12.0;
const PADDING: f64 = 24.0;

const AVATAR_SIZE: f64 = 40.0;
const AVATAR_GAP: f64 = 12.0;
const HEADER_HEIGHT: f64 = 20.0;

const META_HEIGHT: f64 = 20.0;
const DIVIDER_GAP: f64 = 10.0;

const COLOR_BG: &str = "#fff";
const COLOR_BORDER: &str = "#e1e8ed";
const COLOR_NAME: &str = "#14171a";
const COLOR_HANDLE: &str = "#657786";
const COLOR_BODY: &str = "#14171a";
const COLOR_META: &str = "#657786";
const COLOR_LIKE: &str = "#e0245e";
const COLOR_RETWEET: &str = "#17bf63";
const COLOR_DIVIDER: &str = "#e1e8ed";

const AVATAR_COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"),
    ("#e8f5e9", "#2e7d32"),
    ("#fff8e1", "#f57f17"),
    ("#f3e5f5", "#7b1fa2"),
    ("#e0f2f1", "#00695c"),
    ("#fce4ec", "#c62828"),
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

fn tweet_height(tweet: &Tweet) -> f64 {
    let body_h = if tweet.body.is_empty() {
        0.0
    } else {
        tweet.body.len() as f64 * BODY_LINE_HEIGHT + 8.0
    };

    let has_meta = tweet.likes.is_some() || tweet.retweets.is_some();
    let meta_h = if has_meta {
        DIVIDER_GAP + 1.0 + DIVIDER_GAP + META_HEIGHT
    } else {
        0.0
    };

    let time_h = if tweet.time.is_some() { META_HEIGHT } else { 0.0 };

    CARD_V_PAD + HEADER_HEIGHT + 4.0 + body_h + time_h + meta_h + CARD_V_PAD
}

fn render_svg(diagram: &Diagram) -> String {
    let n = diagram.tweets.len();

    // Compute card width based on content
    let max_body_w = diagram.tweets.iter()
        .flat_map(|t| t.body.iter())
        .map(|line| text_width(line) * (BODY_FONT_SIZE / FONT_SIZE))
        .fold(0.0_f64, f64::max);
    let card_w = (AVATAR_SIZE + AVATAR_GAP + max_body_w + CARD_H_PAD * 2.0)
        .max(CARD_WIDTH);

    let total_h_cards: f64 = diagram.tweets.iter()
        .map(|t| tweet_height(t))
        .sum::<f64>()
        + (n.saturating_sub(1)) as f64 * CARD_GAP;

    let total_w = PADDING * 2.0 + card_w;
    let total_h = PADDING * 2.0 + total_h_cards;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f5f8fa\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; font-size: {}px; }}</style>",
        FONT_SIZE
    ));

    let mut y = PADDING;

    for (i, tweet) in diagram.tweets.iter().enumerate() {
        let th = tweet_height(tweet);
        let cx = PADDING;

        // Card background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            cx, y, card_w, th, CARD_RADIUS, COLOR_BG, COLOR_BORDER
        ));

        let inner_x = cx + CARD_H_PAD;
        let mut cy = y + CARD_V_PAD;

        // Avatar circle
        let (avatar_bg, avatar_fg) = AVATAR_COLORS[i % AVATAR_COLORS.len()];
        let avatar_cx = inner_x + AVATAR_SIZE / 2.0;
        let avatar_cy = cy + AVATAR_SIZE / 2.0;
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
            avatar_cx, avatar_cy, AVATAR_SIZE / 2.0, avatar_bg
        ));
        // Initial letter in avatar
        let initial = tweet.name.chars().next().unwrap_or('?');
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            avatar_cx, avatar_cy + 6.0, avatar_fg, escape_xml(&initial.to_string())
        ));

        // Name and handle
        let text_x = inner_x + AVATAR_SIZE + AVATAR_GAP;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            text_x, cy + NAME_FONT_SIZE, NAME_FONT_SIZE, COLOR_NAME, escape_xml(&tweet.name)
        ));
        let name_w = text_width(&tweet.name) * (NAME_FONT_SIZE / FONT_SIZE);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            text_x + name_w + 6.0, cy + HANDLE_FONT_SIZE, HANDLE_FONT_SIZE, COLOR_HANDLE, escape_xml(&tweet.handle)
        ));

        cy += HEADER_HEIGHT + 4.0;

        // Body text
        if !tweet.body.is_empty() {
            for line in &tweet.body {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    text_x, cy + BODY_FONT_SIZE, BODY_FONT_SIZE, COLOR_BODY, escape_xml(line)
                ));
                cy += BODY_LINE_HEIGHT;
            }
            cy += 8.0;
        }

        // Time
        if let Some(ref time) = tweet.time {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                text_x, cy + META_FONT_SIZE, META_FONT_SIZE, COLOR_HANDLE, escape_xml(time)
            ));
            cy += META_HEIGHT;
        }

        // Divider + metrics
        let has_meta = tweet.likes.is_some() || tweet.retweets.is_some();
        if has_meta {
            cy += DIVIDER_GAP;
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                inner_x, cy, cx + card_w - CARD_H_PAD, cy, COLOR_DIVIDER
            ));
            cy += DIVIDER_GAP;

            let mut mx = text_x;

            if let Some(ref retweets) = tweet.retweets {
                // Retweet icon (two arrows)
                let icon_y = cy + META_FONT_SIZE * 0.4;
                svg.push_str(&format!(
                    "<g transform=\"translate({},{})\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-linecap=\"round\">",
                    mx, icon_y - 6.0, COLOR_RETWEET
                ));
                svg.push_str("<polyline points=\"0,4 4,0 8,4\"/><line x1=\"4\" y1=\"0\" x2=\"4\" y2=\"10\"/>");
                svg.push_str("<polyline points=\"10,8 14,12 18,8\"/><line x1=\"14\" y1=\"2\" x2=\"14\" y2=\"12\"/>");
                svg.push_str("</g>");
                mx += 24.0;
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    mx, cy + META_FONT_SIZE, META_FONT_SIZE, COLOR_META, escape_xml(retweets)
                ));
                mx += text_width(retweets) * (META_FONT_SIZE / FONT_SIZE) + 24.0;
            }

            if let Some(ref likes) = tweet.likes {
                // Heart icon
                let icon_y = cy + META_FONT_SIZE * 0.3;
                svg.push_str(&format!(
                    "<g transform=\"translate({},{})\">",
                    mx, icon_y - 5.0
                ));
                svg.push_str(&format!(
                    "<path d=\"M7,3 C7,1.3 5.7,0 4,0 C2.3,0 1,1.3 1,3 C1,6 4,8.5 7,11 C10,8.5 13,6 13,3 C13,1.3 11.7,0 10,0 C8.3,0 7,1.3 7,3Z\" fill=\"{}\" opacity=\"0.8\"/>",
                    COLOR_LIKE
                ));
                svg.push_str("</g>");
                mx += 18.0;
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
                    mx, cy + META_FONT_SIZE, META_FONT_SIZE, COLOR_META, escape_xml(likes)
                ));
            }
        }

        y += th + CARD_GAP;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-tweet - Render tweet/post cards as SVG

Usage: mdd-tweet < input.tweet

Each post starts with: post \"Name\" @handle : \"body text\"
After a post, add optional metadata lines:
  likes N, retweets N, time \"datetime string\"

Example:
  post \"Alice\" @alice : \"Hello world!\"
  likes 42

  post \"Bob\" @bob : \"Welcome!\"
  likes 5
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
            eprintln!("mdd-tweet: {}", e);
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
        let input = "post \"Alice\" @alice : \"Hello world!\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tweets.len(), 1);
        assert_eq!(d.tweets[0].name, "Alice");
        assert_eq!(d.tweets[0].handle, "@alice");
        assert_eq!(d.tweets[0].body, vec!["Hello world!"]);
    }

    #[test]
    fn parse_multiline_body() {
        let input = "post \"Alice\" @alice : \"Line one\nLine two\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tweets[0].body, vec!["Line one", "Line two"]);
    }

    #[test]
    fn parse_with_metadata() {
        let input = "post \"Alice\" @alice : \"Hello!\"\nlikes 42\nretweets 10\ntime \"2025-06-07 10:30\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tweets[0].likes.as_deref(), Some("42"));
        assert_eq!(d.tweets[0].retweets.as_deref(), Some("10"));
        assert_eq!(d.tweets[0].time.as_deref(), Some("2025-06-07 10:30"));
    }

    #[test]
    fn parse_no_body() {
        let input = "post \"Alice\" @alice\n";
        let d = parse(input).unwrap();
        assert!(d.tweets[0].body.is_empty());
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "post \"Alice\" @alice : \"Hello!\"\nlikes 5\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("@alice"));
        assert!(svg.contains("Hello!"));
    }

    #[test]
    fn render_multiple_posts() {
        let input = "post \"A\" @a : \"One\"\npost \"B\" @b : \"Two\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("One"));
        assert!(svg.contains("Two"));
    }
}
