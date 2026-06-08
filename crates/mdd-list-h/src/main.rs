use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Card {
    label: String,
    description: Vec<String>,
}

#[derive(Debug)]
struct ListH {
    cards: Vec<Card>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<ListH, String> {
    let mut cards: Vec<Card> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // card "Label" : "Description" or card "Label"
        if trimmed.starts_with("card ") {
            let rest = trimmed.strip_prefix("card ").unwrap().trim();
            let card = parse_card(rest)?;
            cards.push(card);
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if cards.len() < 2 {
        return Err("At least 2 cards are required".to_string());
    }

    Ok(ListH { cards })
}

fn parse_card(s: &str) -> Result<Card, String> {
    // Expect "Label" or "Label" : "Description"
    if !s.starts_with('"') {
        return Err(format!("Expected quoted label: {}", s));
    }

    let end_quote = s[1..]
        .find('"')
        .ok_or("Unterminated quote in card label")?;
    let label = s[1..=end_quote].to_string();
    let rest = s[end_quote + 2..].trim();

    if rest.is_empty() {
        return Ok(Card {
            label,
            description: Vec::new(),
        });
    }

    // Expect : "Description" — split on | for multi-line
    if rest.starts_with(':') {
        let after_colon = rest[1..].trim();
        let desc_text = strip_quotes(after_colon);
        let desc: Vec<String> = desc_text.split('|').map(|s| s.trim().to_string()).collect();
        return Ok(Card {
            label,
            description: desc,
        });
    }

    Err(format!("Unexpected content after card label: {}", rest))
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const LABEL_FONT_SIZE: f64 = 15.0;
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_LINE_H: f64 = 15.0;
const COLOR_DARK: &str = "#333";

const CARD_MIN_WIDTH: f64 = 140.0;
const CARD_H_PAD: f64 = 16.0;
const CARD_BASE_HEIGHT: f64 = 80.0;
const CARD_GAP: f64 = 12.0;
const ACCENT_HEIGHT: f64 = 4.0;
const PADDING: f64 = 40.0;
const BORDER_COLOR: &str = "#e0e0e0";

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

fn card_height(card: &Card) -> f64 {
    if card.description.is_empty() {
        CARD_BASE_HEIGHT
    } else {
        CARD_BASE_HEIGHT + (card.description.len() as f64 - 1.0) * DESC_LINE_H
    }
}

fn render_svg(list: &ListH) -> String {
    // Compute card widths
    let card_widths: Vec<f64> = list
        .cards
        .iter()
        .map(|card| {
            let label_w = text_width(&card.label) * (LABEL_FONT_SIZE / FONT_SIZE) + CARD_H_PAD * 2.0;
            let desc_w = card
                .description
                .iter()
                .map(|d| text_width(d) * (DESC_FONT_SIZE / FONT_SIZE) + CARD_H_PAD * 2.0)
                .fold(0.0_f64, f64::max);
            label_w.max(desc_w).max(CARD_MIN_WIDTH)
        })
        .collect();

    let max_card_h = list.cards.iter().map(|c| card_height(c)).fold(CARD_BASE_HEIGHT, f64::max);

    let total_cards_w: f64 =
        card_widths.iter().sum::<f64>() + (list.cards.len() as f64 - 1.0) * CARD_GAP;

    let total_w = PADDING * 2.0 + total_cards_w;
    let total_h = PADDING * 2.0 + max_card_h;

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

    // Cards
    let mut cx = PADDING;
    for (i, card) in list.cards.iter().enumerate() {
        let w = card_widths[i];
        let (_bg_color, accent_color) = COLORS[i % COLORS.len()];

        // Card background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"white\" stroke=\"{}\" stroke-width=\"1\"/>",
            cx, content_y, w, max_card_h, BORDER_COLOR
        ));

        // Accent bar (top, clipped within the rounded rect via a clipPath)
        let clip_id = format!("clip-card-{}", i);
        svg.push_str(&format!(
            "<clipPath id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\"/></clipPath>",
            clip_id, cx, content_y, w, max_card_h
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" clip-path=\"url(#{})\"/>",
            cx, content_y, w, ACCENT_HEIGHT, accent_color, clip_id
        ));

        // Label (larger, bold)
        let label_y = if card.description.is_empty() {
            content_y + max_card_h / 2.0 + LABEL_FONT_SIZE * 0.35
        } else {
            content_y + ACCENT_HEIGHT + 12.0 + LABEL_FONT_SIZE
        };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\">{}</text>",
            cx + w / 2.0,
            label_y,
            LABEL_FONT_SIZE,
            escape_xml(&card.label)
        ));

        // Description (multi-line)
        if !card.description.is_empty() {
            let desc_start_y = label_y + 18.0;
            for (j, desc_line) in card.description.iter().enumerate() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"#666\">{}</text>",
                    cx + w / 2.0,
                    desc_start_y + j as f64 * DESC_LINE_H,
                    DESC_FONT_SIZE,
                    escape_xml(desc_line)
                ));
            }
        }

        cx += w + CARD_GAP;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-list-h - Render a horizontal card list as SVG

Usage: mdd-list-h < input.list-h

Each card is: card \"<label>\" [: \"<description>\"]
Use \"|\" inside the description for multi-line text.
At least 2 cards are required.

Example:
  card \"Challenge\" : \"Embrace failure\"
  card \"Integrity\" : \"Stay honest\"
  card \"Teamwork\" : \"Achieve together\"
  card \"Growth\" : \"Keep learning\"
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

    let list = match parse(&input) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mdd-list-h: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&list));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = r#"
card "A"
card "B"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.cards.len(), 2);
        assert_eq!(list.cards[0].label, "A");
        assert!(list.cards[0].description.is_empty());
        assert_eq!(list.cards[1].label, "B");
    }

    #[test]
    fn parse_with_desc() {
        let input = r#"
card "Alpha" : "First letter"
card "Beta" : "Second letter"
"#;
        let list = parse(input).unwrap();
        assert_eq!(list.cards.len(), 2);
        assert_eq!(list.cards[0].label, "Alpha");
        assert_eq!(list.cards[0].description, vec!["First letter"]);
        assert_eq!(list.cards[1].label, "Beta");
        assert_eq!(list.cards[1].description, vec!["Second letter"]);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
card "X"
card "Y"
"#;
        let list = parse(input).unwrap();
        let svg = render_svg(&list);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn error_too_few_cards() {
        let input = r#"
card "Only"
"#;
        assert!(parse(input).is_err());
    }
}
