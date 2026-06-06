use std::collections::HashMap;
use std::io::{self, Read};

use rust_sugiyama::{configure::Config, from_vertices_and_edges};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Column {
    name: String,
    is_pk: bool,
}

#[derive(Debug)]
struct Table {
    name: String,
    columns: Vec<Column>,
}

#[derive(Debug)]
struct Relation {
    from: usize,
    to: usize,
    from_card: String, // "1" or "*"
    to_card: String,
}

#[derive(Debug)]
struct Diagram {
    tables: Vec<Table>,
    relations: Vec<Relation>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut tables: Vec<Table> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut relations: Vec<Relation> = Vec::new();

    let mut in_table = false;
    let mut table_name = String::new();
    let mut table_columns: Vec<Column> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if in_table {
            if line == "}" {
                let id = tables.len();
                name_to_id.insert(table_name.clone(), id);
                tables.push(Table {
                    name: table_name.clone(),
                    columns: table_columns.clone(),
                });
                in_table = false;
                table_name.clear();
                table_columns.clear();
                continue;
            }
            if let Some(pk_name) = line.strip_prefix("* ") {
                table_columns.push(Column {
                    name: pk_name.trim().to_string(),
                    is_pk: true,
                });
            } else {
                table_columns.push(Column {
                    name: line.to_string(),
                    is_pk: false,
                });
            }
            continue;
        }

        if line.starts_with("table ") {
            let rest = line.strip_prefix("table ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                table_name = name.trim().to_string();
                table_columns.clear();
                in_table = true;
                continue;
            }
            return Err(format!("Invalid table syntax: {}", line));
        }

