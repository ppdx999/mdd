use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
}

#[derive(Debug)]
struct Event {
    date: Date,
    label: String,
    section: Option<usize>,
}

#[derive(Debug)]
struct Section {
    name: String,
}

#[derive(Debug)]
struct Diagram {
    sections: Vec<Section>,
    events: Vec<Event>,
}

// ---------------------------------------------------------------------------
// Date helpers
// ---------------------------------------------------------------------------

impl Date {
    fn is_leap(y: i32) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Self::is_leap(year) { 29 } else { 28 },
            _ => 30,
        }
    }

    fn format_ymd(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse_date(s: &str) -> Result<Date, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date: {}", s));
    }
    let year = parts[0].parse::<i32>().map_err(|_| format!("Invalid year: {}", parts[0]))?;
    let month = parts[1].parse::<u32>().map_err(|_| format!("Invalid month: {}", parts[1]))?;
    let day = parts[2].parse::<u32>().map_err(|_| format!("Invalid day: {}", parts[2]))?;
    if month < 1 || month > 12 {
        return Err(format!("Month out of range: {}", month));
    }
    if day < 1 || day > Date::days_in_month(year, month) {
        return Err(format!("Day out of range: {}", day));
    }
    Ok(Date { year, month, day })
}

fn parse(input: &str) -> Result<Diagram, String> {
    let mut sections: Vec<Section> = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut current_section: Option<usize> = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(s) = line.strip_prefix("section ") {
            let name = s.trim().to_string();
            current_section = Some(sections.len());
            sections.push(Section { name });
            continue;
        }

        // Event line: "YYYY-MM-DD : label text"
        if let Some((date_part, label_part)) = line.split_once(':') {
            let date_str = date_part.trim();
            let label = label_part.trim().trim_matches('"').to_string();
            if label.is_empty() {
                return Err(format!("Event label is empty: {}", line));
            }
            let date = parse_date(date_str)?;
            events.push(Event {
                date,
                label,
                section: current_section,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if events.is_empty() {
        return Err("No events defined".to_string());
    }

    // Sort events by date
    events.sort_by_key(|e| e.date);

    Ok(Diagram {
        sections,
        events,
    })
}

// ---------------------------------------------------------------------------
// Text measurement
// ---------------------------------------------------------------------------

fn text_width(s: &str, font_size: f64) -> f64 {
    let base = 13.0_f64;
    let ratio = font_size / base;
    s.chars()
        .map(|c| if c.is_ascii() { 8.0 } else { 14.0 })
        .sum::<f64>()
        * ratio
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Color palette
// ---------------------------------------------------------------------------

const SECTION_COLORS: &[(&str, &str)] = &[
    ("#5b9bd5", "#e3f2fd"), // blue
    ("#66bb6a", "#e8f5e9"), // green
    ("#ffa726", "#fff8e1"), // orange
    ("#ab47bc", "#f3e5f5"), // purple
    ("#ef5350", "#ffebee"), // red
    ("#26c6da", "#e0f7fa"), // cyan
];

fn section_color(idx: usize) -> (&'static str, &'static str) {
    SECTION_COLORS[idx % SECTION_COLORS.len()]
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;
const DATE_FONT: f64 = 10.0;
const PADDING: f64 = 40.0;
const EVENT_SPACING: f64 = 140.0; // minimum horizontal spacing between events
const STEM_HEIGHT: f64 = 60.0; // height of the vertical stem from axis to event box
const DOT_RADIUS: f64 = 6.0;
const BOX_PADDING_X: f64 = 12.0;
const BOX_PADDING_Y: f64 = 8.0;
const BOX_RADIUS: f64 = 8.0;
const LEGEND_ITEM_WIDTH: f64 = 16.0;
const LEGEND_ITEM_HEIGHT: f64 = 10.0;
const LEGEND_SPACING: f64 = 12.0;

fn render_svg(diagram: &Diagram) -> String {
    let has_sections = !diagram.sections.is_empty();
    let n = diagram.events.len();

    // Compute axis positions for each event
    let total_axis_width = (n.max(1) - 1) as f64 * EVENT_SPACING;

    let legend_offset = if has_sections { 30.0 } else { 0.0 };
    let axis_x_start = PADDING + 40.0;

    // Precompute box sizes and positions
    struct EventLayout {
        ax: f64,      // x on the axis
        above: bool,  // true = above axis, false = below
        box_w: f64,
        box_h: f64,
        label_lines: Vec<String>,
    }

    let mut layouts: Vec<EventLayout> = Vec::new();

    for (i, event) in diagram.events.iter().enumerate() {
        let ax = axis_x_start + i as f64 * EVENT_SPACING;
        let above = i % 2 == 0;

        // Word wrap label if too long
        let label_lines = wrap_text(&event.label, 16);
        let line_widths: Vec<f64> = label_lines.iter().map(|l| text_width(l, FONT_SIZE)).collect();
        let max_line_w = line_widths.iter().cloned().fold(0.0_f64, f64::max);
        let date_w = text_width(&event.date.format_ymd(), DATE_FONT);
        let content_w = max_line_w.max(date_w);
        let box_w = content_w + BOX_PADDING_X * 2.0;
        let box_h = BOX_PADDING_Y * 2.0
            + label_lines.len() as f64 * (FONT_SIZE + 4.0)
            + DATE_FONT + 4.0;

        layouts.push(EventLayout {
            ax,
            above,
            box_w,
            box_h,
            label_lines,
        });
    }

    // Compute SVG dimensions
    let max_above_box_h = layouts
        .iter()
        .filter(|l| l.above)
        .map(|l| l.box_h)
        .fold(0.0_f64, f64::max);
    let max_below_box_h = layouts
        .iter()
        .filter(|l| !l.above)
        .map(|l| l.box_h)
        .fold(0.0_f64, f64::max);

    let needed_above = STEM_HEIGHT + max_above_box_h + 10.0;
    let needed_below = STEM_HEIGHT + max_below_box_h + 10.0;

    // Adjust axis_y so there's enough room above
    let final_axis_y = PADDING + legend_offset + needed_above;
    let svg_width = axis_x_start + total_axis_width + PADDING + 40.0;
    let svg_height = final_axis_y + needed_below + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );

    // White background
    svg.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"white\"/>",
        svg_width, svg_height
    ));

    // Style
    svg.push_str(&format!(
        "<style>text {{ font-family: -apple-system, 'Segoe UI', sans-serif; font-size: {}px; }}</style>",
        FONT_SIZE
    ));

    // Legend
    if has_sections {
        let legend_y = PADDING + 6.0;
        let mut lx = PADDING;
        for (i, sec) in diagram.sections.iter().enumerate() {
            let (dot_color, _) = section_color(i);
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\"/>",
                lx,
                legend_y,
                LEGEND_ITEM_WIDTH,
                LEGEND_ITEM_HEIGHT,
                dot_color
            ));
            lx += LEGEND_ITEM_WIDTH + 6.0;
            let text_y = legend_y + LEGEND_ITEM_HEIGHT - 1.0;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#555\">{}</text>",
                lx, text_y, SMALL_FONT, escape_xml(&sec.name)
            ));
            lx += text_width(&sec.name, SMALL_FONT) + LEGEND_SPACING;
        }
    }

    // Horizontal axis line
    let axis_x_end = axis_x_start + total_axis_width;
    let line_extend = 20.0;
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"2\"/>",
        axis_x_start - line_extend,
        final_axis_y,
        axis_x_end + line_extend,
        final_axis_y
    ));

    // Arrow at the end
    svg.push_str(&format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"#ccc\"/>",
        axis_x_end + line_extend, final_axis_y,
        axis_x_end + line_extend - 8.0, final_axis_y - 5.0,
        axis_x_end + line_extend - 8.0, final_axis_y + 5.0,
    ));

    // Draw events
    for (i, event) in diagram.events.iter().enumerate() {
        let layout = &layouts[i];
        let ax = layout.ax;
        let above = layout.above;

        // Determine color
        let (dot_color, box_bg) = if let Some(sec_idx) = event.section {
            section_color(sec_idx)
        } else {
            ("#5b9bd5", "#e3f2fd")
        };

        // Dot on axis
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"white\" stroke-width=\"2\"/>",
            ax, final_axis_y, DOT_RADIUS, dot_color
        ));

        // Stem
        let (stem_y1, stem_y2) = if above {
            (final_axis_y - DOT_RADIUS, final_axis_y - STEM_HEIGHT)
        } else {
            (final_axis_y + DOT_RADIUS, final_axis_y + STEM_HEIGHT)
        };
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"4,3\"/>",
            ax, stem_y1, ax, stem_y2, dot_color
        ));

        // Box position
        let box_x = ax - layout.box_w / 2.0;
        let box_y = if above {
            stem_y2 - layout.box_h
        } else {
            stem_y2
        };

        // Box shadow
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"#00000008\"/>",
            box_x + 2.0, box_y + 2.0, layout.box_w, layout.box_h, BOX_RADIUS
        ));

        // Box
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            box_x, box_y, layout.box_w, layout.box_h, BOX_RADIUS, box_bg, dot_color
        ));

        // Date text (top of box)
        let date_text_y = box_y + BOX_PADDING_Y + DATE_FONT;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"#888\">{}</text>",
            ax, date_text_y, DATE_FONT, escape_xml(&event.date.format_ymd())
        ));

        // Label lines
        let label_start_y = date_text_y + DATE_FONT + 4.0;
        for (j, line) in layout.label_lines.iter().enumerate() {
            let ly = label_start_y + j as f64 * (FONT_SIZE + 4.0);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"500\" fill=\"#333\">{}</text>",
                ax, ly, FONT_SIZE, escape_xml(line)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

/// Simple word wrap: split at character boundary for CJK, or word boundary for ASCII.
fn wrap_text(s: &str, max_chars: usize) -> Vec<String> {
    if s.chars().count() <= max_chars {
        return vec![s.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut count = 0;

    for c in s.chars() {
        current.push(c);
        count += 1;
        if count >= max_chars {
            lines.push(current.clone());
            current.clear();
            count = 0;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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
            eprintln!("mdd-timeline: {}", e);
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
    fn parse_date_valid() {
        let d = parse_date("2025-06-15").unwrap();
        assert_eq!(d.year, 2025);
        assert_eq!(d.month, 6);
        assert_eq!(d.day, 15);
    }

    #[test]
    fn parse_date_invalid_format() {
        assert!(parse_date("2025-13-01").is_err());
        assert!(parse_date("2025-00-01").is_err());
        assert!(parse_date("not-a-date").is_err());
    }

    #[test]
    fn parse_simple() {
        let input = "2025-01-01 : Alpha\n2025-06-01 : Beta\n";
        let d = parse(input).unwrap();
        assert_eq!(d.events.len(), 2);
        assert_eq!(d.events[0].label, "Alpha");
        assert_eq!(d.events[1].label, "Beta");
    }

    #[test]
    fn parse_with_sections() {
        let input = "section Phase1\n2025-01-01 : Start\nsection Phase2\n2025-06-01 : Launch\n";
        let d = parse(input).unwrap();
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.events[0].section, Some(0));
        assert_eq!(d.events[1].section, Some(1));
    }

    #[test]
    fn parse_events_sorted() {
        let input = "2025-06-01 : Later\n2025-01-01 : Earlier\n";
        let d = parse(input).unwrap();
        assert_eq!(d.events[0].label, "Earlier");
        assert_eq!(d.events[1].label, "Later");
    }

    #[test]
    fn parse_quoted_label() {
        let input = "2025-01-01 : \"Quoted label\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.events[0].label, "Quoted label");
    }

    #[test]
    fn parse_empty_events_error() {
        let input = "section Only section\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_unknown_syntax_error() {
        let input = "this is not valid\n";
        assert!(parse(input).is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "2025-01-01 : Alpha\n2025-06-01 : Beta\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alpha"));
        assert!(svg.contains("Beta"));
    }

    #[test]
    fn render_has_white_background() {
        let input = "2025-01-01 : Test\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_contains_date() {
        let input = "2025-03-15 : Event\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("2025-03-15"));
    }

    #[test]
    fn wrap_short_text() {
        let lines = wrap_text("short", 16);
        assert_eq!(lines, vec!["short"]);
    }

    #[test]
    fn wrap_long_text() {
        let lines = wrap_text("this is a longer text for wrapping", 16);
        assert!(lines.len() > 1);
    }

    #[test]
    fn date_format() {
        let d = Date { year: 2025, month: 1, day: 5 };
        assert_eq!(d.format_ymd(), "2025-01-05");
    }
}
