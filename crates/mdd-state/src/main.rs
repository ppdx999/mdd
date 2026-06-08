use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct State {
    name: String,
}

#[derive(Debug)]
struct Transition {
    from: usize,
    to: usize,
    label: String,
}

#[derive(Debug)]
struct Diagram {
    states: Vec<State>,
    transitions: Vec<Transition>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut states: Vec<State> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut transitions: Vec<Transition> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("state ") {
            let name = line.strip_prefix("state ").unwrap().trim().to_string();
            let id = states.len();
            name_to_id.insert(name.clone(), id);
            states.push(State { name });
            continue;
        }

        if line.contains(" -> ") {
            let parts: Vec<&str> = line.splitn(2, " -> ").collect();
            let from_name = parts[0].trim();
            let rest = parts[1];

            let (to_name, label) = if let Some((to_part, label_part)) = rest.split_once(" : ") {
                (
                    to_part.trim(),
                    label_part.trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim(), String::new())
            };

            let from_id = name_to_id
                .get(from_name)
                .ok_or_else(|| format!("Unknown state: {}", from_name))?;
            let to_id = name_to_id
                .get(to_name)
                .ok_or_else(|| format!("Unknown state: {}", to_name))?;
            transitions.push(Transition {
                from: *from_id,
                to: *to_id,
                label,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram {
        states,
        transitions,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

const STATE_H_PAD: f64 = 20.0;
const STATE_V_PAD: f64 = 12.0;
const STATE_MIN_W: f64 = 100.0;
const STATE_MIN_H: f64 = 40.0;
const STATE_RADIUS: f64 = 12.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_STATE_FILL: &str = "#f3e5f5";
const COLOR_STATE_STROKE: &str = "#7b1fa2";

// ---------------------------------------------------------------------------
// Spacing
// ---------------------------------------------------------------------------

struct SpacingConfig {
    nodesep: f64,
    ranksep: f64,
    component_gap: f64,
    vertex_spacing: f64,
}

fn compute_spacing(diagram: &Diagram) -> SpacingConfig {
    let complexity = diagram.states.len() + diagram.transitions.len();
    let factor = if complexity <= 10 {
        1.0 + (complexity as f64 / 20.0).sqrt() * 0.4
    } else if complexity <= 30 {
        1.0 + (complexity as f64 / 10.0).sqrt() * 0.6
    } else {
        2.0 + (complexity - 30) as f64 * 0.06
    }
    .min(5.0);

    let n = diagram.states.len() as f64;
    SpacingConfig {
        nodesep: 30.0 * factor,
        ranksep: 50.0 * factor,
        component_gap: 30.0 * factor,
        vertex_spacing: 8.0 + n * 3.0,
    }
}

// ---------------------------------------------------------------------------
// Text & sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

fn state_size(name: &str) -> (f64, f64) {
    let w = (text_width(name) + STATE_H_PAD * 2.0).max(STATE_MIN_W);
    let h = (LINE_HEIGHT + STATE_V_PAD * 2.0).max(STATE_MIN_H);
    (w, h)
}

// ---------------------------------------------------------------------------
// Edge routing (from DFD)
// ---------------------------------------------------------------------------

fn segment_intersects_rect(
    p1x: f64, p1y: f64, p2x: f64, p2y: f64,
    cx: f64, cy: f64, hw: f64, hh: f64,
) -> bool {
    let margin = 4.0;
    let left = cx - hw - margin;
    let right = cx + hw + margin;
    let top = cy - hh - margin;
    let bottom = cy + hh + margin;

    if (p1x < left && p2x < left) || (p1x > right && p2x > right) {
        return false;
    }
    if (p1y < top && p2y < top) || (p1y > bottom && p2y > bottom) {
        return false;
    }

    let dx = p2x - p1x;
    let dy = p2y - p1y;
    let edges: [(f64, f64, f64, f64); 4] = [
        (left, top, right, top),
        (left, bottom, right, bottom),
        (left, top, left, bottom),
        (right, top, right, bottom),
    ];

    for (ex1, ey1, ex2, ey2) in &edges {
        let edx = ex2 - ex1;
        let edy = ey2 - ey1;
        let denom = dx * edy - dy * edx;
        if denom.abs() < 1e-10 {
            continue;
        }
        let t = ((ex1 - p1x) * edy - (ey1 - p1y) * edx) / denom;
        let u = ((ex1 - p1x) * dy - (ey1 - p1y) * dx) / denom;
        if (0.01..=0.99).contains(&t) && (0.0..=1.0).contains(&u) {
            return true;
        }
    }
    false
}

fn route_around_nodes(
    sx: f64, sy: f64, ex: f64, ey: f64,
    from_id: usize, to_id: usize,
    bounds: &[(f64, f64, f64, f64)],
    offset: f64,
) -> Vec<(f64, f64)> {
    let (sx, sy, ex, ey) = if offset.abs() > 0.1 {
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let nx = -dy / len * offset;
        let ny = dx / len * offset;
        (sx + nx, sy + ny, ex + nx, ey + ny)
    } else {
        (sx, sy, ex, ey)
    };

    let mut blockers: Vec<usize> = Vec::new();
    for (i, &(cx, cy, hw, hh)) in bounds.iter().enumerate() {
        if i == from_id || i == to_id {
            continue;
        }
        if segment_intersects_rect(sx, sy, ex, ey, cx, cy, hw, hh) {
            blockers.push(i);
        }
    }

    if blockers.is_empty() {
        return vec![(sx, sy), (ex, ey)];
    }

    let margin = 20.0;
    let mut waypoints: Vec<(f64, f64)> = vec![(sx, sy)];

    blockers.sort_by(|a, b| {
        let (acx, acy, _, _) = bounds[*a];
        let (bcx, bcy, _, _) = bounds[*b];
        let da = (acx - sx).powi(2) + (acy - sy).powi(2);
        let db = (bcx - sx).powi(2) + (bcy - sy).powi(2);
        da.partial_cmp(&db).unwrap()
    });

    for &bi in &blockers {
        let (cx, cy, hw, hh) = bounds[bi];
        let dx = ex - sx;
        let dy = ey - sy;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let last = waypoints.last().unwrap();
        let cross = (cx - last.0) * dy - (cy - last.1) * dx;

        if cross.abs() / len < hw + hh {
            let top_y = cy - hh - margin;
            let bot_y = cy + hh + margin;
            let left_x = cx - hw - margin;
            let right_x = cx + hw + margin;

            if dy.abs() > dx.abs() {
                if cross > 0.0 {
                    waypoints.push((right_x, cy));
                } else {
                    waypoints.push((left_x, cy));
                }
            } else if cross > 0.0 {
                waypoints.push((cx, top_y));
            } else {
                waypoints.push((cx, bot_y));
            }
        }
    }

    waypoints.push((ex, ey));
    waypoints
}

fn build_smooth_path(points: &[(f64, f64)]) -> String {
    if points.len() < 2 {
        return String::new();
    }
    if points.len() == 2 {
        return format!(
            "M{},{} L{},{}",
            points[0].0, points[0].1, points[1].0, points[1].1
        );
    }
    let mut d = format!("M{},{}", points[0].0, points[0].1);
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 {
            d.push_str(&format!(" L{},{}", mid_prev.0, mid_prev.1));
        }
        d.push_str(&format!(
            " Q{},{} {},{}",
            curr.0, curr.1, mid_next.0, mid_next.1
        ));
    }
    let last = points[points.len() - 1];
    d.push_str(&format!(" L{},{}", last.0, last.1));
    d
}

fn sample_smooth_path(points: &[(f64, f64)], n: usize) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return points.to_vec();
    }
    if points.len() == 2 {
        return (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                (
                    points[0].0 + (points[1].0 - points[0].0) * t,
                    points[0].1 + (points[1].1 - points[0].1) * t,
                )
            })
            .collect();
    }
    let mut segments: Vec<((f64, f64), (f64, f64), (f64, f64))> = Vec::new();
    let mut cursor = points[0];
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];
        let mid_prev = ((prev.0 + curr.0) / 2.0, (prev.1 + curr.1) / 2.0);
        let mid_next = ((curr.0 + next.0) / 2.0, (curr.1 + next.1) / 2.0);
        if i == 1 {
            segments.push((cursor, cursor, mid_prev));
            cursor = mid_prev;
        }
        segments.push((cursor, curr, mid_next));
        cursor = mid_next;
    }
    let last = *points.last().unwrap();
    segments.push((cursor, cursor, last));
    let per_seg = (n / segments.len()).max(2);
    let mut result = Vec::new();
    for (start, ctrl, end) in &segments {
        for j in 0..per_seg {
            let t = j as f64 / per_seg as f64;
            let mt = 1.0 - t;
            result.push((
                mt * mt * start.0 + 2.0 * mt * t * ctrl.0 + t * t * end.0,
                mt * mt * start.1 + 2.0 * mt * t * ctrl.1 + t * t * end.1,
            ));
        }
    }
    result.push(last);
    result
}

