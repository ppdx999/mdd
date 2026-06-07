use std::io::{self, Read};

#[derive(Debug)]
struct QA { question: String, answer: Vec<String> }
#[derive(Debug)]
struct Faq { title: Option<String>, items: Vec<QA> }

fn parse(input: &str) -> Result<Faq, String> {
    let mut title = None; let mut items = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() { i += 1; continue; }
        if t.starts_with("title ") { title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string()); i += 1; continue; }
        if t.starts_with("q ") {
            let q = sq(t.strip_prefix("q ").unwrap().trim()).to_string();
            i += 1;
            let mut answer = Vec::new();
            while i < lines.len() {
                let at = lines[i].trim();
                if at.starts_with("a ") {
                    let rest = at.strip_prefix("a ").unwrap().trim();
                    let (ans, consumed) = parse_ml(rest, &lines, i)?;
                    answer = ans;
                    i += consumed + 1;
                    break;
                }
                i += 1;
            }
            items.push(QA { question: q, answer });
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if items.is_empty() { return Err("At least 1 Q&A required".to_string()); }
    Ok(Faq { title, items })
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
    Err("Unterminated answer".to_string())
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0; const CARD_W: f64 = 400.0; const PAD: f64 = 24.0;
const Q_H: f64 = 32.0; const A_LINE_H: f64 = 18.0; const A_PAD: f64 = 10.0; const ITEM_GAP: f64 = 8.0;
fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn qa_h(qa: &QA) -> f64 { Q_H + A_PAD + qa.answer.len().max(1) as f64 * A_LINE_H + A_PAD }

fn render_svg(faq: &Faq) -> String {
    let card_w = faq.items.iter().map(|qa| {
        let qw = tw(&qa.question) + 40.0;
        let aw = qa.answer.iter().map(|a| tw(a) + 40.0).fold(0.0_f64, f64::max);
        qw.max(aw)
    }).fold(CARD_W, f64::max);
    let title_space = if faq.title.is_some() { 40.0 } else { 0.0 };
    let total_items_h: f64 = faq.items.iter().map(|qa| qa_h(qa)).sum::<f64>() + (faq.items.len()-1) as f64 * ITEM_GAP;
    let total_w = PAD * 2.0 + card_w;
    let total_h = PAD * 2.0 + title_space + total_items_h;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let mut y = PAD;
    if let Some(ref t) = faq.title {
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"18\" font-weight=\"bold\">{}</text>", PAD, y + 18.0, ex(t)));
        y += title_space;
    }

    for qa in &faq.items {
        let h = qa_h(qa);
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"#fafafa\" stroke=\"#e8e8e8\" stroke-width=\"1\"/>", PAD, y, card_w, h));
        // Q badge
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"22\" rx=\"4\" fill=\"#1565c0\"/>", PAD + 12.0, y + Q_H / 2.0 - 11.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"white\">Q</text>", PAD + 23.0, y + Q_H / 2.0 + 5.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\">{}</text>", PAD + 42.0, y + Q_H / 2.0 + 5.0, ex(&qa.question)));
        // A
        let ay = y + Q_H + A_PAD;
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"22\" rx=\"4\" fill=\"#2e7d32\"/>", PAD + 12.0, ay - 2.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"white\">A</text>", PAD + 23.0, ay + 14.0));
        for (j, line) in qa.answer.iter().enumerate() {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#555\">{}</text>", PAD + 42.0, ay + j as f64 * A_LINE_H + 14.0, ex(line)));
        }
        y += h + ITEM_GAP;
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(f) => print!("{}", render_svg(&f)),
        Err(e) => { eprintln!("mdd-faq: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "q \"What?\"\na \"Answer.\"\n";
        let f = parse(input).unwrap();
        assert_eq!(f.items.len(), 1);
        assert_eq!(f.items[0].question, "What?");
    }
    #[test]
    fn render_output() {
        let input = "q Q1\na \"A1\"\n";
        let f = parse(input).unwrap();
        let svg = render_svg(&f);
        assert!(svg.starts_with("<svg"));
    }
}
