use std::io::{self, Read};

#[derive(Debug)]
struct Person { name: String, role: Option<String>, parent: Option<usize> }
#[derive(Debug)]
struct Org { people: Vec<Person> }

fn parse(input: &str) -> Result<Org, String> {
    let mut people: Vec<Person> = Vec::new();
    let mut name_to_id: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("member ") {
            let rest = t.strip_prefix("member ").unwrap().trim();
            let (name, role) = if let Some((n, r)) = rest.split_once(" : ") {
                (sq(n.trim()).to_string(), Some(sq(r.trim()).to_string()))
            } else { (sq(rest).to_string(), None) };
            let id = people.len();
            name_to_id.insert(name.clone(), id);
            people.push(Person { name, role, parent: None });
            continue;
        }
        if t.contains(" -> ") {
            let parts: Vec<&str> = t.splitn(2, " -> ").collect();
            let parent_name = parts[0].trim();
            let child_name = parts[1].trim();
            let parent_id = *name_to_id.get(parent_name).ok_or_else(|| format!("Unknown: {}", parent_name))?;
            let child_id = *name_to_id.get(child_name).ok_or_else(|| format!("Unknown: {}", child_name))?;
            people[child_id].parent = Some(parent_id);
            continue;
        }
        return Err(format!("Unknown syntax: {}", t));
    }
    if people.is_empty() { return Err("At least 1 member required".to_string()); }
    Ok(Org { people })
}

fn sq(s: &str) -> &str { if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s } }

const CW: f64 = 8.0; const CJK: f64 = 14.0; const NODE_H_PAD: f64 = 16.0;
const NODE_H: f64 = 44.0; const NODE_ROLE_H: f64 = 58.0; const NODE_GAP_X: f64 = 24.0;
const LEVEL_GAP_Y: f64 = 60.0; const PAD: f64 = 40.0; const MIN_NODE_W: f64 = 100.0;

const COLORS: &[(&str, &str)] = &[
    ("#e3f2fd","#1565c0"), ("#e8f5e9","#2e7d32"), ("#fff8e1","#f57f17"),
    ("#f3e5f5","#7b1fa2"), ("#e0f2f1","#00695c"), ("#fce4ec","#c62828"),
];

fn tw(s: &str) -> f64 { s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum() }
fn ex(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;") }

fn children_of(people: &[Person], id: usize) -> Vec<usize> {
    people.iter().enumerate().filter(|(_, p)| p.parent == Some(id)).map(|(i, _)| i).collect()
}

fn subtree_width(people: &[Person], id: usize) -> f64 {
    let children = children_of(people, id);
    let nw = node_w(&people[id]);
    if children.is_empty() { return nw; }
    let children_w: f64 = children.iter().map(|&c| subtree_width(people, c)).sum::<f64>()
        + (children.len() - 1) as f64 * NODE_GAP_X;
    children_w.max(nw)
}

fn node_w(p: &Person) -> f64 {
    let nw = tw(&p.name) + NODE_H_PAD * 2.0;
    let rw = p.role.as_ref().map(|r| tw(r) * 0.85 + NODE_H_PAD * 2.0).unwrap_or(0.0);
    nw.max(rw).max(MIN_NODE_W)
}

fn depth(people: &[Person], id: usize) -> usize {
    match people[id].parent { Some(p) => 1 + depth(people, p), None => 0 }
}

fn render_svg(org: &Org) -> String {
    let roots: Vec<usize> = org.people.iter().enumerate().filter(|(_, p)| p.parent.is_none()).map(|(i, _)| i).collect();
    let total_tree_w: f64 = roots.iter().map(|&r| subtree_width(&org.people, r)).sum::<f64>()
        + (roots.len().saturating_sub(1)) as f64 * NODE_GAP_X;
    let max_depth = org.people.iter().enumerate().map(|(i, _)| depth(&org.people, i)).max().unwrap_or(0);

    let total_w = PAD * 2.0 + total_tree_w;
    let total_h = PAD * 2.0 + (max_depth + 1) as f64 * (NODE_ROLE_H + LEVEL_GAP_Y);

    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", total_w, total_h, total_w, total_h);
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let content_y = PAD;

    fn render_node(svg: &mut String, people: &[Person], id: usize, cx: f64, y: f64) {
        let p = &people[id];
        let w = node_w(p);
        let h = if p.role.is_some() { NODE_ROLE_H } else { NODE_H };
        let d = depth(people, id);
        let (bg, accent) = COLORS[d % COLORS.len()];

        let rx = cx - w / 2.0;
        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>", rx, y, w, h, bg, accent));
        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>", cx, y + (if p.role.is_some() { 20.0 } else { h / 2.0 + 5.0 }), accent, ex(&p.name)));
        if let Some(ref role) = p.role {
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#666\">{}</text>", cx, y + 38.0, ex(role)));
        }

        let children = children_of(people, id);
        if children.is_empty() { return; }

        let total_cw: f64 = children.iter().map(|&c| subtree_width(people, c)).sum::<f64>()
            + (children.len() - 1) as f64 * NODE_GAP_X;
        let child_y = y + h + LEVEL_GAP_Y;
        let mut child_x = cx - total_cw / 2.0;

        // Connector line down from parent
        let conn_y = y + h;
        let mid_y = conn_y + LEVEL_GAP_Y / 2.0;
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"1.5\"/>", cx, conn_y, cx, mid_y));

        for &cid in &children {
            let csw = subtree_width(people, cid);
            let ccx = child_x + csw / 2.0;
            // Horizontal + vertical connector
            svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"1.5\"/>", cx.min(ccx), mid_y, cx.max(ccx), mid_y));
            svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"1.5\"/>", ccx, mid_y, ccx, child_y));
            render_node(svg, people, cid, ccx, child_y);
            child_x += csw + NODE_GAP_X;
        }
    }

    let mut root_x = PAD;
    for &rid in &roots {
        let sw = subtree_width(&org.people, rid);
        render_node(&mut svg, &org.people, rid, root_x + sw / 2.0, content_y);
        root_x += sw + NODE_GAP_X;
    }

    svg.push_str("</svg>");
    svg
}

const HELP: &str = "\
mdd-org - Render an org chart as SVG

Usage: mdd-org < input.org

Define members with \"member Name\" or \"member Name : Role\".
Connect them with \"Parent -> Child\".

Example:
  member CEO : \"Chief Executive\"
  member CTO : \"Chief Technology\"
  member Dev
  CEO -> CTO
  CTO -> Dev
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(o) => print!("{}", render_svg(&o)),
        Err(e) => { eprintln!("mdd-org: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "member CEO : \"代表\"\nmember CTO : \"技術\"\nCEO -> CTO\n";
        let o = parse(input).unwrap();
        assert_eq!(o.people.len(), 2);
        assert_eq!(o.people[1].parent, Some(0));
    }
    #[test]
    fn render_output() {
        let input = "member A\nmember B\nA -> B\n";
        let o = parse(input).unwrap();
        let svg = render_svg(&o);
        assert!(svg.starts_with("<svg"));
    }
}
