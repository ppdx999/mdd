use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ColorDef {
    text: String,
    bg: String,
}

#[derive(Debug, Clone)]
struct Segment {
    label: String,
    value: f64,
}

#[derive(Debug)]
struct Bar {
    label: String,
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
enum WaterfallKind {
    Item,
    Subtotal,
}

#[derive(Debug)]
struct WaterfallEntry {
    label: String,
    value: f64,
    kind: WaterfallKind,
}

#[derive(Debug)]
enum ChartData {
    StackedBar { bars: Vec<Bar> },
    Waterfall { entries: Vec<WaterfallEntry> },
}

#[derive(Debug)]
struct Diagram {
    data: ChartData,
    colors: HashMap<String, ColorDef>,
}

// ---------------------------------------------------------------------------
// Named colors
// ---------------------------------------------------------------------------

fn resolve_color(name: &str) -> String {
    match name.trim() {
        "red" => "#c62828".to_string(),
        "blue" => "#1565c0".to_string(),
        "green" => "#2e7d32".to_string(),
        "amber" | "yellow" => "#f57f17".to_string(),
        "orange" => "#e65100".to_string(),
        "teal" => "#00695c".to_string(),
        "purple" => "#6a1b9a".to_string(),
        "pink" => "#ad1457".to_string(),
        "grey" | "gray" => "#9e9e9e".to_string(),
        other => other.to_string(),
    }
}

fn default_palette() -> Vec<(&'static str, &'static str)> {
    vec![
        ("#1565c0", "#e3f2fd"),
        ("#2e7d32", "#e8f5e9"),
        ("#e65100", "#fff3e0"),
        ("#6a1b9a", "#f3e5f5"),
        ("#00695c", "#e0f2f1"),
        ("#c62828", "#ffebee"),
        ("#f57f17", "#fff8e1"),
        ("#ad1457", "#fce4ec"),
    ]
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut chart_type: Option<String> = None;
    let mut colors: HashMap<String, ColorDef> = HashMap::new();
    let mut bars: Vec<Bar> = Vec::new();
    let mut entries: Vec<WaterfallEntry> = Vec::new();
    for (line_no, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // type <chart-type>
        if line.starts_with("type ") {
            let t = line.strip_prefix("type ").unwrap().trim().to_string();
            if t != "stacked-bar" && t != "waterfall" {
                return Err(format!(
                    "Line {}: Unknown chart type '{}'. Use 'stacked-bar' or 'waterfall'",
                    line_no + 1,
                    t
                ));
            }
            chart_type = Some(t);
            continue;
        }

        // color <name> : <text_color>, <bg_color>
        if line.starts_with("color ") {
            let rest = line.strip_prefix("color ").unwrap();
            if let Some((name, colors_str)) = rest.split_once(" : ") {
                let parts: Vec<&str> = colors_str.split(',').collect();
                let text_color = resolve_color(parts[0].trim());
                let bg_color = if parts.len() > 1 {
                    resolve_color(parts[1].trim())
                } else {
                    "#fff".to_string()
                };
                colors.insert(
                    name.trim().to_string(),
                    ColorDef {
                        text: text_color,
                        bg: bg_color,
                    },
                );
                continue;
            }
            return Err(format!("Line {}: Invalid color syntax: {}", line_no + 1, line));
        }

        let ct = chart_type
            .as_deref()
            .ok_or_else(|| format!("Line {}: 'type' must be declared first", line_no + 1))?;

        match ct {
            "stacked-bar" => {
                // bar <label> : <seg1> <val1>, <seg2> <val2>, ...
                if line.starts_with("bar ") {
                    let rest = line.strip_prefix("bar ").unwrap();
                    if let Some((label, segs_str)) = rest.split_once(" : ") {
                        let segments = parse_segments(segs_str, line_no)?;
                        bars.push(Bar {
                            label: label.trim().to_string(),
                            segments,
                        });
                        continue;
                    }
                    return Err(format!("Line {}: Invalid bar syntax: {}", line_no + 1, line));
                }
                return Err(format!("Line {}: Unknown syntax: {}", line_no + 1, line));
            }
            "waterfall" => {
                // subtotal <label>
                if line.starts_with("subtotal ") {
                    let label = line.strip_prefix("subtotal ").unwrap().trim().to_string();
                    entries.push(WaterfallEntry {
                        label,
                        value: 0.0, // computed later
                        kind: WaterfallKind::Subtotal,
                    });
                    continue;
                }
                // item <label> : <value>
                if line.starts_with("item ") {
                    let rest = line.strip_prefix("item ").unwrap();
                    if let Some((label, val_str)) = rest.split_once(" : ") {
                        let value: f64 = val_str.trim().parse().map_err(|_| {
                            format!("Line {}: Invalid number '{}'", line_no + 1, val_str.trim())
                        })?;
                        entries.push(WaterfallEntry {
                            label: label.trim().to_string(),
                            value,
                            kind: WaterfallKind::Item,
                        });
                        continue;
                    }
                    return Err(format!("Line {}: Invalid item syntax: {}", line_no + 1, line));
                }
                return Err(format!("Line {}: Unknown syntax: {}", line_no + 1, line));
            }
            _ => unreachable!(),
        }
    }

    let ct = chart_type.ok_or("Missing 'type' declaration")?;

    let data = match ct.as_str() {
        "stacked-bar" => {
            if bars.is_empty() {
                return Err("No 'bar' entries defined".to_string());
            }
            ChartData::StackedBar { bars }
        }
        "waterfall" => {
            if entries.is_empty() {
                return Err("No 'item' entries defined".to_string());
            }
            // Compute subtotal values
            let mut running = 0.0;
            for entry in &mut entries {
                match entry.kind {
                    WaterfallKind::Item => {
                        running += entry.value;
                    }
                    WaterfallKind::Subtotal => {
                        entry.value = running;
                    }
                }
            }
            ChartData::Waterfall { entries }
        }
        _ => unreachable!(),
    };

    Ok(Diagram {
        data,
        colors,
    })
}

fn parse_segments(s: &str, line_no: usize) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        // Find the last space to split label from value
        if let Some(pos) = part.rfind(' ') {
            let label = part[..pos].trim().to_string();
            let val_str = part[pos + 1..].trim();
            let value: f64 = val_str.parse().map_err(|_| {
                format!("Line {}: Invalid number '{}'", line_no + 1, val_str)
            })?;
            segments.push(Segment { label, value });
        } else {
            return Err(format!(
                "Line {}: Invalid segment '{}'. Expected '<label> <value>'",
                line_no + 1,
                part
            ));
        }
    }
    Ok(segments)
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const JP_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const PADDING: f64 = 20.0;
const BAR_HEIGHT: f64 = 32.0;
const BAR_GAP: f64 = 12.0;
const LABEL_GAP: f64 = 12.0;
const LEGEND_SWATCH: f64 = 12.0;
const LEGEND_GAP: f64 = 6.0;
const LEGEND_ITEM_GAP: f64 = 20.0;
const COLOR_DARK: &str = "#333";
const COLOR_AXIS: &str = "#999";
const COLOR_SUBTOTAL_BG: &str = "#e8eaf6";
const COLOR_SUBTOTAL_TEXT: &str = "#333";
const COLOR_POS_BG: &str = "#e8f5e9";
const COLOR_POS_TEXT: &str = "#2e7d32";
const COLOR_NEG_BG: &str = "#ffebee";
const COLOR_NEG_TEXT: &str = "#c62828";

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { JP_CHAR_WIDTH })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn format_number(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    match &diagram.data {
        ChartData::StackedBar { bars } => render_stacked_bar(bars, &diagram.colors),
        ChartData::Waterfall { entries } => {
            render_waterfall(entries, &diagram.colors)
        }
    }
}