fn midpoint_on_path(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() <= 1 {
        return points.first().copied().unwrap_or((0.0, 0.0));
    }
    if points.len() == 2 {
        return (
            (points[0].0 + points[1].0) / 2.0,
            (points[0].1 + points[1].1) / 2.0,
        );
    }
    let samples = sample_smooth_path(points, 64);
    let mut lengths = vec![0.0_f64];
    for i in 1..samples.len() {
        let dx = samples[i].0 - samples[i - 1].0;
        let dy = samples[i].1 - samples[i - 1].1;
        lengths.push(lengths[i - 1] + (dx * dx + dy * dy).sqrt());
    }
    let half = *lengths.last().unwrap() / 2.0;
    for i in 1..lengths.len() {
        if lengths[i] >= half {
            let t = (half - lengths[i - 1]) / (lengths[i] - lengths[i - 1]).max(1e-10);
            return (
                samples[i - 1].0 + (samples[i].0 - samples[i - 1].0) * t,
                samples[i - 1].1 + (samples[i].1 - samples[i - 1].1) * t,
            );
        }
    }
    *samples.last().unwrap()
}

fn clip_to_rect(cx: f64, cy: f64, tx: f64, ty: f64, hw: f64, hh: f64) -> (f64, f64) {
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let mut t = f64::MAX;
    if dx.abs() > 1e-9 {
        t = t.min(hw / dx.abs());
    }
    if dy.abs() > 1e-9 {
        t = t.min(hh / dy.abs());
    }
    (cx + dx * t, cy + dy * t)
}

