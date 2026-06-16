use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExtKind {
    DataStore,
    Queue,
    Entity,
    File,
}

impl ExtKind {
    fn colors(&self) -> (&'static str, &'static str) {
        match self {
            ExtKind::DataStore => ("#f0fff0", "#339966"),
            ExtKind::Queue => ("#f3e5f5", "#7b1fa2"),
            ExtKind::Entity => ("#fff5ee", "#996633"),
            ExtKind::File => ("#fffde7", "#f9a825"),
        }
    }
}

#[derive(Debug)]
struct Actor {
    name: String,
}

#[derive(Debug)]
struct ExtNode {
    name: String,
    kind: ExtKind,
}

#[derive(Debug)]
struct ExtLink {
    target: String,  // ext node name
    label: String,
}

#[derive(Debug)]
struct Step {
    actor: String,     // who
    process: String,   // what
    ext_links: Vec<ExtLink>, // connections to external nodes
}

#[derive(Debug)]
struct Scenario {
    actors: Vec<Actor>,
    ext_nodes: Vec<ExtNode>,
    steps: Vec<Step>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Scenario, String> {
    let mut actors: Vec<Actor> = Vec::new();
    let mut actor_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut ext_nodes: Vec<ExtNode> = Vec::new();
    let mut ext_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut steps: Vec<Step> = Vec::new();

    let ext_kinds: &[(&str, ExtKind)] = &[
        ("datastore ", ExtKind::DataStore),
        ("queue ", ExtKind::Queue),
        ("entity ", ExtKind::Entity),
        ("file ", ExtKind::File),
    ];

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // actor Name
        if trimmed.starts_with("actor ") {
            let name = trimmed.strip_prefix("actor ").unwrap().trim().to_string();
            actor_names.insert(name.clone());
            actors.push(Actor { name });
            continue;
        }

        // External nodes
        let mut ext_matched = false;
        for (prefix, kind) in ext_kinds {
            if trimmed.starts_with(prefix) {
                let name = trimmed.strip_prefix(prefix).unwrap().trim().to_string();
                ext_names.insert(name.clone());
                ext_nodes.push(ExtNode { name, kind: *kind });
                ext_matched = true;
                break;
            }
        }
        if ext_matched { continue; }

        // Ext link: "  -> Target : "label""  (indented, belongs to previous step)
        if trimmed.starts_with("-> ") {
            let rest = trimmed.strip_prefix("-> ").unwrap().trim();
            let (target, label) = if let Some((t, l)) = rest.split_once(" : ") {
                (t.trim().to_string(), l.trim().trim_matches('"').to_string())
            } else {
                (rest.to_string(), String::new())
            };
            if !ext_names.contains(&target) {
                return Err(format!("Unknown external node: {}", target));
            }
            if let Some(step) = steps.last_mut() {
                step.ext_links.push(ExtLink { target, label });
            } else {
                return Err(format!("-> without a preceding step: {}", trimmed));
            }
            continue;
        }

        // Step: "ActorName : ProcessName"
        if trimmed.contains(" : ") {
            let (actor, process) = trimmed.split_once(" : ").unwrap();
            let actor = actor.trim().to_string();
            let process = process.trim().trim_matches('"').to_string();
            if !actor_names.contains(&actor) {
                return Err(format!("Unknown actor: {}", actor));
            }
            steps.push(Step {
                actor,
                process,
                ext_links: Vec::new(),
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", trimmed));
    }

    if actors.is_empty() {
        return Err("At least 1 actor required".to_string());
    }
    if steps.is_empty() {
        return Err("At least 1 step required".to_string());
    }

    Ok(Scenario { actors, ext_nodes, steps })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const FONT_SIZE: f64 = 12.0;
const PAD: f64 = 30.0;

const LANE_HEADER_H: f64 = 44.0;
const LANE_MIN_W: f64 = 160.0;
const LANE_H_PAD: f64 = 20.0;
const LANE_GAP: f64 = 6.0;

const PROCESS_RY: f64 = 22.0;  // vertical radius
const PROCESS_H_PAD: f64 = 16.0; // horizontal padding for text
const STEP_GAP: f64 = 40.0;
const STEP_Y_START: f64 = 30.0;

const EXT_COL_GAP: f64 = 40.0;
const EXT_NODE_W: f64 = 130.0;
const EXT_NODE_H: f64 = 32.0;
const EXT_NODE_GAP: f64 = 16.0;

const COLOR_DARK: &str = "#333";
const COLOR_PROCESS_FILL: &str = "#f0f8ff";
const COLOR_PROCESS_STROKE: &str = "#336699";
const COLOR_LANE_BG: &[&str] = &["#fafafa", "#f5f5f5"];
const COLOR_LANE_HEADER_BG: &str = "#e8eaf6";
const COLOR_LANE_STROKE: &str = "#e0e0e0";
const COLOR_ARROW: &str = "#888";
const COLOR_STEP_NUM: &str = "#336699";
const COLOR_TIMELINE: &str = "#ddd";

fn text_width(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn process_rx(label: &str) -> f64 {
    (text_width(label) / 2.0 + PROCESS_H_PAD).max(30.0)
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(scenario: &Scenario) -> String {
    let n_lanes = scenario.actors.len();
    let n_steps = scenario.steps.len();
    let n_ext = scenario.ext_nodes.len();

    let actor_to_lane: HashMap<&str, usize> = scenario.actors.iter().enumerate()
        .map(|(i, a)| (a.name.as_str(), i)).collect();

    // Lane widths
    let mut lane_w: Vec<f64> = scenario.actors.iter()
        .map(|a| (text_width(&a.name) + LANE_H_PAD * 2.0 + 40.0).max(LANE_MIN_W))
        .collect();
    for step in &scenario.steps {
        if let Some(&li) = actor_to_lane.get(step.actor.as_str()) {
            let pw = process_rx(&step.process) * 2.0 + LANE_H_PAD;
            lane_w[li] = lane_w[li].max(pw);
        }
    }

    // Ext node widths
    let ext_max_w = scenario.ext_nodes.iter()
        .map(|e| (text_width(&e.name) + 24.0).max(EXT_NODE_W))
        .fold(EXT_NODE_W, f64::max);

    // Split ext nodes: odd indices go left, even go right (by declaration order)
    let left_ext: Vec<usize> = (0..n_ext).filter(|i| i % 2 == 1).collect();
    let right_ext: Vec<usize> = (0..n_ext).filter(|i| i % 2 == 0).collect();
    let has_left = !left_ext.is_empty();
    let has_right = !right_ext.is_empty();

    // X layout: [left_ext] [lanes] [right_ext]
    let left_col_w = if has_left { ext_max_w + EXT_COL_GAP } else { 0.0 };
    let right_col_w = if has_right { EXT_COL_GAP + ext_max_w } else { 0.0 };

    let lanes_start_x = PAD + left_col_w;
    let mut lane_x: Vec<f64> = Vec::new();
    let mut cx = lanes_start_x;
    for (i, &w) in lane_w.iter().enumerate() {
        lane_x.push(cx);
        cx += w;
        if i < n_lanes - 1 { cx += LANE_GAP; }
    }
    let lanes_end_x = cx;
    let lane_cx: Vec<f64> = (0..n_lanes).map(|i| lane_x[i] + lane_w[i] / 2.0).collect();

    let left_ext_cx = if has_left { PAD + ext_max_w / 2.0 } else { 0.0 };
    let right_ext_cx = if has_right { lanes_end_x + EXT_COL_GAP + ext_max_w / 2.0 } else { 0.0 };

    let total_w = lanes_end_x + right_col_w + PAD;

    // Step y positions
    let content_top = PAD + LANE_HEADER_H + STEP_Y_START;
    let step_height = PROCESS_RY * 2.0 + STEP_GAP;
    let step_ys: Vec<f64> = (0..n_steps).map(|i| content_top + i as f64 * step_height).collect();

    // Place ext nodes on left/right sides, ordered by barycenter (average y of connected steps)
    let last_step_y = if n_steps > 0 { step_ys[n_steps - 1] + PROCESS_RY * 2.0 } else { content_top + 100.0 };
    let top_y = content_top + EXT_NODE_H / 2.0;
    let bottom_y = last_step_y - EXT_NODE_H / 2.0;

    // Compute barycenter y for each ext node
    let mut ext_bary: Vec<(usize, f64)> = Vec::new(); // (ext index, barycenter y)
    for (ei, ext) in scenario.ext_nodes.iter().enumerate() {
        let connected_ys: Vec<f64> = scenario.steps.iter().enumerate()
            .filter(|(_, s)| s.ext_links.iter().any(|l| l.target == ext.name))
            .map(|(si, _)| step_ys[si] + PROCESS_RY)
            .collect();
        let bary = if connected_ys.is_empty() {
            (top_y + bottom_y) / 2.0
        } else {
            connected_ys.iter().sum::<f64>() / connected_ys.len() as f64
        };
        ext_bary.push((ei, bary));
    }

    // Split into left/right sides (alternating), then sort each side by barycenter
    let mut right_exts: Vec<(usize, f64)> = Vec::new();
    let mut left_exts: Vec<(usize, f64)> = Vec::new();
    for (i, &(ei, bary)) in ext_bary.iter().enumerate() {
        if i % 2 == 0 { right_exts.push((ei, bary)); } else { left_exts.push((ei, bary)); }
    }
    right_exts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    left_exts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Distribute vertically with minimum gap
    let min_ext_gap = EXT_NODE_H + EXT_NODE_GAP;
    fn distribute_y(nodes: &[(usize, f64)], top: f64, bottom: f64, gap: f64) -> Vec<(usize, f64)> {
        let n = nodes.len();
        if n == 0 { return vec![]; }
        if n == 1 { return vec![(nodes[0].0, nodes[0].1.max(top).min(bottom))]; }
        // Space evenly between top and bottom, but respect minimum gap
        let total_needed = (n - 1) as f64 * gap;
        let available = bottom - top;
        let actual_gap = if total_needed > available { gap } else { available / (n - 1) as f64 };
        let start = if total_needed > available { top } else { top + (available - (n - 1) as f64 * actual_gap) / 2.0 };
        nodes.iter().enumerate().map(|(i, &(ei, _))| {
            (ei, start + i as f64 * actual_gap)
        }).collect()
    }

    let right_placed = distribute_y(&right_exts, top_y, bottom_y, min_ext_gap);
    let left_placed = distribute_y(&left_exts, top_y, bottom_y, min_ext_gap);

    // Build ext_y and side assignment
    let mut ext_y: HashMap<&str, f64> = HashMap::new();
    let mut ext_side: HashMap<&str, bool> = HashMap::new(); // true = left
    for (ei, y) in &right_placed {
        ext_y.insert(&scenario.ext_nodes[*ei].name, *y);
        ext_side.insert(&scenario.ext_nodes[*ei].name, false);
    }
    for (ei, y) in &left_placed {
        ext_y.insert(&scenario.ext_nodes[*ei].name, *y);
        ext_side.insert(&scenario.ext_nodes[*ei].name, true);
    }

    // Ext node x: left or right based on side assignment
    let ext_cx_map: HashMap<&str, f64> = scenario.ext_nodes.iter()
        .map(|e| {
            let is_left = ext_side.get(e.name.as_str()).copied().unwrap_or(false);
            let ecx = if is_left { left_ext_cx } else { right_ext_cx };
            (e.name.as_str(), ecx)
        }).collect();

    let last_step_bottom = if n_steps > 0 { step_ys[n_steps - 1] + PROCESS_RY * 2.0 } else { content_top };
    let ext_bottom = ext_y.values().copied().fold(0.0_f64, f64::max) + EXT_NODE_H;
    let total_h = last_step_bottom.max(ext_bottom) + PAD;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_ARROW
    ));

    // Lane backgrounds
    let lane_h = total_h - PAD * 2.0;
    for (li, actor) in scenario.actors.iter().enumerate() {
        let lx = lane_x[li];
        let lw = lane_w[li];
        let bg = COLOR_LANE_BG[li % COLOR_LANE_BG.len()];

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            lx, PAD, lw, lane_h, bg, COLOR_LANE_STROKE
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            lx, PAD, lw, LANE_HEADER_H, COLOR_LANE_HEADER_BG
        ));
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            lx, PAD + LANE_HEADER_H, lx + lw, PAD + LANE_HEADER_H, COLOR_LANE_STROKE
        ));

        let hcx = lane_cx[li];
        let hcy = PAD + LANE_HEADER_H / 2.0;
        // Stick figure
        svg.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>", hcx - 30.0, hcy - 6.0, COLOR_DARK));
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>", hcx - 30.0, hcy - 1.0, hcx - 30.0, hcy + 8.0, COLOR_DARK));
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>", hcx - 37.0, hcy + 3.0, hcx - 23.0, hcy + 3.0, COLOR_DARK));
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>", hcx - 30.0, hcy + 8.0, hcx - 35.0, hcy + 14.0, COLOR_DARK));
        svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>", hcx - 30.0, hcy + 8.0, hcx - 25.0, hcy + 14.0, COLOR_DARK));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            hcx - 16.0, hcy + 5.0, COLOR_DARK, escape_xml(&actor.name)
        ));

        // Timeline
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>",
            hcx, PAD + LANE_HEADER_H + 8.0, hcx, total_h - PAD, COLOR_TIMELINE
        ));
    }

    // External nodes (left and right)
    for (ei, ext) in scenario.ext_nodes.iter().enumerate() {
        if let (Some(&ey), Some(&ecx)) = (ext_y.get(ext.name.as_str()), ext_cx_map.get(ext.name.as_str())) {
            let (fill, stroke) = ext.kind.colors();
            let nw = (text_width(&ext.name) + 24.0).max(EXT_NODE_W);
            let nx = ecx - nw / 2.0;
            let ny = ey - EXT_NODE_H / 2.0;

            match ext.kind {
                ExtKind::DataStore => {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
                        nx, ny, nw, EXT_NODE_H, fill));
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        nx, ny, nx + nw, ny, stroke));
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        nx, ny + EXT_NODE_H, nx + nw, ny + EXT_NODE_H, stroke));
                }
                _ => {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        nx, ny, nw, EXT_NODE_H, fill, stroke));
                }
            }
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                ecx, ey + 4.0, stroke, escape_xml(&ext.name)
            ));
        }
    }

    // Steps
    for (si, step) in scenario.steps.iter().enumerate() {
        let li = *actor_to_lane.get(step.actor.as_str()).unwrap();
        let pcx = lane_cx[li];
        let pcy = step_ys[si] + PROCESS_RY;

        // Process ellipse
        let prx = process_rx(&step.process);
        svg.push_str(&format!(
            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            pcx, pcy, prx, PROCESS_RY, COLOR_PROCESS_FILL, COLOR_PROCESS_STROKE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            pcx, pcy - 4.0, COLOR_STEP_NUM, si + 1
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10\" fill=\"{}\">{}</text>",
            pcx, pcy + 10.0, COLOR_PROCESS_STROKE, escape_xml(&step.process)
        ));

        // Connect to previous step (swimlane flow)
        if si > 0 {
            let prev_li = *actor_to_lane.get(scenario.steps[si - 1].actor.as_str()).unwrap();
            let prev_pcx = lane_cx[prev_li];
            let prev_pcy = step_ys[si - 1] + PROCESS_RY;

            if prev_li == li {
                // Same lane: vertical arrow
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                    pcx, prev_pcy + PROCESS_RY + 2.0, pcx, pcy - PROCESS_RY - 6.0, COLOR_PROCESS_STROKE
                ));
            } else {
                // Cross-lane: curved arrow
                let from_y = prev_pcy + PROCESS_RY;
                let to_y = pcy - PROCESS_RY;
                let mid_y = (from_y + to_y) / 2.0;
                svg.push_str(&format!(
                    "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                    prev_pcx, from_y + 2.0,
                    prev_pcx, mid_y,
                    pcx, mid_y,
                    pcx, to_y - 6.0,
                    COLOR_PROCESS_STROKE
                ));
            }
        }

        // Ext links
        for link in &step.ext_links {
            if let (Some(&ey), Some(&ecx)) = (ext_y.get(link.target.as_str()), ext_cx_map.get(link.target.as_str())) {
                let ext_nw = (text_width(&link.target) + 24.0).max(EXT_NODE_W);
                let is_left = ecx < pcx;

                let ax1 = if is_left { pcx - prx } else { pcx + prx };
                let ay1 = pcy;
                let ax2 = if is_left { ecx + ext_nw / 2.0 + 6.0 } else { ecx - ext_nw / 2.0 - 6.0 };
                let ay2 = ey;

                let dx = (ax2 - ax1).abs();
                let ctrl_dist = dx * 0.3;

                let ext_kind = scenario.ext_nodes.iter().find(|e| e.name == link.target).map(|e| e.kind);
                let (_, stroke) = ext_kind.map(|k| k.colors()).unwrap_or(("#fff", "#888"));

                if is_left {
                    svg.push_str(&format!(
                        "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" marker-end=\"url(#arrow)\" opacity=\"0.7\"/>",
                        ax1, ay1, ax1 - ctrl_dist, ay1, ax2 + ctrl_dist, ay2, ax2, ay2, stroke
                    ));
                } else {
                    svg.push_str(&format!(
                        "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\" marker-end=\"url(#arrow)\" opacity=\"0.7\"/>",
                        ax1, ay1, ax1 + ctrl_dist, ay1, ax2 - ctrl_dist, ay2, ax2, ay2, stroke
                    ));
                }

                if !link.label.is_empty() {
                    let mx = (ax1 + ax2) / 2.0;
                    let my = (ay1 + ay2) / 2.0 - 8.0;
                    let lw = text_width(&link.label);
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"13\" rx=\"2\" fill=\"white\" opacity=\"0.9\"/>",
                        mx - lw / 2.0 - 2.0, my - 9.0, lw + 4.0
                    ));
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9\" fill=\"{}\">{}</text>",
                        mx, my, stroke, escape_xml(&link.label)
                    ));
                }
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-scenario - Render a system scenario as a swimlane diagram

