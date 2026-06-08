use std::io::{self, Read};

#[derive(Debug)]
struct QA { question: String, answer: Vec<String> }
#[derive(Debug)]
struct Faq { items: Vec<QA> }

fn parse(input: &str) -> Result<Faq, String> {
    let mut items = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() { i += 1; continue; }
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
    Ok(Faq { items })
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
const Q_H: f64 = 32.0; const A_LINE_H: f64 = 18.0; const A_PAD: f64 = 10.0;
const DIVIDER_PAD: f64 = 12.0;
#[allow(dead_code)]
fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn wrap_text(s: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let scale = font_size / 13.0;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w = 0.0;
    for c in s.chars() {
        let cw = if c.is_ascii() { CW } else { CJK };
        let w = cw * scale;
        if current_w + w > max_width && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
            current_w = 0.0;
        }
        current.push(c);
        current_w += w;
    }
    if !current.is_empty() { lines.push(current); }
    lines
}

fn qa_h_wrapped(wrapped_lines: usize) -> f64 { Q_H + A_PAD + wrapped_lines.max(1) as f64 * A_LINE_H + A_PAD }

fn render_svg(faq: &Faq) -> String {
    let card_w = CARD_W;
    // Max text width for answer wrapping: card_w minus left offset (badge area) minus right padding
    let answer_text_x = 42.0;
    let max_answer_w = card_w - answer_text_x - PAD;

    // Pre-wrap all answer lines
    let wrapped_answers: Vec<Vec<String>> = faq.items.iter().map(|qa| {
        qa.answer.iter()
            .flat_map(|line| wrap_text(line, max_answer_w, 12.0))
            .collect()
    }).collect();

    let total_items_h: f64 = wrapped_answers.iter().map(|lines| qa_h_wrapped(lines.len())).sum::<f64>()
        + (faq.items.len().saturating_sub(1)) as f64 * (DIVIDER_PAD * 2.0);
    let total_w = PAD * 2.0 + card_w;
    let total_h = PAD * 2.0 + total_items_h;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let mut y = PAD;

    for (i, qa) in faq.items.iter().enumerate() {
        let lines = &wrapped_answers[i];
        let h = qa_h_wrapped(lines.len());

        // Q badge
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"22\" rx=\"4\" fill=\"#e3f2fd\" stroke=\"#90caf9\" stroke-width=\"1\"/>", PAD + 12.0, y + Q_H / 2.0 - 11.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"#1565c0\">Q</text>", PAD + 23.0, y + Q_H / 2.0 + 5.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\">{}</text>", PAD + answer_text_x, y + Q_H / 2.0 + 5.0, ex(&qa.question)));

        // A badge
        let ay = y + Q_H + A_PAD;
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"22\" height=\"22\" rx=\"4\" fill=\"#e8f5e9\" stroke=\"#a5d6a7\" stroke-width=\"1\"/>", PAD + 12.0, ay - 2.0));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"#2e7d32\">A</text>", PAD + 23.0, ay + 14.0));
        for (j, line) in lines.iter().enumerate() {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#555\">{}</text>", PAD + answer_text_x, ay + j as f64 * A_LINE_H + 14.0, ex(line)));
        }

        y += h;

        if i < faq.items.len() - 1 {
            y += DIVIDER_PAD;
            svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e8e8e8\" stroke-width=\"1\"/>",
                PAD + 12.0, y, PAD + card_w - 12.0, y));
            y += DIVIDER_PAD;
        }
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-faq - Render a FAQ (Q&A list) as SVG

Usage: mdd-faq < input.faq

Each Q&A pair starts with \"q\" followed by the question, then \"a\"
followed by the answer. Wrap text in quotes. Multi-line answers
use a closing quote on a subsequent line.

Example:
  q \"What is mdd?\"
  a \"A text-to-diagram tool.\"

  q \"Is it free?\"
  a \"Yes, it is free.\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }
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