// ---------------------------------------------------------------------------
// Layout & SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let sp = compute_spacing(diagram);

    let config = Config {
        vertex_spacing: sp.vertex_spacing,
        ..Config::default()
    };

    let vertices: Vec<(u32, (f64, f64))> = diagram
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (w, h) = state_size(&s.name);
            (i as u32, (h, w)) // swap for LTR
        })
        .collect();

    // Exclude self-transitions from layout
    let edges: Vec<(u32, u32)> = diagram
        .transitions
        .iter()
        .filter(|t| t.from != t.to)
        .map(|t| (t.from as u32, t.to as u32))
        .collect();

    let layouts = from_vertices_and_edges(&vertices, &edges, &config);

    // Post-scale
    let base = sp.vertex_spacing.max(1.0);
    let nodesep_ratio = sp.nodesep / base;
    let ranksep_ratio = sp.ranksep / base;

    let scaled_components: Vec<(HashMap<usize, (f64, f64)>, f64, f64)> = layouts
        .iter()
        .map(|(coords, _w, _h)| {
            let n = coords.len() as f64;
            let cx = coords.iter().map(|(_, (x, _))| x).sum::<f64>() / n;
            let cy = coords.iter().map(|(_, (_, y))| y).sum::<f64>() / n;

            let mut scaled: HashMap<usize, (f64, f64)> = HashMap::new();
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;

            for &(id, (sx, sy)) in coords {
                let new_sx = cx + (sx - cx) * nodesep_ratio;
                let new_sy = cy + (sy - cy) * ranksep_ratio;
                let final_x = new_sy; // swap back for LTR
                let final_y = new_sx;

                let (w, h) = state_size(&diagram.states[id].name);
                min_x = min_x.min(final_x);
                min_y = min_y.min(final_y);
                max_x = max_x.max(final_x + w);
                max_y = max_y.max(final_y + h);
                scaled.insert(id, (final_x, final_y));
            }

            (scaled, (max_x - min_x).max(0.0), (max_y - min_y).max(0.0))
        })
        .collect();

    // Row-based packing
    let total_area: f64 = scaled_components
        .iter()
        .map(|(_, w, h)| (w + sp.component_gap) * (h + sp.component_gap))
        .sum();
    let target_width = total_area.sqrt() * 1.3;

    let mut comp_indices: Vec<usize> = (0..scaled_components.len()).collect();
    comp_indices.sort_by(|a, b| {
        scaled_components[*b]
            .2
            .partial_cmp(&scaled_components[*a].2)
            .unwrap()
    });

    let mut positions: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut row_x: f64 = 0.0;
    let mut row_y: f64 = 0.0;
    let mut row_max_height: f64 = 0.0;

    for &ci in &comp_indices {
        let (ref coords, comp_w, comp_h) = scaled_components[ci];
        if row_x > 0.0 && row_x + comp_w > target_width {
            row_y += row_max_height + sp.component_gap;
            row_x = 0.0;
            row_max_height = 0.0;
        }
        let cmin_x = coords.values().map(|(x, _)| *x).fold(f64::MAX, f64::min);
        let cmin_y = coords.values().map(|(_, y)| *y).fold(f64::MAX, f64::min);
        for (&id, &(x, y)) in coords {
            positions.insert(id, (x - cmin_x + row_x, y - cmin_y + row_y));
        }
        row_x += comp_w + sp.component_gap;
        row_max_height = row_max_height.max(comp_h);
    }

    // SVG dimensions
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (i, s) in diagram.states.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = state_size(&s.name);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING * 2.0;
    let svg_height = max_y + PADDING * 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));
    svg.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto\"><polygon points=\"0,1 10,5 0,9\" fill=\"{}\"/></marker></defs>",
        COLOR_EDGE
    ));

    // Render states
    for (i, s) in diagram.states.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let px = PADDING + x;
        let py = PADDING + y;
        let (w, h) = state_size(&s.name);

        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            px, py, w, h, STATE_RADIUS, COLOR_STATE_FILL, COLOR_STATE_STROKE
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\">{}</text>",
            px + w / 2.0,
            py + h / 2.0 + LINE_HEIGHT * 0.35,
            escape_xml(&s.name)
        ));
    }

    // Node bounds for routing
    let node_bounds: Vec<(f64, f64, f64, f64)> = diagram
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
            let (w, h) = state_size(&s.name);
            (PADDING + x + w / 2.0, PADDING + y + h / 2.0, w / 2.0, h / 2.0)
        })
        .collect();

    // Reciprocal edge counting
    let mut pair_count: HashMap<(usize, usize), usize> = HashMap::new();
    for t in &diagram.transitions {
        if t.from == t.to {
            continue;
        }
        let key = (t.from.min(t.to), t.from.max(t.to));
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(usize, usize), usize> = HashMap::new();

    // Render transitions
    for trans in &diagram.transitions {
        let (x1, y1) = positions.get(&trans.from).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = state_size(&diagram.states[trans.from].name);
        let cx1 = PADDING + x1 + fw / 2.0;
        let cy1 = PADDING + y1 + fh / 2.0;

        // Self-transition
        if trans.from == trans.to {
            let rx = PADDING + x1 + fw;
            let ry_top = PADDING + y1 + fh * 0.3;
            let ry_bot = PADDING + y1 + fh * 0.7;
            let bulge = 35.0;
            svg.push_str(&format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
                rx, ry_top,
                rx + bulge, ry_top - 15.0,
                rx + bulge, ry_bot + 15.0,
                rx, ry_bot,
                COLOR_EDGE
            ));
            if !trans.label.is_empty() {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                    rx + bulge + 6.0,
                    (ry_top + ry_bot) / 2.0 + 4.0,
                    COLOR_EDGE,
                    escape_xml(&trans.label)
                ));
            }
            continue;
        }

        let (x2, y2) = positions.get(&trans.to).copied().unwrap_or((0.0, 0.0));
        let (tw, th) = state_size(&diagram.states[trans.to].name);
        let cx2 = PADDING + x2 + tw / 2.0;
        let cy2 = PADDING + y2 + th / 2.0;

        let pair_key = (trans.from.min(trans.to), trans.from.max(trans.to));
        let total = *pair_count.get(&pair_key).unwrap_or(&1);
        let idx = {
            let seen = pair_seen.entry(pair_key).or_insert(0);
            let v = *seen;
            *seen += 1;
            v
        };
        let offset = if total > 1 {
            (idx as f64 - (total as f64 - 1.0) / 2.0) * 15.0
        } else {
            0.0
        };

        let route = route_around_nodes(cx1, cy1, cx2, cy2, trans.from, trans.to, &node_bounds, offset);

        let start_target = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_target = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = clip_to_rect(cx1, cy1, start_target.0, start_target.1, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = clip_to_rect(cx2, cy2, end_target.0, end_target.1, tw / 2.0, th / 2.0);

        let mut clipped_route = vec![(ax1, ay1)];
        if route.len() > 2 {
            clipped_route.extend_from_slice(&route[1..route.len() - 1]);
        }
        clipped_route.push((ax2, ay2));

        let path_d = if clipped_route.len() == 2 {
            format!(
                "M{},{} L{},{}",
                clipped_route[0].0, clipped_route[0].1,
                clipped_route[1].0, clipped_route[1].1
            )
        } else {
            build_smooth_path(&clipped_route)
        };

        svg.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/>",
            path_d, COLOR_EDGE
        ));

        if !trans.label.is_empty() {
            let (mx, my) = midpoint_on_path(&clipped_route);
            let lx = mx;
            let ly = my - 6.0;
            let lw = text_width(&trans.label);
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"16\" rx=\"3\" fill=\"white\" opacity=\"0.85\"/>",
                lx - lw / 2.0 - 3.0, ly - 12.0, lw + 6.0
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" fill=\"{}\">{}</text>",
                lx, ly, COLOR_EDGE, escape_xml(&trans.label)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-state - Render a state machine diagram as SVG

Usage: mdd-state < input.state

Declare states with \"state Name\", then define transitions:
  From -> To : \"label\"
  From -> To              (no label)
Self-transitions (A -> A) are supported.

Example:
  state Idle
  state Running
  state Done

  Idle -> Running : \"start\"
  Running -> Done : \"finish\"
  Running -> Idle : \"cancel\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-state: {}", e);
            std::process::exit(1);
        }
    };

    let svg = render_svg(&diagram);
    print!("{}", svg);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_state() {
        let d = parse("state Idle\n").unwrap();
        assert_eq!(d.states.len(), 1);
        assert_eq!(d.states[0].name, "Idle");
    }

    #[test]
    fn parse_transition_with_label() {
        let input = "state A\nstate B\nA -> B : \"go\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions.len(), 1);
        assert_eq!(d.transitions[0].label, "go");
    }

    #[test]
    fn parse_transition_without_label() {
        let input = "state A\nstate B\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions[0].label, "");
    }

    #[test]
    fn parse_self_transition() {
        let input = "state A\nA -> A : \"retry\"\n";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions[0].from, d.transitions[0].to);
    }

    #[test]
    fn render_produces_svg() {
        let input = "state A\nstate B\nA -> B : \"go\"\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("arrow"));
    }

    #[test]
    fn spacing_scales_with_complexity() {
        let small = parse("state A\nstate B\nA -> B\n").unwrap();
        let small_sp = compute_spacing(&small);
        let big_input = "state A\nstate B\nstate C\nstate D\nstate E\n\
                         A -> B\nB -> C\nC -> D\nD -> E\nE -> A\n";
        let big = parse(big_input).unwrap();
        let big_sp = compute_spacing(&big);
        assert!(big_sp.nodesep > small_sp.nodesep);
    }
}
