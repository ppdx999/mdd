use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Item {
    label: String,
    value: f64,
}

#[derive(Debug)]
struct Ranking {
    unit: Option<String>,
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Ranking, String> {
    let mut unit: Option<String> = None;
    let mut items: Vec<Item> = Vec::new();

    for (line_no, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // unit "..."
        if line.starts_with("unit ") {
            let rest = line.strip_prefix("unit ").unwrap().trim();
            unit = Some(strip_quotes(rest));
            continue;
        }

        // label : value
        if let Some((label, value_str)) = line.split_once(" : ") {
            let value: f64 = value_str
                .trim()
                .parse()
                .map_err(|_| format!("Line {}: invalid number '{}'", line_no + 1, value_str.trim()))?;
            items.push(Item {
                label: label.trim().to_string(),
                value,
            });
            continue;
        }

        return Err(format!("Line {}: unknown syntax '{}'", line_no + 1, line));
    }

    if items.is_empty() {
        return Err("No ranking items defined".to_string());
    }

    Ok(Ranking { unit, items })
}

fn strip_quotes(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const PADDING: f64 = 20.0;
const ROW_HEIGHT: f64 = 36.0;
const ROW_GAP: f64 = 6.0;
const RANK_WIDTH: f64 = 32.0;
const LABEL_PAD: f64 = 12.0;
const BAR_MAX_WIDTH: f64 = 300.0;
const BAR_HEIGHT: f64 = 26.0;
const VALUE_PAD: f64 = 10.0;

const COLOR_DARK: &str = "#333";

const RANK_COLORS: &[(&str, &str)] = &[
    ("#fff8e1", "#f57f17"), // 1st — gold
    ("#f5f5f5", "#757575"), // 2nd — silver
    ("#fff3e0", "#e65100"), // 3rd — bronze
];

const DEFAULT_BAR_BG: &str = "#e3f2fd";
const DEFAULT_BAR_TEXT: &str = "#1565c0";

// ---------------------------------------------------------------------------
// Text utilities
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

fn render_svg(ranking: &Ranking) -> String {
    let max_value = ranking
        .items
        .iter()
        .map(|i| i.value)
        .fold(0.0_f64, f64::max);

    let max_label_w = ranking
        .items
        .iter()
        .map(|i| text_width(&i.label))
        .fold(0.0_f64, f64::max);

    let label_col_w = max_label_w + LABEL_PAD * 2.0;

    // Format values for display
    let formatted: Vec<String> = ranking
        .items
        .iter()
        .map(|i| format_value(i.value, ranking.unit.as_deref()))
        .collect();

    let max_value_text_w = formatted.iter().map(|s| text_width(s)).fold(0.0_f64, f64::max);

    let total_w =
        PADDING * 2.0 + RANK_WIDTH + label_col_w + BAR_MAX_WIDTH + VALUE_PAD + max_value_text_w + LABEL_PAD;

    let total_h =
        PADDING * 2.0 + (ROW_HEIGHT + ROW_GAP) * ranking.items.len() as f64 - ROW_GAP;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let base_x = PADDING;
    let y = PADDING;

    // Items
    for (i, item) in ranking.items.iter().enumerate() {
        let rank = i + 1;
        let row_y = y + (ROW_HEIGHT + ROW_GAP) * i as f64;
        let center_y = row_y + ROW_HEIGHT / 2.0;

        // Rank badge
        let (badge_bg, badge_text) = if i < RANK_COLORS.len() {
            RANK_COLORS[i]
        } else {
            ("#f5f5f5", "#999")
        };

        let badge_size = 26.0;
        let badge_cx = base_x + RANK_WIDTH / 2.0;
        let badge_cy = center_y;

        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
            badge_cx,
            badge_cy,
            badge_size / 2.0,
            badge_bg
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            badge_cx,
            badge_cy + 5.0,
            badge_text,
            rank
        ));

        // Label
        let label_x = base_x + RANK_WIDTH + LABEL_PAD;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\">{}</text>",
            label_x,
            center_y + 5.0,
            escape_xml(&item.label)
        ));

        // Bar
        let bar_x = base_x + RANK_WIDTH + label_col_w;
        let bar_w = if max_value > 0.0 {
            (item.value / max_value) * BAR_MAX_WIDTH
        } else {
            0.0
        };
        let bar_y = center_y - BAR_HEIGHT / 2.0;

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\"/>",
            bar_x, bar_y, bar_w, BAR_HEIGHT, DEFAULT_BAR_BG
        ));

        // Value text
        let value_text = &formatted[i];
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            bar_x + bar_w + VALUE_PAD,
            center_y + 5.0,
            DEFAULT_BAR_TEXT,
            escape_xml(value_text)
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn format_value(value: f64, unit: Option<&str>) -> String {
    let num = if value == value.floor() {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    };
    match unit {
        Some(u) => format!("{}{}", num, u),
        None => num,
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let ranking = match parse(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mdd-ranking: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&ranking));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "A : 100\nB : 50\n";
        let r = parse(input).unwrap();
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.items[0].label, "A");
        assert_eq!(r.items[0].value, 100.0);
        assert_eq!(r.items[1].label, "B");
        assert_eq!(r.items[1].value, 50.0);
    }

    #[test]
    fn parse_with_unit() {
        let input = "unit \"万円\"\nA : 100\n";
        let r = parse(input).unwrap();
        assert_eq!(r.unit.as_deref(), Some("万円"));
    }

    #[test]
    fn parse_japanese_labels() {
        let input = "営業一課 : 1500\n営業二課 : 800\n";
        let r = parse(input).unwrap();
        assert_eq!(r.items[0].label, "営業一課");
    }

    #[test]
    fn parse_decimal_values() {
        let input = "A : 3.14\nB : 2.71\n";
        let r = parse(input).unwrap();
        assert!((r.items[0].value - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_empty_lines_skipped() {
        let input = "\nA : 10\n\nB : 20\n\n";
        let r = parse(input).unwrap();
        assert_eq!(r.items.len(), 2);
    }

    #[test]
    fn parse_no_items_error() {
        let result = parse("unit \"empty\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_number_error() {
        let result = parse("A : abc\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid number"));
    }

    #[test]
    fn parse_unknown_syntax_error() {
        let result = parse("something weird\n");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown syntax"));
    }

    #[test]
    fn format_value_integer() {
        assert_eq!(format_value(100.0, None), "100");
        assert_eq!(format_value(100.0, Some("万円")), "100万円");
    }

    #[test]
    fn format_value_decimal() {
        assert_eq!(format_value(3.14, None), "3.14");
    }

    #[test]
    fn render_produces_svg() {
        let input = "A : 100\nB : 50\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn render_contains_white_background() {
        let input = "A : 100\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_items() {
        let input = "X : 200\nY : 100\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.contains(">X<"));
        assert!(svg.contains(">Y<"));
        assert!(svg.contains(">200<"));
        assert!(svg.contains(">100<"));
    }

    #[test]
    fn render_rank_badges() {
        let input = "A : 30\nB : 20\nC : 10\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.contains(">1<"));
        assert!(svg.contains(">2<"));
        assert!(svg.contains(">3<"));
    }

    #[test]
    fn render_unit_shown() {
        let input = "unit \"件\"\nA : 50\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.contains("50件"));
    }
}