// ---------------------------------------------------------------------------
// Stacked bar rendering
// ---------------------------------------------------------------------------

fn render_stacked_bar(
    bars: &[Bar],
    colors: &HashMap<String, ColorDef>,
) -> String {
    let palette = default_palette();

    // Collect all unique segment labels in order
    let mut seg_labels: Vec<String> = Vec::new();
    for bar in bars {
        for seg in &bar.segments {
            if !seg_labels.contains(&seg.label) {
                seg_labels.push(seg.label.clone());
            }
        }
    }

    // Build color map for segments
    let seg_colors: HashMap<String, (String, String)> = seg_labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if let Some(def) = colors.get(label) {
                (label.clone(), (def.text.clone(), def.bg.clone()))
            } else {
                let (text, bg) = palette[i % palette.len()];
                (label.clone(), (text.to_string(), bg.to_string()))
            }
        })
        .collect();

    // Compute max total for scaling
    let max_total: f64 = bars
        .iter()
        .map(|b| b.segments.iter().map(|s| s.value).sum::<f64>())
        .fold(0.0_f64, f64::max);

    // Layout
    let label_w = bars
        .iter()
        .map(|b| text_width(&b.label))
        .fold(60.0_f64, f64::max)
        + LABEL_GAP;

    let chart_w = 400.0;
    let legend_h = 30.0;
    let chart_h = bars.len() as f64 * (BAR_HEIGHT + BAR_GAP) - BAR_GAP;
    let total_w = PADDING * 2.0 + label_w + chart_w;
    let total_h = PADDING * 2.0 + chart_h + legend_h + 10.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let y_offset = PADDING;

    let chart_x = PADDING + label_w;

    // Bars
    for (i, bar) in bars.iter().enumerate() {
        let by = y_offset + i as f64 * (BAR_HEIGHT + BAR_GAP);
        let total: f64 = bar.segments.iter().map(|s| s.value).sum();

        // Label
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-weight=\"bold\">{}</text>",
            chart_x - LABEL_GAP,
            by + BAR_HEIGHT / 2.0 + 5.0,
            escape_xml(&bar.label)
        ));

        // Segments
        let mut sx = chart_x;
        for seg in &bar.segments {
            let w = if max_total > 0.0 {
                (seg.value / max_total) * chart_w
            } else {
                0.0
            };
            let (text_c, bg_c) = seg_colors
                .get(&seg.label)
                .cloned()
                .unwrap_or_else(|| (COLOR_DARK.to_string(), "#e3f2fd".to_string()));

            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\"/>",
                sx, by, w, BAR_HEIGHT, bg_c
            ));

            // Value text inside bar (only if wide enough)
            let val_text = format_number(seg.value);
            let val_w = text_width(&val_text);
            if w > val_w + 8.0 {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    sx + w / 2.0,
                    by + BAR_HEIGHT / 2.0 + 5.0,
                    text_c,
                    val_text
                ));
            }

            sx += w;
        }

        // Total label at end
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            sx + 6.0,
            by + BAR_HEIGHT / 2.0 + 5.0,
            COLOR_AXIS,
            format_number(total)
        ));
    }

    // Legend
    let legend_y = y_offset + chart_h + 20.0;
    let mut lx = chart_x;
    for label in &seg_labels {
        let (text_c, bg_c) = seg_colors
            .get(label)
            .cloned()
            .unwrap_or_else(|| (COLOR_DARK.to_string(), "#e3f2fd".to_string()));

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{}\"/>",
            lx,
            legend_y,
            LEGEND_SWATCH,
            LEGEND_SWATCH,
            bg_c
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            lx + LEGEND_SWATCH + LEGEND_GAP,
            legend_y + LEGEND_SWATCH - 1.0,
            text_c,
            escape_xml(label)
        ));
        lx += LEGEND_SWATCH + LEGEND_GAP + text_width(label) + LEGEND_ITEM_GAP;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Waterfall rendering
