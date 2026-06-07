use std::io::{self, Read};

#[derive(Debug)]
struct Stage { name: String, action: String, emotion: i32 }
#[derive(Debug)]
struct Journey { title: Option<String>, persona: Option<String>, stages: Vec<Stage> }

fn parse(input: &str) -> Result<Journey, String> {
    let mut title = None; let mut persona = None; let mut stages = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("title ") { title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string()); continue; }
        if t.starts_with("persona ") { persona = Some(sq(t.strip_prefix("persona ").unwrap().trim()).to_string()); continue; }
        if t.starts_with("stage ") {
            let rest = t.strip_prefix("stage ").unwrap().trim();
            let parts: Vec<&str> = rest.splitn(3, " : ").collect();
            if parts.len() < 3 { return Err(format!("stage needs name : action : emotion(1-5): {}", t)); }
            let emotion = parts[2].trim().parse::<i32>().map_err(|_| format!("Invalid emotion: {}", parts[2]))?;
            stages.push(Stage { name: sq(parts[0].trim()).to_string(), action: sq(parts[1].trim()).to_string(), emotion });
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if stages.len() < 2 { return Err("At least 2 stages required".to_string()); }
    Ok(Journey { title, persona, stages })
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0;
const STAGE_W: f64 = 140.0; const STAGE_GAP: f64 = 8.0; const PAD: f64 = 40.0;
const HEADER_H: f64 = 60.0; const ACTION_H: f64 = 50.0; const GRAPH_H: f64 = 100.0; const BOTTOM_PAD: f64 = 20.0;

const EMOTIONS: &[(&str, &str)] = &[("😡","#c62828"),("😟","#f57f17"),("😐","#757575"),("🙂","#2e7d32"),("😄","#1565c0")];

fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn render_svg(j: &Journey) -> String {
    let n = j.stages.len();
    let sw = j.stages.iter().map(|s| (tw(&s.name) + 24.0).max((tw(&s.action) * 0.85 + 24.0).max(STAGE_W))).fold(0.0_f64, f64::max);
    let total_w = PAD * 2.0 + n as f64 * sw + (n-1) as f64 * STAGE_GAP;
    let total_h = PAD + HEADER_H + ACTION_H + GRAPH_H + BOTTOM_PAD + PAD;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 12px; fill: #333; }</style>");

    let mut cy = PAD;
    if let Some(ref t) = j.title {
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"16\" font-weight=\"bold\">{}</text>", PAD, cy + 16.0, ex(t)));
        if let Some(ref p) = j.persona {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#666\">{}</text>", PAD, cy + 34.0, ex(p)));
        }
    }
    cy += HEADER_H;

    // Stage headers
    let mut x = PAD;
    for (i, stage) in j.stages.iter().enumerate() {
        let bg = if i % 2 == 0 { "#f7f8fc" } else { "#fff" };
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>", x, cy - 20.0, sw, ACTION_H + GRAPH_H + 40.0, bg));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" font-size=\"13\">{}</text>", x + sw/2.0, cy, ex(&stage.name)));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#666\">{}</text>", x + sw/2.0, cy + 18.0, ex(&stage.action)));
        x += sw + STAGE_GAP;
    }
    cy += ACTION_H;

    // Emotion graph
    let graph_top = cy;
    let graph_bot = cy + GRAPH_H;
    svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>", PAD, (graph_top+graph_bot)/2.0, PAD + n as f64 * (sw + STAGE_GAP), (graph_top+graph_bot)/2.0));

    let points: Vec<(f64, f64)> = j.stages.iter().enumerate().map(|(i, s)| {
        let px = PAD + i as f64 * (sw + STAGE_GAP) + sw / 2.0;
        let ey = graph_bot - (s.emotion.max(1).min(5) - 1) as f64 * (GRAPH_H / 4.0);
        (px, ey)
    }).collect();

    // Line
    for w in points.windows(2) {
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1565c0\" stroke-width=\"2\"/>", w[0].0, w[0].1, w[1].0, w[1].1));
    }
    // Dots + emoji
    for (i, (px, py)) in points.iter().enumerate() {
        let emo = j.stages[i].emotion.max(1).min(5) as usize - 1;
        let (emoji, color) = EMOTIONS[emo];
        svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"{}\" stroke=\"white\" stroke-width=\"2\"/>", px, py, color));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"16\">{}</text>", px, py - 12.0, emoji));
    }

    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(j) => print!("{}", render_svg(&j)),
        Err(e) => { eprintln!("mdd-journey: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "stage 認知 : 広告を見る : 3\nstage 検討 : 比較する : 4\n";
        let j = parse(input).unwrap();
        assert_eq!(j.stages.len(), 2);
        assert_eq!(j.stages[0].emotion, 3);
    }
    #[test]
    fn render_output() {
        let input = "stage A : do : 3\nstage B : do : 5\n";
        let j = parse(input).unwrap();
        let svg = render_svg(&j);
        assert!(svg.starts_with("<svg"));
    }
}