Usage: mdd-scenario < input.txt

Actors become swimlanes. Steps are \"who : what\" flowing top-to-bottom.
External connections (datastores, queues) use \"-> Target : label\".

Syntax:
  actor Name
  datastore Name
  queue Name

  ActorName : ProcessName
    -> DatastoreName : \"label\"
    -> QueueName : \"label\"

Example:
  actor ユーザー
  actor 社員
  datastore ユーザーDB
  queue メール送信キュー

  ユーザー : 会員登録フォーム送信
    -> ユーザーDB : \"仮登録\"
    -> メール送信キュー : \"確認メール依頼\"

  ユーザー : メール認証リンククリック
    -> ユーザーDB : \"認証済み更新\"

  社員 : 会員審査実施
    -> ユーザーDB : \"本登録\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    let scenario = match parse(&input) {
        Ok(s) => s,
        Err(e) => { eprintln!("mdd-scenario: {}", e); std::process::exit(1); }
    };

    print!("{}", render_svg(&scenario));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "actor A\ndatastore DB\nA : do something\n  -> DB : \"save\"\n";
        let s = parse(input).unwrap();
        assert_eq!(s.actors.len(), 1);
        assert_eq!(s.ext_nodes.len(), 1);
        assert_eq!(s.steps.len(), 1);
        assert_eq!(s.steps[0].process, "do something");
        assert_eq!(s.steps[0].ext_links.len(), 1);
    }

    #[test]
    fn parse_multi_actor() {
        let input = "actor A\nactor B\nA : step1\nB : step2\nA : step3\n";
        let s = parse(input).unwrap();
        assert_eq!(s.steps.len(), 3);
        assert_eq!(s.steps[1].actor, "B");
    }

    #[test]
    fn render_produces_svg() {
        let input = "actor A\ndatastore DB\nA : do thing\n  -> DB : \"write\"\nA : another thing\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
