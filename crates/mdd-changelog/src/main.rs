use std::io::{self, Read};

#[derive(Debug)]
struct Change { kind: String, text: String }
#[derive(Debug)]
struct Release { version: String, date: Option<String>, changes: Vec<Change> }
#[derive(Debug)]
struct Changelog { releases: Vec<Release> }

fn parse(input: &str) -> Result<Changelog, String> {
    let mut releases = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("release ") {
            let rest = t.strip_prefix("release ").unwrap().trim();
            let (version, date) = if let Some((v, d)) = rest.split_once(" : ") {
                (sq(v.trim()).to_string(), Some(sq(d.trim()).to_string()))
            } else { (sq(rest).to_string(), None) };
            releases.push(Release { version, date, changes: Vec::new() });
            continue;
        }
        if t.starts_with("- ") {
            if let Some(rel) = releases.last_mut() {
                let rest = t.strip_prefix("- ").unwrap().trim();
                let (kind, text) = if let Some((k, txt)) = rest.split_once(' ') {
                    let k_lower = k.to_lowercase();
                    if ["add", "fix", "change", "remove", "improve", "security"].contains(&k_lower.as_str()) {
                        (k_lower, txt.trim().to_string())
                    } else { ("change".to_string(), rest.to_string()) }
                } else { ("change".to_string(), rest.to_string()) };
                rel.changes.push(Change { kind, text });
            }
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if releases.is_empty() { return Err("At least 1 release required".to_string()); }
    Ok(Changelog { releases })
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0; const CARD_W: f64 = 400.0; const PAD: f64 = 24.0;
const REL_GAP: f64 = 16.0; const ITEM_H: f64 = 24.0; const HEADER_H: f64 = 36.0; const CARD_PAD: f64 = 14.0;

fn kind_color(k: &str) -> (&str, &str) {
    match k { "add" => ("#e8f5e9","#2e7d32"), "fix" => ("#fce4ec","#c62828"), "remove" => ("#fff3e0","#e65100"),
        "security" => ("#fff8e1","#f57f17"), "improve" => ("#e3f2fd","#1565c0"), _ => ("#f5f5f5","#616161") }
}
fn kind_label(k: &str) -> &str {
    match k { "add" => "ADD", "fix" => "FIX", "remove" => "DEL", "security" => "SEC", "improve" => "IMP", _ => "CHG" }
}

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

const CHANGE_LINE_H: f64 = 18.0;

fn render_svg(cl: &Changelog) -> String {
    let card_w = CARD_W;
    // The text area for change text starts after the kind badge
    let badge_max_w: f64 = 50.0; // approximate max badge width
    let text_x_offset = CARD_PAD + badge_max_w + 8.0;
    let max_text_w = card_w - text_x_offset - CARD_PAD;

    // Pre-wrap all change texts and compute heights
    let wrapped: Vec<Vec<Vec<String>>> = cl.releases.iter().map(|r| {
        r.changes.iter().map(|c| wrap_text(&c.text, max_text_w, 12.0)).collect()
    }).collect();

    let total_cards_h: f64 = cl.releases.iter().enumerate().map(|(ri, _r)| {
        let changes_h: f64 = wrapped[ri].iter().map(|lines| {
            let num = lines.len().max(1);
            num as f64 * CHANGE_LINE_H + (ITEM_H - CHANGE_LINE_H)
        }).sum();
        HEADER_H + CARD_PAD + changes_h + CARD_PAD
    }).sum::<f64>() + (cl.releases.len() - 1) as f64 * REL_GAP;

    let total_w = PAD * 2.0 + card_w;
    let total_h = PAD * 2.0 + total_cards_h;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let mut y = PAD;

    for (ri, rel) in cl.releases.iter().enumerate() {
        let changes_h: f64 = wrapped[ri].iter().map(|lines| {
            let num = lines.len().max(1);
            num as f64 * CHANGE_LINE_H + (ITEM_H - CHANGE_LINE_H)
        }).sum();
        let card_h = HEADER_H + CARD_PAD + changes_h + CARD_PAD;

        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"#fafafa\" stroke=\"#e8e8e8\" stroke-width=\"1\"/>", PAD, y, card_w, card_h));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"15\" font-weight=\"bold\">{}</text>", PAD + CARD_PAD, y + HEADER_H / 2.0 + 6.0, ex(&rel.version)));
        if let Some(ref date) = rel.date {
            let vw = tw(&rel.version) * (15.0 / 13.0);
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#999\">{}</text>", PAD + CARD_PAD + vw + 12.0, y + HEADER_H / 2.0 + 5.0, ex(date)));
        }

        let mut iy = y + HEADER_H + CARD_PAD;
        for (ci, change) in rel.changes.iter().enumerate() {
            let (bg, fg) = kind_color(&change.kind);
            let label = kind_label(&change.kind);
            let lw = tw(label) * 0.75 + 12.0;
            svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"18\" rx=\"3\" fill=\"{}\"/>", PAD + CARD_PAD, iy, lw, bg));
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"10\" font-weight=\"bold\" fill=\"{}\">{}</text>", PAD + CARD_PAD + 6.0, iy + 13.0, fg, label));

            let lines = &wrapped[ri][ci];
            let text_x = PAD + CARD_PAD + lw + 8.0;
            for (k, line) in lines.iter().enumerate() {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>", text_x, iy + 13.0 + k as f64 * CHANGE_LINE_H, ex(line)));
            }

            let num = lines.len().max(1);
            iy += num as f64 * CHANGE_LINE_H + (ITEM_H - CHANGE_LINE_H);
        }
        y += card_h + REL_GAP;
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-changelog - Render a changelog as SVG

Usage: mdd-changelog < input.changelog

Each release starts with \"release <version>\" optionally followed
by \" : <date>\". Changes are listed as \"- <kind> <text>\" where
kind is one of: add, fix, change, remove, improve, security.

Example:
  release v1.0 : \"2025-01-01\"
  - add New feature
  - fix Bug fix

  release v0.9
  - change Refactored API
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(c) => print!("{}", render_svg(&c)),
        Err(e) => { eprintln!("mdd-changelog: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "release v1.0\n- add New feature\n- fix Bug fix\n";
        let c = parse(input).unwrap();
        assert_eq!(c.releases[0].version, "v1.0");
        assert_eq!(c.releases[0].changes.len(), 2);
    }
    #[test]
    fn render_output() {
        let input = "release v1.0\n- add Feature\n";
        let c = parse(input).unwrap();
        let svg = render_svg(&c);
        assert!(svg.starts_with("<svg"));
    }
}
