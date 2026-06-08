use std::io::{self, Read};

#[derive(Debug)]
struct Title {
    title: String,
    subtitle: Option<String>,
}

fn parse(input: &str) -> Result<Title, String> {
    let mut title = None;
    let mut subtitle = None;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("title ") {
            title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string());
            continue;
        }
        if t.starts_with("subtitle ") {
            subtitle = Some(sq(t.strip_prefix("subtitle ").unwrap().trim()).to_string());
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }

    let title = title.ok_or("Missing 'title'")?;
    Ok(Title { title, subtitle })
}

fn sq(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const TITLE_SIZE: f64 = 36.0;
const SUBTITLE_SIZE: f64 = 18.0;
const GAP: f64 = 16.0;
const H_PAD: f64 = 60.0;
const V_PAD: f64 = 40.0;
const MIN_W: f64 = 500.0;
const MIN_H: f64 = 200.0;

fn tw(s: &str, font_size: f64) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum::<f64>() * (font_size / 13.0)
}

fn ex(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn render_svg(t: &Title) -> String {
    let title_w = tw(&t.title, TITLE_SIZE);
    let sub_w = t.subtitle.as_ref().map(|s| tw(s, SUBTITLE_SIZE)).unwrap_or(0.0);
    let content_w = title_w.max(sub_w);
    let content_h = TITLE_SIZE + if t.subtitle.is_some() { GAP + SUBTITLE_SIZE } else { 0.0 };

    let w = (content_w + H_PAD * 2.0).max(MIN_W);
    let h = (content_h + V_PAD * 2.0).max(MIN_H);
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        w, h, w, h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let title_y = if t.subtitle.is_some() {
        cy - GAP / 2.0
    } else {
        cy + TITLE_SIZE * 0.35
    };

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"{}\" font-weight=\"bold\" fill=\"#1a1a1a\">{}</text>",
        cx, title_y, TITLE_SIZE, ex(&t.title)
    ));

    if let Some(ref sub) = t.subtitle {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"#666\">{}</text>",
            cx, title_y + GAP + SUBTITLE_SIZE, SUBTITLE_SIZE, ex(sub)
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(t) => print!("{}", render_svg(&t)),
        Err(e) => { eprintln!("mdd-title: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let t = parse("title \"Hello\"\n").unwrap();
        assert_eq!(t.title, "Hello");
        assert!(t.subtitle.is_none());
    }

    #[test]
    fn parse_with_subtitle() {
        let t = parse("title \"Hello\"\nsubtitle \"World\"\n").unwrap();
        assert_eq!(t.title, "Hello");
        assert_eq!(t.subtitle.as_deref(), Some("World"));
    }

    #[test]
    fn render_output() {
        let t = parse("title \"Test\"\nsubtitle \"Sub\"\n").unwrap();
        let svg = render_svg(&t);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Test"));
        assert!(svg.contains("Sub"));
    }
}
