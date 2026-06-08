use std::io::{self, Read};

#[derive(Debug)]
struct Plan { name: String, price: String, features: Vec<String>, highlighted: bool }
#[derive(Debug)]
struct PriceTable { plans: Vec<Plan> }

fn parse(input: &str) -> Result<PriceTable, String> {
    let mut plans = Vec::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("plan ") || t.starts_with("plan* ") {
            let highlighted = t.starts_with("plan* ");
            let prefix = if highlighted { "plan* " } else { "plan " };
            let rest = t.strip_prefix(prefix).unwrap().trim();
            let (name, price) = if let Some((n, p)) = rest.split_once(" : ") {
                (sq(n.trim()).to_string(), sq(p.trim()).to_string())
            } else { (sq(rest).to_string(), String::new()) };
            plans.push(Plan { name, price, features: Vec::new(), highlighted });
            continue;
        }
        if t.starts_with("- ") {
            if let Some(plan) = plans.last_mut() {
                plan.features.push(t.strip_prefix("- ").unwrap().trim().to_string());
            }
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if plans.is_empty() { return Err("At least 1 plan required".to_string()); }
    Ok(PriceTable { plans })
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0; const PLAN_W: f64 = 180.0; const PLAN_GAP: f64 = 12.0;
const PAD: f64 = 24.0; const HEADER_H: f64 = 80.0; const FEATURE_H: f64 = 28.0; const PLAN_PAD: f64 = 16.0;

fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn render_svg(pt: &PriceTable) -> String {
    let n = pt.plans.len();
    let max_features = pt.plans.iter().map(|p| p.features.len()).max().unwrap_or(0);
    let plan_w = pt.plans.iter().flat_map(|p| {
        let nw = tw(&p.name) + 24.0;
        let fw = p.features.iter().map(|f| tw(f) + 24.0).fold(0.0_f64, f64::max);
        vec![nw, fw].into_iter()
    }).fold(PLAN_W, f64::max);

    let plan_h = HEADER_H + PLAN_PAD + max_features as f64 * FEATURE_H + PLAN_PAD;
    let total_w = PAD * 2.0 + n as f64 * plan_w + (n-1) as f64 * PLAN_GAP;
    let total_h = PAD * 2.0 + plan_h;

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let y = PAD;
    let mut x = PAD;
    for plan in &pt.plans {
        let (border, header_bg, header_fg) = if plan.highlighted {
            ("#1565c0", "#1565c0", "white")
        } else {
            ("#e0e0e0", "#f5f5f5", "#333")
        };

        // Card
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"white\" stroke=\"{}\" stroke-width=\"{}\"/>",
            x, y, plan_w, plan_h, border, if plan.highlighted { "2" } else { "1" }));
        // Header
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>", x, y, plan_w, HEADER_H, header_bg));
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" fill=\"{}\"/>", x, y + HEADER_H - 16.0, plan_w, header_bg));

        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            x + plan_w / 2.0, y + 30.0, header_fg, ex(&plan.name)));
        if !plan.price.is_empty() {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"20\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                x + plan_w / 2.0, y + 60.0, header_fg, ex(&plan.price)));
        }

        // Features
        let mut fy = y + HEADER_H + PLAN_PAD;
        for feat in &plan.features {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#555\">\u{2713} {}</text>",
                x + PLAN_PAD, fy + 14.0, ex(feat)));
            fy += FEATURE_H;
        }

        x += plan_w + PLAN_GAP;
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-pricetable - Render a pricing table as SVG

Usage: mdd-pricetable < input.pricetable

Define plans with \"plan Name : Price\" followed by \"- feature\" lines.
Use \"plan*\" instead of \"plan\" to highlight a recommended plan.

Example:
  plan Free : \"$0/mo\"
  - 5 users
  - 1GB storage

  plan* Pro : \"$10/mo\"
  - Unlimited users
  - 100GB storage
  - API access
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(p) => print!("{}", render_svg(&p)),
        Err(e) => { eprintln!("mdd-pricetable: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "plan Free : \"$0/月\"\n- 基本機能\nplan* Pro : \"$10/月\"\n- 全機能\n";
        let p = parse(input).unwrap();
        assert_eq!(p.plans.len(), 2);
        assert!(!p.plans[0].highlighted);
        assert!(p.plans[1].highlighted);
    }
    #[test]
    fn render_output() {
        let input = "plan A : \"$0\"\n- feature\n";
        let p = parse(input).unwrap();
        let svg = render_svg(&p);
        assert!(svg.starts_with("<svg"));
    }
}