// ---------------------------------------------------------------------------

fn render_waterfall(
    entries: &[WaterfallEntry],
    colors: &HashMap<String, ColorDef>,
) -> String {
    // Resolve colors
    let pos_bg = colors
        .get("positive")
        .map(|c| c.bg.clone())
        .unwrap_or_else(|| COLOR_POS_BG.to_string());
    let pos_text = colors
        .get("positive")
        .map(|c| c.text.clone())
        .unwrap_or_else(|| COLOR_POS_TEXT.to_string());
    let neg_bg = colors
        .get("negative")
        .map(|c| c.bg.clone())
        .unwrap_or_else(|| COLOR_NEG_BG.to_string());
    let neg_text = colors
        .get("negative")
        .map(|c| c.text.clone())
        .unwrap_or_else(|| COLOR_NEG_TEXT.to_string());
    let sub_bg = colors
        .get("subtotal")
        .map(|c| c.bg.clone())
        .unwrap_or_else(|| COLOR_SUBTOTAL_BG.to_string());
    let sub_text = colors
        .get("subtotal")
        .map(|c| c.text.clone())
        .unwrap_or_else(|| COLOR_SUBTOTAL_TEXT.to_string());

    // Find min/max running totals for scale
    let mut running = 0.0_f64;
    let mut min_val = 0.0_f64;
    let mut max_val = 0.0_f64;
    let mut positions: Vec<(f64, f64)> = Vec::new(); // (start, end) for each entry

    for entry in entries {
        match entry.kind {
            WaterfallKind::Item => {
                let start = running;
                running += entry.value;
                positions.push((start, running));
            }
            WaterfallKind::Subtotal => {
                positions.push((0.0, running));
            }
        }
        min_val = min_val.min(running).min(0.0);
        max_val = max_val.max(running);
    }

    let range = max_val - min_val;
    let range = if range < 1.0 { 1.0 } else { range };

    // Layout
    let label_w = entries
        .iter()
        .map(|e| text_width(&e.label))
        .fold(60.0_f64, f64::max)
        + LABEL_GAP;

    let chart_w = 400.0;
    let chart_h = entries.len() as f64 * (BAR_HEIGHT + BAR_GAP) - BAR_GAP;
    let total_w = PADDING * 2.0 + label_w + chart_w + 60.0;
    let total_h = PADDING * 2.0 + chart_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\
         <style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let y_offset = PADDING;

    let chart_x = PADDING + label_w;

    // Scale helper
    let to_x = |val: f64| -> f64 { chart_x + ((val - min_val) / range) * chart_w };

    // Zero line
    let zero_x = to_x(0.0);
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>",
        zero_x,
        y_offset - 4.0,
        zero_x,
        y_offset + chart_h + 4.0,
        COLOR_AXIS
    ));

    // Entries
    for (i, entry) in entries.iter().enumerate() {
        let by = y_offset + i as f64 * (BAR_HEIGHT + BAR_GAP);
        let (start, end) = positions[i];

        // Label
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-weight=\"bold\">{}</text>",
            chart_x - LABEL_GAP,
            by + BAR_HEIGHT / 2.0 + 5.0,
            escape_xml(&entry.label)
        ));

        // Bar
        let (bg, text_c) = match entry.kind {
            WaterfallKind::Subtotal => (sub_bg.clone(), sub_text.clone()),
            WaterfallKind::Item => {
                if entry.value >= 0.0 {
                    (pos_bg.clone(), pos_text.clone())
                } else {
                    (neg_bg.clone(), neg_text.clone())
                }
            }
        };

        let x1 = to_x(start.min(end));
        let x2 = to_x(start.max(end));
        let bar_w = (x2 - x1).max(2.0);

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\"/>",
            x1, by, bar_w, BAR_HEIGHT, bg
        ));

        // Value text
        let val_text = match entry.kind {
            WaterfallKind::Subtotal => format_number(entry.value),
            WaterfallKind::Item => {
                if entry.value >= 0.0 {
                    format!("+{}", format_number(entry.value))
                } else {
                    format_number(entry.value)
                }
            }
        };

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            x2 + 6.0,
            by + BAR_HEIGHT / 2.0 + 5.0,
            text_c,
            val_text
        ));

        // Connector line to next entry
        if i + 1 < entries.len() {
            let next_by = by + BAR_HEIGHT + BAR_GAP;
            let conn_x = to_x(end);
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"3,2\"/>",
                conn_x,
                by + BAR_HEIGHT,
                conn_x,
                next_by,
                COLOR_AXIS
            ));
        }
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
            eprintln!("mdd-analysis: {}", e);
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
    fn parse_stacked_bar_basic() {
        let input = "type stacked-bar\nbar Q1 : A 100, B 200\n";
        let d = parse(input).unwrap();
        match &d.data {
            ChartData::StackedBar { bars } => {
                assert_eq!(bars.len(), 1);
                assert_eq!(bars[0].label, "Q1");
                assert_eq!(bars[0].segments.len(), 2);
                assert_eq!(bars[0].segments[0].label, "A");
                assert_eq!(bars[0].segments[0].value, 100.0);
            }
            _ => panic!("Expected StackedBar"),
        }
    }

    #[test]
    fn parse_stacked_bar_multiple() {
        let input = "type stacked-bar\nbar Q1 : A 100, B 200\nbar Q2 : A 150, B 180\n";
        let d = parse(input).unwrap();
        match &d.data {
            ChartData::StackedBar { bars } => assert_eq!(bars.len(), 2),
            _ => panic!("Expected StackedBar"),
        }
    }

    #[test]
    fn parse_waterfall_basic() {
        let input = "type waterfall\nitem 売上 : 1000\nitem 原価 : -400\nsubtotal 粗利\n";
        let d = parse(input).unwrap();
        match &d.data {
            ChartData::Waterfall { entries } => {
                assert_eq!(entries.len(), 3);
                assert_eq!(entries[0].value, 1000.0);
                assert_eq!(entries[1].value, -400.0);
                assert_eq!(entries[2].value, 600.0); // subtotal computed
            }
            _ => panic!("Expected Waterfall"),
        }
    }

    #[test]
    fn parse_with_colors() {
        let input = "type stacked-bar\ncolor A : blue, #e3f2fd\nbar Q1 : A 100\n";
        let d = parse(input).unwrap();
        assert!(d.colors.contains_key("A"));
        assert_eq!(d.colors["A"].text, "#1565c0");
        assert_eq!(d.colors["A"].bg, "#e3f2fd");
    }

    #[test]
    fn parse_missing_type_error() {
        let result = parse("bar Q1 : A 100\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unknown_type_error() {
        let result = parse("type pie\nbar Q1 : A 100\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_no_bars_error() {
        let result = parse("type stacked-bar\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_no_items_error() {
        let result = parse("type waterfall\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_japanese_labels() {
        let input = "type stacked-bar\nbar 第1四半期 : 製品A 300, 製品B 200\n";
        let d = parse(input).unwrap();
        match &d.data {
            ChartData::StackedBar { bars } => {
                assert_eq!(bars[0].label, "第1四半期");
                assert_eq!(bars[0].segments[0].label, "製品A");
            }
            _ => panic!("Expected StackedBar"),
        }
    }

    #[test]
    fn render_stacked_bar_svg() {
        let input = "type stacked-bar\nbar Q1 : A 100, B 200\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn render_waterfall_svg() {
        let input = "type waterfall\nitem 売上 : 1000\nitem 原価 : -400\nsubtotal 粗利\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("fill=\"white\""));
    }

    #[test]
    fn waterfall_subtotal_computation() {
        let input = "type waterfall\n\
                      item A : 100\n\
                      item B : -30\n\
                      subtotal S1\n\
                      item C : -20\n\
                      subtotal S2\n";
        let d = parse(input).unwrap();
        match &d.data {
            ChartData::Waterfall { entries } => {
                assert_eq!(entries[2].value, 70.0);  // S1 = 100 - 30
                assert_eq!(entries[4].value, 50.0);  // S2 = 70 - 20
            }
            _ => panic!("Expected Waterfall"),
        }
    }
}
