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
    sets: Vec<SetDef>,
    overlaps: Vec<OverlapDef>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Venn, String> {
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

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const COLOR_DARK: &str = "#333";

const SET_COLORS: &[&str] = &["#1565c0", "#2e7d32", "#f57f17"];

const MIN_CIRCLE_RADIUS: f64 = 130.0;
const OVERLAP_RATIO: f64 = 0.4;
const PADDING: f64 = 60.0;
const LABEL_FONT_SIZE: f64 = 18.0;
const DESC_FONT_SIZE: f64 = 11.0;
const DESC_COLOR: &str = "#666";
const DESC_LINE_LEN: f64 = 30.0;
const DESC_LINE_HEIGHT: f64 = 15.0;

/// Compute circle radius based on label width.
fn set_radius(set: &SetDef) -> f64 {
    let label_w = text_width(&set.label);
    let content_r = label_w / 2.0 + 30.0;
    content_r.max(MIN_CIRCLE_RADIUS)
}

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

/// Render items radiating outward from a point.
/// `line_start` = distance from (cx,cy) where the line begins.
/// `text_start` = distance from (cx,cy) where the text begins.
fn render_items_outside(
    svg: &mut String,
    cx: f64,
    cy: f64,
    line_start: f64,
    text_start: f64,
    dir: (f64, f64),
    items: &[String],
    color: &str,
) {
    if items.is_empty() {
        return;
    }
    let ls_x = cx + dir.0 * line_start;
    let ls_y = cy + dir.1 * line_start;
    let le_x = cx + dir.0 * text_start;
    let le_y = cy + dir.1 * text_start;

    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" opacity=\"0.5\"/>",
        ls_x, ls_y, le_x, le_y, color
    ));

    let anchor = if dir.0 > 0.3 {
        "start"
    } else if dir.0 < -0.3 {
        "end"
    } else {
        "middle"
    };
    let text_gap = 6.0;
    let tx = le_x + dir.0 * text_gap;
    let ty = le_y + dir.1 * text_gap;

    for (i, line) in items.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            tx,
            ty + i as f64 * DESC_LINE_HEIGHT,
            anchor, DESC_FONT_SIZE, DESC_COLOR,
            escape_xml(line)
        ));
    }
}

fn render_svg(venn: &Venn) -> String {
    if venn.sets.len() == 2 {
        render_two_sets(venn)
    } else {
        render_three_sets(venn)
    }
}

fn render_two_sets(venn: &Venn) -> String {
    let has_items = venn.sets.iter().any(|s| !s.items.is_empty());
    let pad = if has_items { PADDING + 80.0 } else { PADDING };

    let r1 = set_radius(&venn.sets[0]);
    let r2 = set_radius(&venn.sets[1]);
    let r_avg = (r1 + r2) / 2.0;
    let overlap_dist = r_avg * 2.0 * (1.0 - OVERLAP_RATIO);

    let cx1 = pad + r1;
    let cx2 = cx1 + overlap_dist;
    let r_max = r1.max(r2);
    let cy_circles = pad + r_max;

    let total_w = cx2 + r2 + pad;
    let total_h = cy_circles + r_max + pad;

    let mut svg = svg_header(total_w, total_h);

    // Circles
    for (i, &(cx, cy, r)) in [(cx1, cy_circles, r1), (cx2, cy_circles, r2)].iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, cy, r, SET_COLORS[i], SET_COLORS[i]
        ));
    }

    // Set labels
    let set_a_label_x = cx1 - (r1 - overlap_dist / 2.0) / 2.0;
    let set_b_label_x = cx2 + (r2 - overlap_dist / 2.0) / 2.0;
    let label_y = cy_circles - r_max * 0.3;

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        set_a_label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[0], escape_xml(&venn.sets[0].label)
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        set_b_label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[1], escape_xml(&venn.sets[1].label)
    ));

    // Items radiating outward from each circle
    let venn_cx = (cx1 + cx2) / 2.0;
    let centers = [(cx1, cy_circles, r1), (cx2, cy_circles, r2)];
    for (i, &(cx, cy, r)) in centers.iter().enumerate() {
        let dx = cx - venn_cx;
        let dy = cy - cy_circles;
        let dist = (dx * dx + dy * dy).sqrt();
        let dir = if dist > 1.0 {
            (dx / dist, dy / dist)
        } else {
            (0.0, -1.0)
        };
        render_items_outside(&mut svg, cx, cy, r, r + DESC_LINE_LEN, dir, &venn.sets[i].items, SET_COLORS[i]);
    }

    // Overlap
    let overlap_x = (cx1 + cx2) / 2.0;
    let overlap_label_y = cy_circles;

    if !venn.overlaps.is_empty() {
        let ol = &venn.overlaps[0];
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            overlap_x, overlap_label_y, LABEL_FONT_SIZE, COLOR_DARK, escape_xml(&ol.label)
        ));
        // Overlap items radiate downward
        let dir = (0.0, 1.0);
        let edge_dist = r_max;
        render_items_outside(&mut svg, overlap_x, overlap_label_y, LABEL_FONT_SIZE, edge_dist + DESC_LINE_LEN, dir, &ol.items, COLOR_DARK);
    }

    svg.push_str("</svg>");
    svg
}