        // Relation: Users 1--* Orders
        if let Some(rel) = parse_relation(line, &name_to_id) {
            relations.push(rel?);
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if in_table {
        return Err(format!("Unclosed table block: {}", table_name));
    }

    Ok(Diagram {
        tables,
        relations,
    })
}

fn parse_relation(
    line: &str,
    name_to_id: &HashMap<String, usize>,
) -> Option<Result<Relation, String>> {
    // Find pattern: <name> <card>--<card> <name>
    // card is "1" or "*"
    let patterns = ["1--1", "1--*", "*--1", "*--*"];
    for pat in &patterns {
        if let Some(pos) = line.find(pat) {
            let from_name = line[..pos].trim();
            let to_name = line[pos + pat.len()..].trim();
            let from_card = pat.chars().next().unwrap().to_string();
            let to_card = pat.chars().last().unwrap().to_string();

            let from_id = match name_to_id.get(from_name) {
                Some(id) => *id,
                None => return Some(Err(format!("Unknown table: {}", from_name))),
            };
            let to_id = match name_to_id.get(to_name) {
                Some(id) => *id,
                None => return Some(Err(format!("Unknown table: {}", to_name))),
            };

            return Some(Ok(Relation {
                from: from_id,
                to: to_id,
                from_card,
                to_card,
            }));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_WIDTH: f64 = 8.0;
const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

const TBL_H_PAD: f64 = 16.0;
const TBL_HEADER_H: f64 = 28.0;
const TBL_MIN_W: f64 = 140.0;
const TBL_COL_GAP: f64 = 12.0;
const TBL_MAX_ROWS: usize = 10;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_HEADER_BG: &str = "#e8f5e9";
const COLOR_HEADER_TEXT: &str = "#2e7d32";
const COLOR_BODY_BG: &str = "#fff";
const COLOR_BODY_STROKE: &str = "#aaa";
const COLOR_PK: &str = "#c8a415";

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
    let complexity = diagram.tables.len() + diagram.relations.len();
    let factor = if complexity <= 10 {
        1.0 + (complexity as f64 / 20.0).sqrt() * 0.4
    } else if complexity <= 30 {
        1.0 + (complexity as f64 / 10.0).sqrt() * 0.6
    } else {
        2.0 + (complexity - 30) as f64 * 0.06
    }
    .min(5.0);

    let tbl_count = diagram.tables.len() as f64;
    SpacingConfig {
        nodesep: 30.0 * factor,
        ranksep: 50.0 * factor,
        component_gap: 30.0 * factor,
        vertex_spacing: 8.0 + tbl_count * 3.0,
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { 14.0 })
        .sum()
}

// ---------------------------------------------------------------------------
// Table sizing (multi-column layout for many columns)
// ---------------------------------------------------------------------------

fn col_display_name(col: &Column) -> String {
    if col.is_pk {
        format!("\u{1f511} {}", col.name) // key emoji for sizing
    } else {
        col.name.clone()
    }
}

fn column_layout(columns: &[Column]) -> (usize, Vec<f64>, usize) {
    if columns.is_empty() {
        return (1, vec![0.0], 0);
    }
    let num_cols = ((columns.len() + TBL_MAX_ROWS - 1) / TBL_MAX_ROWS).max(1);
    let num_rows = (columns.len() + num_cols - 1) / num_cols;

    let mut col_widths = vec![0.0_f64; num_cols];
    for (i, col) in columns.iter().enumerate() {
        let c = i / num_rows;
        let display = col_display_name(col);
        col_widths[c] = col_widths[c].max(text_width(&display));
    }
    (num_cols, col_widths, num_rows)
}

fn table_size(table: &Table) -> (f64, f64) {
    let header_w = text_width(&table.name) + TBL_H_PAD * 2.0;
    let (num_cols, col_widths, num_rows) = column_layout(&table.columns);
    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * TBL_COL_GAP;
    let w = header_w.max(inner_w + TBL_H_PAD * 2.0).max(TBL_MIN_W);
    let body_h = if table.columns.is_empty() {
        8.0
    } else {
        num_rows as f64 * LINE_HEIGHT + 8.0
    };
    let h = TBL_HEADER_H + body_h;
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
        let curr = points[i];
        let next = points[i + 1];
        let prev = points[i - 1];
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
            let x = mt * mt * start.0 + 2.0 * mt * t * ctrl.0 + t * t * end.0;
            let y = mt * mt * start.1 + 2.0 * mt * t * ctrl.1 + t * t * end.1;
            result.push((x, y));
        }
    }
    result.push(last);
    result
}

#[allow(dead_code)]
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
    let total = *lengths.last().unwrap();
    let half = total / 2.0;

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

/// Point near start/end of path for cardinality labels
fn point_near_end(points: &[(f64, f64)], from_start: bool, dist: f64) -> (f64, f64) {
    let samples = sample_smooth_path(points, 64);
    if samples.len() < 2 {
        return samples.first().copied().unwrap_or((0.0, 0.0));
    }

    if from_start {
        let mut traveled = 0.0;
        for i in 1..samples.len() {
            let dx = samples[i].0 - samples[i - 1].0;
            let dy = samples[i].1 - samples[i - 1].1;
            let seg_len = (dx * dx + dy * dy).sqrt();
            traveled += seg_len;
            if traveled >= dist {
                return samples[i];
            }
        }
        *samples.last().unwrap()
    } else {
        let mut traveled = 0.0;
        for i in (0..samples.len() - 1).rev() {
            let dx = samples[i + 1].0 - samples[i].0;
            let dy = samples[i + 1].1 - samples[i].1;
            let seg_len = (dx * dx + dy * dy).sqrt();
            traveled += seg_len;
            if traveled >= dist {
                return samples[i];
            }
        }
        samples[0]
    }
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
        .tables
        .iter()
        .enumerate()
        .map(|(i, tbl)| {
            let (w, h) = table_size(tbl);
            (i as u32, (h, w)) // swap for LTR
        })
        .collect();

    // Exclude self-referencing relations from layout (they cause cycles)
    let edges: Vec<(u32, u32)> = diagram
        .relations
        .iter()
        .filter(|r| r.from != r.to)
        .map(|r| (r.from as u32, r.to as u32))
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
                let final_x = new_sy;
                let final_y = new_sx;

                let (w, h) = table_size(&diagram.tables[id]);
                min_x = min_x.min(final_x);
                min_y = min_y.min(final_y);
                max_x = max_x.max(final_x + w);
                max_y = max_y.max(final_y + h);
                scaled.insert(id, (final_x, final_y));
            }

