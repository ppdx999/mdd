use std::io::{self, Read};

#[derive(Debug)]
struct Quote { text: Vec<String>, author: Option<String>, role: Option<String> }
#[derive(Debug)]
struct Diagram { quotes: Vec<Quote> }

fn parse(input: &str) -> Result<Diagram, String> {
    let mut quotes = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() { i += 1; continue; }
        if t.starts_with("quote ") {
            let rest = t.strip_prefix("quote ").unwrap().trim();
            let (text, consumed) = parse_ml(rest, &lines, i)?;
            i += consumed;
            quotes.push(Quote { text, author: None, role: None });
            i += 1; continue;
        }
        if t.starts_with("author ") {
            if let Some(q) = quotes.last_mut() { q.author = Some(sq(t.strip_prefix("author ").unwrap().trim()).to_string()); }
            i += 1; continue;
        }
        if t.starts_with("role ") {
            if let Some(q) = quotes.last_mut() { q.role = Some(sq(t.strip_prefix("role ").unwrap().trim()).to_string()); }
            i += 1; continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if quotes.is_empty() { return Err("At least 1 quote required".to_string()); }
    Ok(Diagram { quotes })
}

fn parse_ml(start: &str, lines: &[&str], cur: usize) -> Result<(Vec<String>, usize), String> {
    let c = start.strip_prefix('"').unwrap_or(start);
    if let Some(end) = c.find('"') { return Ok((vec![c[..end].to_string()], 0)); }
    let mut dl = vec![c.to_string()]; let mut extra = 0;
    for j in (cur+1)..lines.len() {
        extra += 1; let l = lines[j].trim();
        if l.ends_with('"') { dl.push(l[..l.len()-1].to_string()); return Ok((dl, extra)); }
        dl.push(l.to_string());
    }
    Err("Unterminated quote".to_string())
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0; const CARD_W: f64 = 380.0; const PAD: f64 = 24.0;
const CARD_PAD: f64 = 20.0; const QUOTE_LINE_H: f64 = 22.0; const AUTHOR_H: f64 = 24.0; const CARD_GAP: f64 = 16.0;
const ACCENT_W: f64 = 4.0;

const COLORS: &[&str] = &["#1565c0", "#2e7d32", "#f57f17", "#7b1fa2", "#00695c", "#c62828"];

fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn quote_h(q: &Quote) -> f64 {
    let text_h = q.text.len() as f64 * QUOTE_LINE_H;
    let author_h = if q.author.is_some() { AUTHOR_H + 8.0 } else { 0.0 };
    CARD_PAD + text_h + author_h + CARD_PAD
}

fn render_svg(diagram: &Diagram) -> String {
    let card_w = diagram.quotes.iter().flat_map(|q| q.text.iter().map(|l| tw(l) + CARD_PAD * 2.0 + 20.0)).fold(CARD_W, f64::max);
    let total_h_cards: f64 = diagram.quotes.iter().map(|q| quote_h(q)).sum::<f64>() + (diagram.quotes.len()-1) as f64 * CARD_GAP;
    let total_w = PAD * 2.0 + card_w;
    let total_h = PAD * 2.0 + total_h_cards;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 14px; fill: #333; }</style>");

    let mut y = PAD;
    for (i, q) in diagram.quotes.iter().enumerate() {
        let h = quote_h(q);
        let color = COLORS[i % COLORS.len()];

        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"#fafafa\" stroke=\"#e8e8e8\" stroke-width=\"1\"/>", PAD, y, card_w, h));
        // Left accent bar
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{}\"/>", PAD, y, ACCENT_W, h, color));

        // Quote mark
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"32\" fill=\"{}\" opacity=\"0.3\">\u{201C}</text>", PAD + 16.0, y + CARD_PAD + 24.0, color));

        // Text
        let tx = PAD + 20.0;
        for (j, line) in q.text.iter().enumerate() {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" font-style=\"italic\" fill=\"#444\">{}</text>",
                tx, y + CARD_PAD + j as f64 * QUOTE_LINE_H + 16.0, ex(line)));
        }

        // Author
        if let Some(ref author) = q.author {
            let ay = y + CARD_PAD + q.text.len() as f64 * QUOTE_LINE_H + 20.0;
            let author_text = if let Some(ref role) = q.role {
                format!("— {} / {}", author, role)
            } else {
                format!("— {}", author)
            };
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>", tx, ay, color, ex(&author_text)));
        }

        y += h + CARD_GAP;
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-quote - Render quote cards as SVG

Usage: mdd-quote < input.quote

Each quote block starts with: quote \"text\"
Optionally followed by: author \"name\" and role \"title\".

Example:
  quote \"This tool is amazing!\"
  author \"Alice\"
  role \"Engineer\"

  quote \"Highly recommended.\"
  author \"Bob\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(d) => print!("{}", render_svg(&d)),
        Err(e) => { eprintln!("mdd-quote: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "quote \"Hello world\"\nauthor Alice\n";
        let d = parse(input).unwrap();
        assert_eq!(d.quotes[0].text, vec!["Hello world"]);
        assert_eq!(d.quotes[0].author.as_deref(), Some("Alice"));
    }
    #[test]
    fn render_output() {
        let input = "quote \"Test\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
    }
}