fn render_three_sets(venn: &Venn) -> String {
    let has_items = venn.sets.iter().any(|s| !s.items.is_empty());
    let pad = if has_items { PADDING + 80.0 } else { PADDING };

    let radii: Vec<f64> = venn.sets.iter().map(|s| set_radius(s)).collect();
    let r_avg = radii.iter().sum::<f64>() / 3.0;
    let r_max = radii.iter().copied().fold(0.0_f64, f64::max);
    let overlap_dist = r_avg * 2.0 * (1.0 - OVERLAP_RATIO);

    let tri_h = overlap_dist * (3.0_f64).sqrt() / 2.0;

    let cx_top = pad + r_max + overlap_dist / 2.0;
    let cy_top = pad + r_max;

    let cx_bl = cx_top - overlap_dist / 2.0;
    let cy_bl = cy_top + tri_h;

    let cx_br = cx_top + overlap_dist / 2.0;
    let cy_br = cy_top + tri_h;

    let total_w = cx_br + r_max + pad;
    let total_h = cy_bl + r_max + pad;

    let mut svg = svg_header(total_w, total_h);

    let centers = [(cx_top, cy_top), (cx_bl, cy_bl), (cx_br, cy_br)];

    // Circles with individual radii
    for (i, (cx, cy)) in centers.iter().enumerate() {
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" stroke=\"{}\" stroke-width=\"2\"/>",
            cx, cy, radii[i], SET_COLORS[i], SET_COLORS[i]
        ));
    }

    // Set labels
    let center_x = (cx_top + cx_bl + cx_br) / 3.0;
    let center_y = (cy_top + cy_bl + cy_br) / 3.0;

    // Fixed outward directions: top→upper-right, bottom-left→lower-left, bottom-right→lower-right
    let s = std::f64::consts::FRAC_1_SQRT_2; // 0.707
    let set_dirs = [(s, -s), (-s, s), (s, s)];

    for (i, (cx, cy)) in centers.iter().enumerate() {
        let dx = cx - center_x;
        let dy = cy - center_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let label_dir = (dx / dist, dy / dist);
        let label_x = cx + label_dir.0 * radii[i] * 0.55;
        let label_y = cy + label_dir.1 * radii[i] * 0.55;

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            label_x, label_y, LABEL_FONT_SIZE, SET_COLORS[i], escape_xml(&venn.sets[i].label)
        ));

        // Items radiating outward in fixed directions
        render_items_outside(&mut svg, *cx, *cy, radii[i], radii[i] + DESC_LINE_LEN, set_dirs[i], &venn.sets[i].items, SET_COLORS[i]);
    }

    // Overlap: label at centroid, items radiate upper-left
    if !venn.overlaps.is_empty() {
        let ol = &venn.overlaps[0];

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            center_x, center_y, LABEL_FONT_SIZE, COLOR_DARK, escape_xml(&ol.label)
        ));

        // Distance from centroid to farthest circle edge
        let max_edge_dist = centers.iter().enumerate()
            .map(|(i, (cx, cy))| {
                let dx = cx - center_x;
                let dy = cy - center_y;
                (dx * dx + dy * dy).sqrt() + radii[i]
            })
            .fold(0.0_f64, f64::max);

        let overlap_dir = (-s, -s); // upper-left
        render_items_outside(&mut svg, center_x, center_y, LABEL_FONT_SIZE, max_edge_dist + DESC_LINE_LEN, overlap_dir, &ol.items, COLOR_DARK);
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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-venn - Render a Venn diagram as SVG

Usage: mdd-venn < input.venn

Define 2 or 3 sets with \"set Name { items... }\".
Items are displayed outside the circle with radiating lines.
Optionally define overlapping items with \"overlap Name { items... }\".

Example:
  set \"Frontend\" {
    HTML/CSS
    React
  }
  set \"Backend\" {
    DB design
    API design
  }
  overlap \"Shared\" {
    TypeScript
    Git
  }
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