            ((max_x - min_x).max(0.0), (max_y - min_y).max(0.0));
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
    for (i, tbl) in diagram.tables.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        let (w, h) = table_size(tbl);
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

    // Render tables
    for (i, tbl) in diagram.tables.iter().enumerate() {
        let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
        render_table(&mut svg, PADDING + x, PADDING + y, tbl);
    }

    // Build node bounds for routing
    let node_bounds: Vec<(f64, f64, f64, f64)> = diagram
        .tables
        .iter()
        .enumerate()
        .map(|(i, tbl)| {
            let (x, y) = positions.get(&i).copied().unwrap_or((0.0, 0.0));
            let (w, h) = table_size(tbl);
            (PADDING + x + w / 2.0, PADDING + y + h / 2.0, w / 2.0, h / 2.0)
        })
        .collect();

    // Reciprocal edge counting
    let mut pair_count: HashMap<(usize, usize), usize> = HashMap::new();
    for rel in &diagram.relations {
        let key = (rel.from.min(rel.to), rel.from.max(rel.to));
        *pair_count.entry(key).or_insert(0) += 1;
    }
    let mut pair_seen: HashMap<(usize, usize), usize> = HashMap::new();

    // Render relations
    for rel in &diagram.relations {
        let (x1, y1) = positions.get(&rel.from).copied().unwrap_or((0.0, 0.0));
        let (x2, y2) = positions.get(&rel.to).copied().unwrap_or((0.0, 0.0));
        let (fw, fh) = table_size(&diagram.tables[rel.from]);
        let (tw, th) = table_size(&diagram.tables[rel.to]);

        let cx1 = PADDING + x1 + fw / 2.0;
        let cy1 = PADDING + y1 + fh / 2.0;
        let cx2 = PADDING + x2 + tw / 2.0;
        let cy2 = PADDING + y2 + th / 2.0;

        // Self-referencing relation: draw a loop on the right side
        if rel.from == rel.to {
            let rx = PADDING + x1 + fw;
            let ry_top = PADDING + y1 + fh * 0.3;
            let ry_bot = PADDING + y1 + fh * 0.7;
            let bulge = 40.0;
            svg.push_str(&format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                rx, ry_top,
                rx + bulge, ry_top - 15.0,
                rx + bulge, ry_bot + 15.0,
                rx, ry_bot,
                COLOR_EDGE
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                rx + bulge + 4.0, (ry_top + ry_bot) / 2.0 - 6.0, COLOR_EDGE, escape_xml(&rel.from_card)
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"start\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>",
                rx + bulge + 4.0, (ry_top + ry_bot) / 2.0 + 10.0, COLOR_EDGE, escape_xml(&rel.to_card)
            ));
            continue;
        }

        let pair_key = (rel.from.min(rel.to), rel.from.max(rel.to));
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

