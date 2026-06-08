use std::io::{self, Read};

#[derive(Debug)]
struct Slice { label: String, value: f64 }
#[derive(Debug)]
struct Pie { slices: Vec<Slice> }

fn parse(input: &str) -> Result<Pie, String> {
    let mut slices = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("slice ") {
            let rest = t.strip_prefix("slice ").unwrap().trim();
            if let Some((label, val)) = rest.split_once(" : ") {
                let v = val.trim().parse::<f64>().map_err(|e| format!("Invalid value: {}", e))?;
                slices.push(Slice { label: strip_quotes(label.trim()).to_string(), value: v });
            } else { return Err(format!("Missing value: {}", t)); }
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if slices.len() < 2 { return Err("At least 2 slices required".to_string()); }
    Ok(Pie { slices })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

const RADIUS: f64 = 120.0;
const PADDING: f64 = 40.0;
const LEGEND_W: f64 = 160.0;
const CHAR_W: f64 = 8.0;
const CJK_W: f64 = 14.0;

const COLORS: &[&str] = &[
    "#1565c0", "#2e7d32", "#f57f17", "#7b1fa2", "#00695c", "#c62828", "#283593", "#e65100",
];

fn text_width(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CHAR_W } else { CJK_W }).sum() }
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn render_svg(pie: &Pie) -> String {
    let cx = PADDING + RADIUS;
    let cy = PADDING + RADIUS;
    let total = pie.slices.iter().map(|s| s.value).sum::<f64>();

    let legend_w = pie.slices.iter().map(|s| text_width(&s.label) + 40.0).fold(LEGEND_W, f64::max);
    let total_w = cx + RADIUS + 40.0 + legend_w + PADDING;
    let total_h = cy + RADIUS + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 12px; fill: #333; }</style>");

    let mut start_angle = -std::f64::consts::FRAC_PI_2;
    for (i, slice) in pie.slices.iter().enumerate() {
        let frac = slice.value / total;
        let sweep = frac * std::f64::consts::TAU;
        let end_angle = start_angle + sweep;
        let color = COLORS[i % COLORS.len()];

        let x1 = cx + RADIUS * start_angle.cos();
        let y1 = cy + RADIUS * start_angle.sin();
        let x2 = cx + RADIUS * end_angle.cos();
        let y2 = cy + RADIUS * end_angle.sin();
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };

        svg.push_str(&format!(
            "<path d=\"M{},{} L{},{} A{},{} 0 {},1 {},{} Z\" fill=\"{}\" stroke=\"white\" stroke-width=\"2\"/>",
            cx, cy, x1, y1, RADIUS, RADIUS, large, x2, y2, color
        ));

        // Percentage label
        let mid_angle = start_angle + sweep / 2.0;
        let label_r = RADIUS * 0.65;
        let lx = cx + label_r * mid_angle.cos();
        let ly = cy + label_r * mid_angle.sin();
        let pct = format!("{:.0}%", frac * 100.0);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"white\">{}</text>",
            lx, ly + 4.0, pct
        ));
        start_angle = end_angle;
    }

    // Legend
    let legend_x = cx + RADIUS + 40.0;
    let mut ly = cy - (pie.slices.len() as f64 * 22.0) / 2.0;
    for (i, slice) in pie.slices.iter().enumerate() {
        let color = COLORS[i % COLORS.len()];
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"14\" rx=\"3\" fill=\"{}\"/>",
            legend_x, ly - 10.0, color
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>",
            legend_x + 20.0, ly, escape_xml(&slice.label)
        ));
        ly += 22.0;
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(p) => print!("{}", render_svg(&p)),
        Err(e) => { eprintln!("mdd-pie: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "slice A : 60\nslice B : 40\n";
        let p = parse(input).unwrap();
        assert_eq!(p.slices.len(), 2);
    }
    #[test]
    fn render_output() {
        let input = "slice A : 60\nslice B : 40\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.contains("<path"));
    }
}
