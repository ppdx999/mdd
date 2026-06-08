use std::io::{self, Read};

#[derive(Debug)]
enum Element {
    Header(String), Subheader(String), Text(String), Link(String),
    Button(String), Input(String), Textarea(String), Select(String),
    Checkbox(String, bool), Radio(String, bool), Toggle(String, bool),
    Image(String), Avatar(String), Progress(u8), Nav(Vec<String>),
    Divider, List(Vec<String>),
}
#[derive(Debug)]
struct Wireframe { title: Option<String>, elements: Vec<Element> }

fn parse(input: &str) -> Result<Wireframe, String> {
    let mut title = None; let mut elements = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("title ") { title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string()); continue; }
        if t.starts_with("header ") { elements.push(Element::Header(sq(t.strip_prefix("header ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("subheader ") { elements.push(Element::Subheader(sq(t.strip_prefix("subheader ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("text ") { elements.push(Element::Text(sq(t.strip_prefix("text ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("link ") { elements.push(Element::Link(sq(t.strip_prefix("link ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("button ") { elements.push(Element::Button(sq(t.strip_prefix("button ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("input ") { elements.push(Element::Input(sq(t.strip_prefix("input ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("textarea ") { elements.push(Element::Textarea(sq(t.strip_prefix("textarea ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("select ") { elements.push(Element::Select(sq(t.strip_prefix("select ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("checkbox ") {
            let rest = t.strip_prefix("checkbox ").unwrap().trim();
            if let Some(label) = rest.strip_prefix("checked ") {
                elements.push(Element::Checkbox(sq(label.trim()).to_string(), true));
            } else {
                elements.push(Element::Checkbox(sq(rest).to_string(), false));
            }
            continue;
        }
        if t.starts_with("radio ") {
            let rest = t.strip_prefix("radio ").unwrap().trim();
            if let Some(label) = rest.strip_prefix("selected ") {
                elements.push(Element::Radio(sq(label.trim()).to_string(), true));
            } else {
                elements.push(Element::Radio(sq(rest).to_string(), false));
            }
            continue;
        }
        if t.starts_with("toggle ") {
            let rest = t.strip_prefix("toggle ").unwrap().trim();
            if let Some(label) = rest.strip_prefix("on ") {
                elements.push(Element::Toggle(sq(label.trim()).to_string(), true));
            } else {
                elements.push(Element::Toggle(sq(rest).to_string(), false));
            }
            continue;
        }
        if t.starts_with("image ") { elements.push(Element::Image(sq(t.strip_prefix("image ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("avatar ") { elements.push(Element::Avatar(sq(t.strip_prefix("avatar ").unwrap().trim()).to_string())); continue; }
        if t.starts_with("progress ") {
            let v = t.strip_prefix("progress ").unwrap().trim();
            let pct: u8 = v.parse().map_err(|_| format!("Invalid progress value: {}", v))?;
            elements.push(Element::Progress(pct.min(100)));
            continue;
        }
        if t.starts_with("nav ") {
            let items: Vec<String> = t.strip_prefix("nav ").unwrap().split('|').map(|s| s.trim().to_string()).collect();
            elements.push(Element::Nav(items));
            continue;
        }
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
        Element::Header(_) => 28.0, Element::Subheader(_) => 24.0,
        Element::Text(_) => 20.0, Element::Link(_) => 20.0,
        Element::Button(_) => 36.0,
        Element::Input(_) => 36.0, Element::Textarea(_) => 72.0, Element::Select(_) => 36.0,
        Element::Checkbox(_, _) => 24.0, Element::Radio(_, _) => 24.0, Element::Toggle(_, _) => 28.0,
        Element::Image(_) => 80.0, Element::Avatar(_) => 52.0, Element::Progress(_) => 24.0,
        Element::Nav(_) => 36.0, Element::Divider => 8.0,
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
            Element::Subheader(text) => {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"#555\">{}</text>", inner_x, cy + 16.0, ex(text)));
            }
            Element::Text(text) => {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#666\">{}</text>", inner_x, cy + 14.0, ex(text)));
            }
            Element::Link(text) => {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#1565c0\" text-decoration=\"underline\">{}</text>", inner_x, cy + 14.0, ex(text)));
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
            Element::Textarea(placeholder) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"64\" rx=\"4\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>", inner_x, cy, inner_w));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#aaa\">{}</text>", inner_x + 10.0, cy + 20.0, ex(placeholder)));
            }
            Element::Select(placeholder) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"32\" rx=\"4\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>", inner_x, cy, inner_w));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#aaa\">{}</text>", inner_x + 10.0, cy + 20.0, ex(placeholder)));
                // Dropdown arrow
                let ax = inner_x + inner_w - 20.0;
                let ay = cy + 13.0;
                svg.push_str(&format!("<polygon points=\"{},{} {},{} {},{}\" fill=\"#999\"/>", ax, ay, ax + 8.0, ay, ax + 4.0, ay + 6.0));
            }
            Element::Checkbox(label, checked) => {
                let bx = inner_x;
                let by = cy + 3.0;
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"16\" rx=\"3\" fill=\"{}\" stroke=\"#ccc\" stroke-width=\"1\"/>",
                    bx, by, if *checked { "#1565c0" } else { "white" }));
                if *checked {
                    svg.push_str(&format!("<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"white\" stroke-width=\"2\"/>",
                        bx + 3.0, by + 8.0, bx + 7.0, by + 12.0, bx + 13.0, by + 4.0));
                }
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", bx + 22.0, cy + 16.0, ex(label)));
            }
            Element::Radio(label, selected) => {
                let cx_r = inner_x + 8.0;
                let cy_r = cy + 11.0;
                svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>", cx_r, cy_r));
                if *selected {
                    svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#1565c0\"/>", cx_r, cy_r));
                }
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", inner_x + 22.0, cy + 16.0, ex(label)));
            }
            Element::Toggle(label, on) => {
                let tx = inner_x;
                let ty = cy + 4.0;
                let track_color = if *on { "#1565c0" } else { "#ccc" };
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"36\" height=\"20\" rx=\"10\" fill=\"{}\"/>", tx, ty, track_color));
                let knob_x = if *on { tx + 18.0 } else { tx + 2.0 };
                svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"white\"/>", knob_x + 8.0, ty + 10.0));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", tx + 44.0, cy + 18.0, ex(label)));
            }
            Element::Image(alt) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"72\" rx=\"4\" fill=\"#e8e8e8\" stroke=\"#ccc\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>", inner_x, cy, inner_w));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" fill=\"#999\">{}</text>", inner_x + inner_w / 2.0, cy + 40.0, ex(alt)));
            }
            Element::Avatar(name) => {
                let acx = inner_x + 20.0;
                let acy = cy + 20.0;
                svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"20\" fill=\"#e0e0e0\"/>", acx, acy));
                let initial: String = name.chars().next().unwrap_or('?').to_uppercase().collect();
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\" fill=\"#666\">{}</text>", acx, acy + 6.0, ex(&initial)));
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", inner_x + 48.0, cy + 24.0, ex(name)));
            }
            Element::Progress(pct) => {
                let bar_h = 8.0;
                let bar_y = cy + 8.0;
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#e0e0e0\"/>", inner_x, bar_y, inner_w, bar_h));
                let fill_w = inner_w * (*pct as f64 / 100.0);
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#1565c0\"/>", inner_x, bar_y, fill_w, bar_h));
            }
            Element::Nav(items) => {
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"32\" rx=\"4\" fill=\"#f5f5f5\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>", inner_x, cy, inner_w));
                let n = items.len() as f64;
                let item_w = inner_w / n;
                for (i, item) in items.iter().enumerate() {
                    let ix = inner_x + i as f64 * item_w + item_w / 2.0;
                    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#555\">{}</text>", ix, cy + 20.0, ex(item)));
                    if i > 0 {
                        let sep_x = inner_x + i as f64 * item_w;
                        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>", sep_x, cy + 4.0, sep_x, cy + 28.0));
                    }
                }
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
    #[test]
    fn parse_subheader() {
        let w = parse("subheader Section").unwrap();
        assert!(matches!(&w.elements[0], Element::Subheader(s) if s == "Section"));
    }
    #[test]
    fn parse_link() {
        let w = parse("link \"Click here\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Link(s) if s == "Click here"));
    }
    #[test]
    fn parse_textarea() {
        let w = parse("textarea \"Enter message\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Textarea(s) if s == "Enter message"));
    }
    #[test]
    fn parse_select() {
        let w = parse("select \"Choose...\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Select(s) if s == "Choose..."));
    }
    #[test]
    fn parse_checkbox() {
        let w = parse("checkbox \"Agree\"\ncheckbox checked \"Done\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Checkbox(s, false) if s == "Agree"));
        assert!(matches!(&w.elements[1], Element::Checkbox(s, true) if s == "Done"));
    }
    #[test]
    fn parse_radio() {
        let w = parse("radio \"Option A\"\nradio selected \"Option B\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Radio(s, false) if s == "Option A"));
        assert!(matches!(&w.elements[1], Element::Radio(s, true) if s == "Option B"));
    }
    #[test]
    fn parse_toggle() {
        let w = parse("toggle \"Dark mode\"\ntoggle on \"Notifications\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Toggle(s, false) if s == "Dark mode"));
        assert!(matches!(&w.elements[1], Element::Toggle(s, true) if s == "Notifications"));
    }
    #[test]
    fn parse_avatar() {
        let w = parse("avatar \"Taro\"").unwrap();
        assert!(matches!(&w.elements[0], Element::Avatar(s) if s == "Taro"));
    }
    #[test]
    fn parse_progress() {
        let w = parse("progress 75").unwrap();
        assert!(matches!(&w.elements[0], Element::Progress(75)));
    }
    #[test]
    fn parse_nav() {
        let w = parse("nav Home | Settings | Profile").unwrap();
        if let Element::Nav(items) = &w.elements[0] {
            assert_eq!(items, &["Home", "Settings", "Profile"]);
        } else { panic!("Expected Nav"); }
    }
    #[test]
    fn render_all_elements() {
        let input = "title \"Test\"\nheader H\nsubheader SH\ntext T\nlink L\nbutton B\ninput \"I\"\ntextarea \"TA\"\nselect \"S\"\ncheckbox \"C\"\ncheckbox checked \"CC\"\nradio \"R\"\nradio selected \"RS\"\ntoggle \"TG\"\ntoggle on \"TGO\"\nimage \"IMG\"\navatar \"A\"\nprogress 50\nnav X | Y\n---\n- item1\n";
        let w = parse(input).unwrap();
        let svg = render_svg(&w);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }
}
