use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Pin {
    label: String,
    x: f64,
    y: f64,
}

#[derive(Debug)]
struct Route {
    from: usize,
    to: usize,
}

#[derive(Debug)]
struct Map {
    width: f64,
    height: f64,
    pins: Vec<Pin>,
    routes: Vec<Route>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Map, String> {
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut pins: Vec<Pin> = Vec::new();
    let mut routes: Vec<Route> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // width N
        if trimmed.starts_with("width ") {
            let rest = trimmed.strip_prefix("width ").unwrap().trim();
            width = rest
                .parse::<f64>()
                .map_err(|_| format!("Invalid width: {}", rest))?;
            continue;
        }

        // height N
        if trimmed.starts_with("height ") {
            let rest = trimmed.strip_prefix("height ").unwrap().trim();
            height = rest
                .parse::<f64>()
                .map_err(|_| format!("Invalid height: {}", rest))?;
            continue;
        }

        // pin "Label" at x,y
        if trimmed.starts_with("pin ") {
            let rest = trimmed.strip_prefix("pin ").unwrap().trim();
            let pin = parse_pin(rest)?;
            pins.push(pin);
            continue;
        }

        // route 0 -- 1
        if trimmed.starts_with("route ") {
            let rest = trimmed.strip_prefix("route ").unwrap().trim();
            let route = parse_route(rest)?;
            routes.push(route);
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if pins.is_empty() {
        return Err("At least 1 pin is required".to_string());
    }

    // Validate route indices
    for route in &routes {
        if route.from >= pins.len() || route.to >= pins.len() {
            return Err(format!(
                "Route index out of range: {} -- {} (only {} pins)",
                route.from,
                route.to,
                pins.len()
            ));
        }
    }

    Ok(Map {
        width,
        height,
        pins,
        routes,
    })
}

fn parse_pin(rest: &str) -> Result<Pin, String> {
    // "Label" at x,y
    let quote_start = rest
        .find('"')
        .ok_or("Expected '\"' in pin definition")?;
    let quote_end = rest[quote_start + 1..]
        .find('"')
        .ok_or("Unterminated quote in pin label")?
        + quote_start
        + 1;
    let label = rest[quote_start + 1..quote_end].to_string();

    let after_label = rest[quote_end + 1..].trim();
    let coords_str = after_label
        .strip_prefix("at ")
        .or_else(|| after_label.strip_prefix("at\t"))
        .ok_or("Expected 'at' after pin label")?
        .trim();

    let parts: Vec<&str> = coords_str.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Expected x,y coordinates, got: {}", coords_str));
    }

    let x = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Invalid x coordinate: {}", parts[0]))?;
    let y = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("Invalid y coordinate: {}", parts[1]))?;

    Ok(Pin { label, x, y })
}

fn parse_route(rest: &str) -> Result<Route, String> {
    // 0 -- 1
    let parts: Vec<&str> = rest.split("--").collect();
    if parts.len() != 2 {
        return Err(format!("Expected 'from -- to' in route, got: {}", rest));
    }

    let from = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("Invalid route index: {}", parts[0].trim()))?;
    let to = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("Invalid route index: {}", parts[1].trim()))?;

    Ok(Route { from, to })
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const LABEL_FONT_SIZE: f64 = 12.0;
const COLOR_DARK: &str = "#333";

const DEFAULT_WIDTH: f64 = 500.0;
const DEFAULT_HEIGHT: f64 = 350.0;
const PIN_RADIUS: f64 = 8.0;
const PIN_TOTAL_HEIGHT: f64 = 22.0;
const PADDING: f64 = 40.0;
const ROUTE_COLOR: &str = "#666";

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

fn render_svg(map: &Map) -> String {
    let canvas_x = PADDING;
    let canvas_y = PADDING;
    let canvas_w = map.width;
    let canvas_h = map.height;

    let total_w = canvas_w + PADDING * 2.0;
    let total_h = canvas_y + canvas_h + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    // Canvas area
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"#f5f5f5\" stroke=\"#ddd\" stroke-width=\"1\"/>",
        canvas_x, canvas_y, canvas_w, canvas_h
    ));

    // Routes (draw before pins so pins appear on top)
    for route in &map.routes {
        let from = &map.pins[route.from];
        let to = &map.pins[route.to];
        let x1 = canvas_x + from.x;
        let y1 = canvas_y + from.y;
        let x2 = canvas_x + to.x;
        let y2 = canvas_y + to.y;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6,3\"/>",
            x1, y1, x2, y2, ROUTE_COLOR
        ));
    }

    // Pins
    for (i, pin) in map.pins.iter().enumerate() {
        let (bg_color, fg_color) = COLORS[i % COLORS.len()];
        let px = canvas_x + pin.x;
        let py = canvas_y + pin.y;

        // Triangle pointing down (below circle)
        let tri_top = py + PIN_RADIUS * 0.5;
        let tri_bottom = py + PIN_TOTAL_HEIGHT - PIN_RADIUS;
        let tri_half_w = PIN_RADIUS * 0.5;
        svg.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            px - tri_half_w,
            tri_top,
            px + tri_half_w,
            tri_top,
            px,
            tri_bottom,
            fg_color
        ));

        // Circle on top
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            px, py, PIN_RADIUS, bg_color, fg_color
        ));

        // Label above the pin
        let label_y = py - PIN_RADIUS - 6.0;
        let _label_w = text_width(&pin.label);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\">{}</text>",
            px,
            label_y,
            LABEL_FONT_SIZE,
            escape_xml(&pin.label)
        ));
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

    let map = match parse(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mdd-map: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&map));
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
width 600
height 400
pin "A" at 100,200
pin "B" at 300,150
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.width, 600.0);
        assert_eq!(m.height, 400.0);
        assert_eq!(m.pins.len(), 2);
        assert_eq!(m.pins[0].label, "A");
        assert_eq!(m.pins[0].x, 100.0);
        assert_eq!(m.pins[0].y, 200.0);
        assert_eq!(m.pins[1].label, "B");
        assert_eq!(m.pins[1].x, 300.0);
        assert_eq!(m.pins[1].y, 150.0);
        assert!(m.routes.is_empty());
    }

    #[test]
    fn parse_with_routes() {
        let input = r#"
pin "X" at 10,20
pin "Y" at 30,40
pin "Z" at 50,60
route 0 -- 1
route 1 -- 2
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.pins.len(), 3);
        assert_eq!(m.routes.len(), 2);
        assert_eq!(m.routes[0].from, 0);
        assert_eq!(m.routes[0].to, 1);
        assert_eq!(m.routes[1].from, 1);
        assert_eq!(m.routes[1].to, 2);
    }

    #[test]
    fn render_produces_svg() {
        let input = r#"
pin "A" at 100,100
pin "B" at 200,200
route 0 -- 1
"#;
        let m = parse(input).unwrap();
        let svg = render_svg(&m);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("white"));
        assert!(svg.contains("#f5f5f5"));
        assert!(svg.contains("A"));
        assert!(svg.contains("B"));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn parse_error_no_pins() {
        let input = "";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_error_route_out_of_range() {
        let input = r#"
pin "A" at 10,20
route 0 -- 5
"#;
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_defaults() {
        let input = "pin \"Solo\" at 50,50\n";
        let m = parse(input).unwrap();
        assert_eq!(m.width, DEFAULT_WIDTH);
        assert_eq!(m.height, DEFAULT_HEIGHT);
    }
}
