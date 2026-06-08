use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Entry {
    when: String,
    what: String,
}

#[derive(Debug)]
struct Timetable {
    entries: Vec<Entry>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Timetable, String> {
    let mut entries: Vec<Entry> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(pos) = trimmed.find(' ') {
            let when = trimmed[..pos].to_string();
            let what = trimmed[pos + 1..].trim().to_string();
            if !what.is_empty() {
                entries.push(Entry { when, what });
            }
        }
    }

    if entries.is_empty() {
        return Err("At least 1 entry is required".to_string());
    }

    Ok(Timetable { entries })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const WHEN_FONT_SIZE: f64 = 12.0;
const COLOR_DARK: &str = "#333";

const PADDING: f64 = 40.0;
const ROW_HEIGHT: f64 = 52.0;
const DOT_RADIUS: f64 = 5.0;
const CONNECTOR_LEN: f64 = 20.0;
const BOX_PADDING_X: f64 = 14.0;
const BOX_PADDING_Y: f64 = 8.0;
const BOX_RADIUS: f64 = 6.0;
const WHEN_RIGHT_MARGIN: f64 = 16.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"), // blue
    ("#e8f5e9", "#2e7d32"), // green
    ("#fff8e1", "#f57f17"), // yellow
    ("#f3e5f5", "#7b1fa2"), // purple
    ("#e0f2f1", "#00695c"), // teal
    ("#fce4ec", "#c62828"), // pink
    ("#e8eaf6", "#283593"), // indigo
    ("#fff3e0", "#e65100"), // orange
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

fn render_svg(tt: &Timetable) -> String {
    let n = tt.entries.len();

    // Compute column widths
    let max_when_w = tt
        .entries
        .iter()
        .map(|e| text_width(&e.when) * (WHEN_FONT_SIZE / FONT_SIZE))
        .fold(0.0_f64, f64::max);

    let max_what_w = tt
        .entries
        .iter()
        .map(|e| text_width(&e.what))
        .fold(0.0_f64, f64::max);

    let box_w = max_what_w + BOX_PADDING_X * 2.0;

    // Layout positions
    let when_x = PADDING + max_when_w; // right-aligned when text
    let axis_x = when_x + WHEN_RIGHT_MARGIN + DOT_RADIUS;
    let box_x = axis_x + DOT_RADIUS + CONNECTOR_LEN;

    let total_w = box_x + box_w + PADDING;
    let total_h = PADDING * 2.0 + n as f64 * ROW_HEIGHT;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Timeline axis (vertical line)
    let y_start = PADDING + ROW_HEIGHT / 2.0;
    let y_end = PADDING + (n as f64 - 0.5) * ROW_HEIGHT;
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"2\"/>",
        axis_x, y_start, axis_x, y_end
    ));

    // Entries
    for (i, entry) in tt.entries.iter().enumerate() {
        let cy = PADDING + i as f64 * ROW_HEIGHT + ROW_HEIGHT / 2.0;
        let (fill, accent) = COLORS[i % COLORS.len()];

        // When label (right-aligned)
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" font-weight=\"bold\" fill=\"#888\">{}</text>",
            when_x, cy + WHEN_FONT_SIZE * 0.35, WHEN_FONT_SIZE, escape_xml(&entry.when)
        ));

        // Dot on axis
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
            axis_x, cy, DOT_RADIUS, accent
        ));

        // Connector line
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            axis_x + DOT_RADIUS, cy, box_x, cy, accent
        ));

        // Event box
        let box_h = FONT_SIZE + BOX_PADDING_Y * 2.0;
        let box_y = cy - box_h / 2.0;
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            box_x, box_y, box_w, box_h, BOX_RADIUS, fill, accent
        ));

        // Event text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            box_x + BOX_PADDING_X,
            cy + FONT_SIZE * 0.35,
            FONT_SIZE,
            accent,
            escape_xml(&entry.what)
        ));
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-timetable - Render a timetable as SVG

Usage: mdd-timetable < input.timetable

Each line defines an entry: <when> <what>
The first space separates the time/date label from the event name.
Any format for the time label is accepted (HH:MM, YYYY-MM-DD, etc.).

Example:
  09:00 Morning standup
  10:00 Development
  12:00 Lunch break
  13:00 Code review
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

    let timetable = match parse(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("mdd-timetable: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&timetable));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "09:00 朝会\n10:00 開発\n";
        let tt = parse(input).unwrap();
        assert_eq!(tt.entries.len(), 2);
        assert_eq!(tt.entries[0].when, "09:00");
        assert_eq!(tt.entries[0].what, "朝会");
        assert_eq!(tt.entries[1].when, "10:00");
        assert_eq!(tt.entries[1].what, "開発");
    }

    #[test]
    fn parse_dates() {
        let input = "2025-01-15 企画開始\n2025-03-01 開発着手\n";
        let tt = parse(input).unwrap();
        assert_eq!(tt.entries.len(), 2);
        assert_eq!(tt.entries[0].when, "2025-01-15");
        assert_eq!(tt.entries[0].what, "企画開始");
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "\n09:00 A\n\n10:00 B\n\n";
        let tt = parse(input).unwrap();
        assert_eq!(tt.entries.len(), 2);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(parse("").is_err());
        assert!(parse("  \n  \n").is_err());
    }

    #[test]
    fn parse_preserves_order() {
        let input = "15:00 C\n09:00 A\n12:00 B\n";
        let tt = parse(input).unwrap();
        assert_eq!(tt.entries[0].what, "C");
        assert_eq!(tt.entries[1].what, "A");
        assert_eq!(tt.entries[2].what, "B");
    }

    #[test]
    fn render_produces_svg() {
        let input = "09:00 A\n10:00 B\n";
        let tt = parse(input).unwrap();
        let svg = render_svg(&tt);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn render_contains_entries() {
        let input = "09:00 朝会\n10:00 開発\n";
        let tt = parse(input).unwrap();
        let svg = render_svg(&tt);
        assert!(svg.contains("09:00"));
        assert!(svg.contains("朝会"));
        assert!(svg.contains("10:00"));
        assert!(svg.contains("開発"));
    }

    #[test]
    fn render_has_timeline_axis() {
        let input = "09:00 A\n10:00 B\n";
        let tt = parse(input).unwrap();
        let svg = render_svg(&tt);
        assert!(svg.contains("<line")); // axis
        assert!(svg.contains("<circle")); // dots
        assert!(svg.contains("<rect")); // boxes
    }
}
