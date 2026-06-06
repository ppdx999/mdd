use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SetDef {
    label: String,
    items: Vec<String>,
}

#[derive(Debug)]
struct OverlapDef {
    label: String,
    items: Vec<String>,
}

#[derive(Debug)]
struct Venn {
    title: Option<String>,
    sets: Vec<SetDef>,
    overlaps: Vec<OverlapDef>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Venn, String> {
    let mut title: Option<String> = None;
    let mut sets: Vec<SetDef> = Vec::new();
    let mut overlaps: Vec<OverlapDef> = Vec::new();

    let mut current_kind: Option<&str> = None; // "set" or "overlap"
    let mut current_label = String::new();
    let mut current_items: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // title "..."
        if trimmed.starts_with("title ") {
            let rest = trimmed.strip_prefix("title ").unwrap().trim();
            title = Some(strip_quotes(rest).to_string());
            continue;
        }

        // set "label" {
        if trimmed.starts_with("set ") && current_kind.is_none() {
            let rest = trimmed.strip_prefix("set ").unwrap().trim();
            let label = parse_section_header(rest)?;
            current_kind = Some("set");
            current_label = label;
            current_items = Vec::new();
            continue;
        }

        // overlap "label" {
        if trimmed.starts_with("overlap ") && current_kind.is_none() {
            let rest = trimmed.strip_prefix("overlap ").unwrap().trim();
            let label = parse_section_header(rest)?;
            current_kind = Some("overlap");
            current_label = label;
            current_items = Vec::new();
            continue;
        }

        // closing brace
        if trimmed == "}" {
            match current_kind {
                Some("set") => {
                    sets.push(SetDef {
                        label: current_label.clone(),
                        items: std::mem::take(&mut current_items),
                    });
                    current_kind = None;
                }
                Some("overlap") => {
                    overlaps.push(OverlapDef {
                        label: current_label.clone(),
                        items: std::mem::take(&mut current_items),
                    });
                    current_kind = None;
                }
                _ => return Err("Unexpected '}'".to_string()),
            }
            continue;
        }

        // inside a section
        if current_kind.is_some() {
            current_items.push(trimmed.to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if current_kind.is_some() {
        return Err("Unclosed section (missing '}')".to_string());
    }

    let n = sets.len();
    if n < 2 || n > 3 {
        return Err(format!(
            "Expected 2 or 3 sets, got {}",
            n
        ));
    }

    Ok(Venn {
        title,
        sets,
        overlaps,
    })
}

fn parse_section_header(rest: &str) -> Result<String, String> {
    let rest = rest.trim();
    if rest.starts_with('"') {
        let end_quote = rest[1..]
            .find('"')
            .ok_or("Unterminated quote in section header")?;
        let label = rest[1..=end_quote].to_string();
        let after_quote = rest[end_quote + 2..].trim();
        if after_quote == "{" || after_quote.is_empty() {
            return Ok(label);
        }
        return Err(format!("Expected '{{' after label, got: {}", after_quote));
    }
    if rest.ends_with('{') {
        let label = rest.trim_end_matches('{').trim();
        return Ok(label.to_string());
    }
    Ok(rest.to_string())
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
const COLOR_DARK: &str = "#333";

const SET_COLORS: &[&str] = &["#1565c0", "#2e7d32", "#f57f17"];

const CIRCLE_RADIUS: f64 = 120.0;
const OVERLAP_RATIO: f64 = 0.4;
const PADDING: f64 = 60.0;
const TITLE_HEIGHT: f64 = 24.0;
const TITLE_GAP: f64 = 16.0;
const ITEM_LINE_HEIGHT: f64 = 18.0;
const LABEL_FONT_SIZE: f64 = 14.0;

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

fn render_svg(venn: &Venn) -> String {
    if venn.sets.len() == 2 {
        render_two_sets(venn)
    } else {
        render_three_sets(venn)
    }
}

fn render_two_sets(venn: &Venn) -> String {
    let r = CIRCLE_RADIUS;
    let overlap_dist = r * 2.0 * (1.0 - OVERLAP_RATIO);

    // Circle centers
    let cx1 = PADDING + r;
    let cx2 = cx1 + overlap_dist;
    let cy_circles = PADDING + if venn.title.is_some() { TITLE_HEIGHT + TITLE_GAP } else { 0.0 } + r;

    let total_w = cx2 + r + PADDING;
    let total_h = cy_circles + r + PADDING;

    let mut svg = svg_header(total_w, total_h);

    // Title
    if let Some(ref title) = venn.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            escape_xml(title)
        ));
    }

    // Circles
    for (i, (cx, cy)) in [(cx1, cy_circles), (cx2, cy_circles)].iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, cy, r, SET_COLORS[i], SET_COLORS[i]
        ));
    }

    // Set labels — positioned in the non-overlapping part
    let label_offset = overlap_dist / 2.0 + r / 2.0;
    // Set A label: left side
    let set_a_label_x = cx1 - (r - overlap_dist / 2.0) / 2.0;
    let set_b_label_x = cx2 + (r - overlap_dist / 2.0) / 2.0;

    let label_y = cy_circles - r * 0.3;

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        set_a_label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[0], escape_xml(&venn.sets[0].label)
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        set_b_label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[1], escape_xml(&venn.sets[1].label)
    ));

    // Set A items
    render_items_at(&mut svg, set_a_label_x, label_y + ITEM_LINE_HEIGHT, &venn.sets[0].items, COLOR_DARK);

    // Set B items
    render_items_at(&mut svg, set_b_label_x, label_y + ITEM_LINE_HEIGHT, &venn.sets[1].items, COLOR_DARK);

    // Overlap items in the center
    let overlap_x = (cx1 + cx2) / 2.0;
    let overlap_label_y = cy_circles - r * 0.3;

    if !venn.overlaps.is_empty() {
        let ol = &venn.overlaps[0];
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            overlap_x, overlap_label_y, LABEL_FONT_SIZE, COLOR_DARK, escape_xml(&ol.label)
        ));
        render_items_at(&mut svg, overlap_x, overlap_label_y + ITEM_LINE_HEIGHT, &ol.items, COLOR_DARK);
    }

    let _ = label_offset;
    let _ = text_width("");

    svg.push_str("</svg>");
    svg
}

