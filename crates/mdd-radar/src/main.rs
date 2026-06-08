use std::io::{self, Read};

#[derive(Debug)]
struct Dataset {
    name: String,
    values: Vec<f64>,
}

#[derive(Debug)]
struct Radar {
    axes: Vec<String>,
    datasets: Vec<Dataset>,
}

fn parse(input: &str) -> Result<Radar, String> {
    let mut axes: Vec<String> = Vec::new();
    let mut datasets: Vec<Dataset> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if trimmed.starts_with("axis ") {
            axes.push(strip_quotes(trimmed.strip_prefix("axis ").unwrap().trim()).to_string());
            continue;
        }
        if trimmed.starts_with("data ") {
            let rest = trimmed.strip_prefix("data ").unwrap().trim();
            if let Some((name, vals)) = rest.split_once(" : ") {
                let values: Result<Vec<f64>, _> = vals.split(',').map(|v| v.trim().parse::<f64>()).collect();
                let values = values.map_err(|e| format!("Invalid value: {}", e))?;
                datasets.push(Dataset { name: strip_quotes(name.trim()).to_string(), values });
            } else {
                return Err(format!("Missing values in data: {}", trimmed));
            }
            continue;
        }
        return Err(format!("Unknown syntax: {}", trimmed));
    }
    if axes.len() < 3 { return Err("At least 3 axes required".to_string()); }
    if datasets.is_empty() { return Err("At least 1 dataset required".to_string()); }
    for ds in &datasets {
        if ds.values.len() != axes.len() {
            return Err(format!("Dataset '{}' has {} values but {} axes", ds.name, ds.values.len(), axes.len()));
        }
    }
    Ok(Radar { axes, datasets })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

const CHAR_W: f64 = 8.0;
const CJK_W: f64 = 14.0;
const RADIUS: f64 = 140.0;
const PADDING: f64 = 80.0;
const GRID_LEVELS: usize = 5;

const COLORS: &[(&str, &str)] = &[
    ("#1565c0", "rgba(21,101,192,0.15)"), ("#c62828", "rgba(198,40,40,0.15)"),
    ("#2e7d32", "rgba(46,125,50,0.15)"), ("#f57f17", "rgba(245,127,23,0.15)"),
];

fn text_width(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CHAR_W } else { CJK_W }).sum() }
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn render_svg(radar: &Radar) -> String {
    let n = radar.axes.len();
    let angle_step = std::f64::consts::TAU / n as f64;

    let max_label_w = radar.axes.iter().map(|a| text_width(a)).fold(0.0_f64, f64::max);
    let pad = PADDING + max_label_w * 0.5;

    let cx = pad + RADIUS;
    let cy = pad + RADIUS;
    let total_w = (pad + RADIUS) * 2.0;
    let total_h = cy + RADIUS + pad;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 12px; fill: #333; }</style>");

    // Grid circles
    for level in 1..=GRID_LEVELS {
        let r = RADIUS * level as f64 / GRID_LEVELS as f64;
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"#e0e0e0\" stroke-width=\"0.5\"/>",
            cx, cy, r
        ));
    }

    // Axis lines and labels
    for i in 0..n {
        let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
        let ex = cx + RADIUS * angle.cos();
        let ey = cy + RADIUS * angle.sin();
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"0.5\"/>",
            cx, cy, ex, ey
        ));
        let lx = cx + (RADIUS + 14.0) * angle.cos();
        let ly = cy + (RADIUS + 14.0) * angle.sin();
        let anchor = if angle.cos().abs() < 0.3 { "middle" } else if angle.cos() > 0.0 { "start" } else { "end" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-size=\"12\">{}</text>",
            lx, ly + 4.0, anchor, escape_xml(&radar.axes[i])
        ));
    }

    // Datasets
    for (di, ds) in radar.datasets.iter().enumerate() {
        let (stroke, fill) = COLORS[di % COLORS.len()];
        let mut points = String::new();
        for (i, &val) in ds.values.iter().enumerate() {
            let r = RADIUS * (val / 100.0).min(1.0).max(0.0);
            let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
            let px = cx + r * angle.cos();
            let py = cy + r * angle.sin();
            if i > 0 { points.push(' '); }
            points.push_str(&format!("{},{}", px, py));
        }
        svg.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            points, fill, stroke
        ));
        // Dots
        for (i, &val) in ds.values.iter().enumerate() {
            let r = RADIUS * (val / 100.0).min(1.0).max(0.0);
            let angle = -std::f64::consts::FRAC_PI_2 + angle_step * i as f64;
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"{}\"/>",
                cx + r * angle.cos(), cy + r * angle.sin(), stroke
            ));
        }
    }

    // Legend
    if radar.datasets.len() > 1 {
        let mut ly = cy + RADIUS + 30.0;
        for (di, ds) in radar.datasets.iter().enumerate() {
            let (stroke, _) = COLORS[di % COLORS.len()];
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{}\"/>",
                cx - 60.0, ly - 10.0, stroke
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\">{}</text>",
                cx - 44.0, ly, escape_xml(&ds.name)
            ));
            ly += 18.0;
        }
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(r) => print!("{}", render_svg(&r)),
        Err(e) => { eprintln!("mdd-radar: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "axis A\naxis B\naxis C\ndata \"X\" : 80, 60, 90\n";
        let r = parse(input).unwrap();
        assert_eq!(r.axes.len(), 3);
        assert_eq!(r.datasets[0].values, vec![80.0, 60.0, 90.0]);
    }
    #[test]
    fn render_output() {
        let input = "axis A\naxis B\naxis C\ndata X : 80, 60, 90\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("polygon"));
    }
}
