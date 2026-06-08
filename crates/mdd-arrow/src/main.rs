use std::io::{self, Read};

#[derive(Debug, PartialEq)]
enum Direction { Down, Up, Right, Left }

#[derive(Debug)]
struct Arrow {
    direction: Direction,
    label: Option<String>,
}

fn parse(input: &str) -> Result<Arrow, String> {
    let mut direction = Direction::Down;
    let mut label = None;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("direction ") {
            let d = t.strip_prefix("direction ").unwrap().trim();
            direction = match d {
                "down" => Direction::Down,
                "up" => Direction::Up,
                "right" => Direction::Right,
                "left" => Direction::Left,
                _ => return Err(format!("Unknown direction: {} (use down/up/right/left)", d)),
            };
            continue;
        }
        if t.starts_with("label ") {
            label = Some(sq(t.strip_prefix("label ").unwrap().trim()).to_string());
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }

    Ok(Arrow { direction, label })
}

fn sq(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const LABEL_SIZE: f64 = 14.0;
const ARROW_LEN: f64 = 60.0;
const ARROW_COLOR: &str = "#1565c0";
const LABEL_COLOR: &str = "#1565c0";
const LABEL_GAP: f64 = 8.0;
const PAD: f64 = 12.0;
const HEAD_SIZE: f64 = 8.0;

fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn render_svg(arrow: &Arrow) -> String {
    let label_w = arrow.label.as_ref().map(|l| tw(l) * (LABEL_SIZE / 13.0)).unwrap_or(0.0);
    let label_h = if arrow.label.is_some() { LABEL_SIZE + 4.0 } else { 0.0 };

    let (svg_w, svg_h, x1, y1, x2, y2, hx, hy, label_x, label_y, anchor) = match arrow.direction {
        Direction::Down => {
            let w = (label_w + LABEL_GAP * 2.0 + HEAD_SIZE * 2.0).max(HEAD_SIZE * 4.0) + PAD * 2.0;
            let h = ARROW_LEN + PAD * 2.0;
            let cx = w / 2.0;
            let top = PAD;
            let bot = h - PAD;
            (w, h, cx, top, cx, bot - HEAD_SIZE, cx, bot,
             cx + HEAD_SIZE + LABEL_GAP, (top + bot) / 2.0 + LABEL_SIZE * 0.35, "start")
        }
        Direction::Up => {
            let w = (label_w + LABEL_GAP * 2.0 + HEAD_SIZE * 2.0).max(HEAD_SIZE * 4.0) + PAD * 2.0;
            let h = ARROW_LEN + PAD * 2.0;
            let cx = w / 2.0;
            let top = PAD;
            let bot = h - PAD;
            (w, h, cx, bot, cx, top + HEAD_SIZE, cx, top,
             cx + HEAD_SIZE + LABEL_GAP, (top + bot) / 2.0 + LABEL_SIZE * 0.35, "start")
        }
        Direction::Right => {
            let w = ARROW_LEN + PAD * 2.0;
            let h = (label_h + LABEL_GAP + HEAD_SIZE * 2.0).max(HEAD_SIZE * 4.0) + PAD * 2.0;
            let cy = h / 2.0;
            let left = PAD;
            let right = w - PAD;
            (w, h, left, cy, right - HEAD_SIZE, cy, right, cy,
             (left + right) / 2.0, cy - HEAD_SIZE - LABEL_GAP, "middle")
        }
        Direction::Left => {
            let w = ARROW_LEN + PAD * 2.0;
            let h = (label_h + LABEL_GAP + HEAD_SIZE * 2.0).max(HEAD_SIZE * 4.0) + PAD * 2.0;
            let cy = h / 2.0;
            let left = PAD;
            let right = w - PAD;
            (w, h, right, cy, left + HEAD_SIZE, cy, left, cy,
             (left + right) / 2.0, cy - HEAD_SIZE - LABEL_GAP, "middle")
        }
    };

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_w, svg_h, svg_w, svg_h
    );

    // Arrow line
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2.5\"/>",
        x1, y1, x2, y2, ARROW_COLOR
    ));

    // Arrowhead
    let (p1, p2) = match arrow.direction {
        Direction::Down => (
            format!("{},{}", hx - HEAD_SIZE, hy - HEAD_SIZE * 1.5),
            format!("{},{}", hx + HEAD_SIZE, hy - HEAD_SIZE * 1.5),
        ),
        Direction::Up => (
            format!("{},{}", hx - HEAD_SIZE, hy + HEAD_SIZE * 1.5),
            format!("{},{}", hx + HEAD_SIZE, hy + HEAD_SIZE * 1.5),
        ),
        Direction::Right => (
            format!("{},{}", hx - HEAD_SIZE * 1.5, hy - HEAD_SIZE),
            format!("{},{}", hx - HEAD_SIZE * 1.5, hy + HEAD_SIZE),
        ),
        Direction::Left => (
            format!("{},{}", hx + HEAD_SIZE * 1.5, hy - HEAD_SIZE),
            format!("{},{}", hx + HEAD_SIZE * 1.5, hy + HEAD_SIZE),
        ),
    };
    svg.push_str(&format!(
        "<polygon points=\"{},{} {} {}\" fill=\"{}\"/>",
        hx, hy, p1, p2, ARROW_COLOR
    ));

    // Label
    if let Some(ref label) = arrow.label {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            label_x, label_y, anchor, LABEL_SIZE, LABEL_COLOR, ex(label)
        ));
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-arrow - Render a directional arrow as SVG

Usage: mdd-arrow < input.arrow

Directives:
  direction <down|up|right|left>   Arrow direction (default: down)
  label \"<text>\"                   Optional label alongside the arrow

Example:
  direction down
  label \"next step\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(a) => print!("{}", render_svg(&a)),
        Err(e) => { eprintln!("mdd-arrow: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default() {
        let input = "label \"test\"\n";
        let a = parse(input).unwrap();
        assert_eq!(a.direction, Direction::Down);
        assert_eq!(a.label.as_deref(), Some("test"));
    }

    #[test]
    fn parse_directions() {
        for d in ["down", "up", "right", "left"] {
            let input = format!("direction {}\n", d);
            let a = parse(&input).unwrap();
            assert!(a.label.is_none());
        }
    }

    #[test]
    fn render_output() {
        let input = "direction down\nlabel \"go\"\n";
        let a = parse(input).unwrap();
        let svg = render_svg(&a);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("go"));
    }
}