        let route = route_around_nodes(cx1, cy1, cx2, cy2, rel.from, rel.to, &node_bounds, offset);

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
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            path_d, COLOR_EDGE
        ));

        // Cardinality labels near endpoints
        let near_start = point_near_end(&clipped_route, true, 18.0);
        let near_end = point_near_end(&clipped_route, false, 18.0);

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            near_start.0, near_start.1 - 6.0, COLOR_EDGE, escape_xml(&rel.from_card)
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            near_end.0, near_end.1 - 6.0, COLOR_EDGE, escape_xml(&rel.to_card)
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn render_table(svg: &mut String, x: f64, y: f64, table: &Table) {
    let (w, h) = table_size(table);

    // Body background + border
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x, y, w, h, COLOR_BODY_BG, COLOR_BODY_STROKE
    ));

    // Header background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\"/>",
        x, y, w, TBL_HEADER_H, COLOR_HEADER_BG
    ));
    // Cover bottom corners of header (they overlap with body)
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"4\" fill=\"{}\"/>",
        x, y + TBL_HEADER_H - 4.0, w, COLOR_HEADER_BG
    ));

    // Header text
    let cx = x + w / 2.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        cx,
        y + TBL_HEADER_H * 0.7,
        COLOR_HEADER_TEXT,
        escape_xml(&table.name)
    ));

    if table.columns.is_empty() {
        return;
    }

    // Column names in grid
    let (num_cols, col_widths, num_rows) = column_layout(&table.columns);
    let inner_w: f64 =
        col_widths.iter().sum::<f64>() + (num_cols as f64 - 1.0).max(0.0) * TBL_COL_GAP;
    let grid_start_x = x + (w - inner_w) / 2.0;

    for (i, col) in table.columns.iter().enumerate() {
        let display_col = i / num_rows;
        let display_row = i % num_rows;

        let col_x: f64 =
            col_widths[..display_col].iter().sum::<f64>() + display_col as f64 * TBL_COL_GAP;
        let text_y = y + TBL_HEADER_H + (display_row as f64 + 0.75) * LINE_HEIGHT;

        if col.is_pk {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"{}\">🔑</text>",
                grid_start_x + col_x,
                text_y,
                COLOR_PK
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\" font-weight=\"bold\">{}</text>",
                grid_start_x + col_x + 18.0,
                text_y,
                escape_xml(&col.name)
            ));
        } else {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#555\">{}</text>",
                grid_start_x + col_x,
                text_y,
                escape_xml(&col.name)
            ));
        }
    }
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

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let diagram = match parse(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mdd-er: {}", e);
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
    fn parse_table() {
        let input = "table Users {\n  * id\n  name\n  email\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tables.len(), 1);
        assert_eq!(d.tables[0].name, "Users");
        assert_eq!(d.tables[0].columns.len(), 3);
        assert!(d.tables[0].columns[0].is_pk);
        assert!(!d.tables[0].columns[1].is_pk);
    }

    #[test]
    fn parse_relation_1_to_many() {
        let input = "table A {\n  * id\n}\ntable B {\n  * id\n  a_id\n}\nA 1--* B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.relations[0].from_card, "1");
        assert_eq!(d.relations[0].to_card, "*");
    }

    #[test]
    fn parse_relation_many_to_many() {
        let input = "table A {\n  * id\n}\ntable B {\n  * id\n}\nA *--* B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.relations[0].from_card, "*");
        assert_eq!(d.relations[0].to_card, "*");
    }

    #[test]
    fn parse_relation_1_to_1() {
        let input = "table A {\n  * id\n}\ntable B {\n  * id\n}\nA 1--1 B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.relations[0].from_card, "1");
        assert_eq!(d.relations[0].to_card, "1");
    }

    #[test]
    fn render_produces_svg() {
        let input = "table A {\n  * id\n}\ntable B {\n  * id\n}\nA 1--* B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn spacing_scales_with_complexity() {
        let small = parse("table A {\n  * id\n}\ntable B {\n  * id\n}\nA 1--1 B\n").unwrap();
        let small_sp = compute_spacing(&small);

        let big_input = "table A {\n  * id\n}\ntable B {\n  * id\n}\ntable C {\n  * id\n}\ntable D {\n  * id\n}\ntable E {\n  * id\n}\nA 1--* B\nB 1--* C\nC 1--* D\nD 1--* E\n";
        let big = parse(big_input).unwrap();
        let big_sp = compute_spacing(&big);

        assert!(big_sp.nodesep > small_sp.nodesep);
        assert!(big_sp.ranksep > small_sp.ranksep);
    }
}