fn render_three_sets(venn: &Venn) -> String {
    let r = CIRCLE_RADIUS;
    let overlap_dist = r * 2.0 * (1.0 - OVERLAP_RATIO);

    // Triangular arrangement: top center, bottom-left, bottom-right
    let title_space = if venn.title.is_some() { TITLE_HEIGHT + TITLE_GAP } else { 0.0 };

    // Vertical offset for triangle
    let tri_h = overlap_dist * (3.0_f64).sqrt() / 2.0;

    let cx_top = PADDING + r + overlap_dist / 2.0;
    let cy_top = PADDING + title_space + r;

    let cx_bl = cx_top - overlap_dist / 2.0;
    let cy_bl = cy_top + tri_h;

    let cx_br = cx_top + overlap_dist / 2.0;
    let cy_br = cy_top + tri_h;

    let total_w = cx_br + r + PADDING;
    let total_h = cy_bl + r + PADDING;

    let mut svg = svg_header(total_w, total_h);

    // Title
    if let Some(ref title) = venn.title {
        let title_y = PADDING + TITLE_HEIGHT / 2.0 + 6.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>",
            total_w / 2.0,
            title_y,
            escape_xml(title)
        ));
    }

    let centers = [(cx_top, cy_top), (cx_bl, cy_bl), (cx_br, cy_br)];

    // Circles
    for (i, (cx, cy)) in centers.iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, cy, r, SET_COLORS[i], SET_COLORS[i]
        ));
    }

    // Set labels — positioned away from center
    let center_x = (cx_top + cx_bl + cx_br) / 3.0;
    let center_y = (cy_top + cy_bl + cy_br) / 3.0;

    for (i, (cx, cy)) in centers.iter().enumerate() {
        // Push label away from centroid
        let dx = cx - center_x;
        let dy = cy - center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let label_x = cx + dx / dist * r * 0.55;
        let label_y = cy + dy / dist * r * 0.55;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[i], escape_xml(&venn.sets[i].label)
        ));

        render_items_at(&mut svg, label_x, label_y + ITEM_LINE_HEIGHT, &venn.sets[i].items, COLOR_DARK);
    }

    // Overlaps rendered at centroid
    if !venn.overlaps.is_empty() {
        let ol = &venn.overlaps[0];
        let ol_y = center_y - (ol.items.len() as f64 * ITEM_LINE_HEIGHT) / 2.0;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            center_x, ol_y, LABEL_FONT_SIZE, COLOR_DARK, escape_xml(&ol.label)
        ));
        render_items_at(&mut svg, center_x, ol_y + ITEM_LINE_HEIGHT, &ol.items, COLOR_DARK);
    }

    svg.push_str("</svg>");
    svg
}

fn svg_header(width: f64, height: f64) -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));
    svg
}

fn render_items_at(svg: &mut String, x: f64, start_y: f64, items: &[String], color: &str) {
    for (i, item) in items.iter().enumerate() {
        let y = start_y + i as f64 * ITEM_LINE_HEIGHT;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"{}\">{}</text>",
            x, y, FONT_SIZE, color, escape_xml(item)
        ));
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

    let venn = match parse(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mdd-venn: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&venn));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_sets() {
        let input = r#"
title "Test"
set "A" {
  item1
  item2
}
set "B" {
  item3
}
overlap "A ∩ B" {
  shared
}
"#;
        let v = parse(input).unwrap();
        assert_eq!(v.title.as_deref(), Some("Test"));
        assert_eq!(v.sets.len(), 2);
        assert_eq!(v.sets[0].label, "A");
        assert_eq!(v.sets[0].items.len(), 2);
        assert_eq!(v.sets[1].label, "B");
        assert_eq!(v.sets[1].items.len(), 1);
        assert_eq!(v.overlaps.len(), 1);
        assert_eq!(v.overlaps[0].label, "A ∩ B");
        assert_eq!(v.overlaps[0].items[0], "shared");
    }

    #[test]
    fn parse_three_sets() {
        let input = r#"
set "X" {
  a
}
set "Y" {
  b
}
set "Z" {
  c
}
"#;
        let v = parse(input).unwrap();
        assert!(v.title.is_none());
        assert_eq!(v.sets.len(), 3);
        assert_eq!(v.sets[0].label, "X");
        assert_eq!(v.sets[1].label, "Y");
        assert_eq!(v.sets[2].label, "Z");
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
set "A" {
  item1
}
set "B" {
  item2
}
"#;
        let v = parse(input).unwrap();
        let svg = render_svg(&v);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
    }

    #[test]
    fn error_one_set() {
        let input = r#"
set "A" {
  item1
}
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_four_sets() {
        let input = r#"
set "A" { a }
set "B" { b }
set "C" { c }
set "D" { d }
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn error_unclosed() {
        let input = r#"
set "A" {
  item1
"#;
        assert!(parse(input).is_err());
    }
}
