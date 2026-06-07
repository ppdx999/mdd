use std::io::{self, Read};

#[derive(Debug)]
enum Element { Header(String), Text(String), Button(String), Input(String), Image(String), Divider, List(Vec<String>) }
#[derive(Debug)]
struct Wireframe { title: Option<String>, elements: Vec<Element> }

fn parse(input: &str) -> Result<Wireframe, String> {
    let mut title = None; let mut elements = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("title ") { title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string()); continue; }
        if t.starts_with("header ") { elements.push(Element::Header(sq(t.strip_prefix("header ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("text ") { elements.push(Element::Text(sq(t.strip_prefix("text ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("button ") { elements.push(Element::Button(sq(t.strip_prefix("button ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("input ") { elements.push(Element::Input(sq(t.strip_prefix("input ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("image ") { elements.push(Element::Image(sq(t.strip_prefix("image ").unwrap().trim()).to_string())); continue; }
        if t == "---" { elements.push(Element::Divider); continue; }
        if t.starts_with("- ") {
            let item = t.strip_prefix("- ").unwrap().trim().to_string();
            if let Some(Element::List(items)) = elements.last_mut() {
                items.push(item); continue;
            }
            elements.push(Element::List(vec![item])); continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if elements.is_empty() { return Err("At least 1 element required".to_string()); }
    Ok(Wireframe { title, elements })
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const W: f64 = 320.0; const PAD: f64 = 20.0; const CW: f64 = 8.0; const CJK: f64 = 14.0;
const FRAME_PAD: f64 = 16.0; const ELEM_GAP: f64 = 12.0;
fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn elem_h(e: &Element) -> f64 {
    match e {
        Element::Header(_) => 28.0, Element::Text(_) => 20.0, Element::Button(_) => 36.0,
        Element::Input(_) => 36.0, Element::Image(_) => 80.0, Element::Divider => 8.0,
        Element::List(items) => items.len() as f64 * 22.0,
    }
}

fn render_svg(wf: &Wireframe) -> String {
    let frame_w = W;
    let content_h: f64 = wf.elements.iter().map(|e| elem_h(e) + ELEM_GAP).sum::<f64>();
    let title_bar = 36.0;
    let frame_h = title_bar + FRAME_PAD + content_h + FRAME_PAD;
    let total_w = PAD * 2.0 + frame_w;
    let total_h = PAD * 2.0 + frame_h;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f0f0f0\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let fx = PAD; let fy = PAD;
    // Browser frame
    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>", fx, fy, frame_w, frame_h));
    // Title bar
    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"#f5f5f5\"/>", fx, fy, frame_w, title_bar));
    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" fill=\"#f5f5f5\"/>", fx, fy + title_bar - 16.0, frame_w));
    // Dots
    for (i, color) in ["#ff5f57","#febc2e","#28c840"].iter().enumerate() {
        svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>", fx + 18.0 + i as f64 * 16.0, fy + title_bar / 2.0, color));
    }
    if let Some(ref t) = wf.title {
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#999\">{}</text>", fx + frame_w / 2.0, fy + title_bar / 2.0 + 4.0, ex(t)));
    }

    let mut cy = fy + title_bar + FRAME_PAD;
    let inner_x = fx + FRAME_PAD;
    let inner_w = frame_w - FRAME_PAD * 2.0;

    for elem in &wf.elements {
        match elem {
            Element::Header(text) => {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"16\" font-weight=\"bold\">{}</text>", inner_x, cy + 18.0, ex(text)));
            }
            Element::Text(text) => {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#666\">{}</text>", inner_x, cy + 14.0, ex(text)));
            }
            Element::Button(text) => {
                let bw = (tw(text) + 24.0).max(80.0);
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"32\" rx=\"4\" fill=\"#1565c0\"/>", inner_x, cy, bw));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"white\" font-weight=\"bold\">{}</text>", inner_x + bw / 2.0, cy + 20.0, ex(text)));
            }
            Element::Input(placeholder) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"32\" rx=\"4\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>", inner_x, cy, inner_w));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#aaa\">{}</text>", inner_x + 10.0, cy + 20.0, ex(placeholder)));
            }
            Element::Image(alt) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"72\" rx=\"4\" fill=\"#e8e8e8\" stroke=\"#ccc\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>", inner_x, cy, inner_w));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#999\">{}</text>", inner_x + inner_w / 2.0, cy + 40.0, ex(alt)));
            }
            Element::Divider => {
                svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>", inner_x, cy + 4.0, inner_x + inner_w, cy + 4.0));
            }
            Element::List(items) => {
                for (j, item) in items.iter().enumerate() {
                    let iy = cy + j as f64 * 22.0;
                    svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#999\"/>", inner_x + 6.0, iy + 10.0));
                    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", inner_x + 16.0, iy + 14.0, ex(item)));
                }
            }
        }
        cy += elem_h(elem) + ELEM_GAP;
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(w) => print!("{}", render_svg(&w)),
        Err(e) => { eprintln!("mdd-wireframe: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "header Welcome\ntext Description\nbutton Submit\n";
        let w = parse(input).unwrap();
        assert_eq!(w.elements.len(), 3);
    }
    #[test]
    fn render_output() {
        let input = "header Hello\ninput \"Email\"\nbutton Login\n";
        let w = parse(input).unwrap();
        let svg = render_svg(&w);
        assert!(svg.starts_with("<svg"));
    }
}
