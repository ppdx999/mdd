use std::collections::HashMap;
use std::io::{self, Read};

use mdd_layout::text::{text_width, escape_xml};

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
struct Group {
    name: String,
    children: Vec<Element>,
}

#[derive(Debug)]
enum Element {
    TableRef(usize),  // index into tables vec
    GroupRef(usize),  // index into groups vec
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
    groups: Vec<Group>,
    top_level: Vec<Element>,
    relations: Vec<Relation>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Diagram, String> {
    let mut tables: Vec<Table> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut top_level: Vec<Element> = Vec::new();
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut relations: Vec<Relation> = Vec::new();

    // Stack for nested groups/tables
    // GroupCtx: building a group
    // TableCtx: building a table
    enum Ctx {
        Group(usize, Vec<Element>),
        Table(String, Vec<Column>),
    }
    let mut stack: Vec<Ctx> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check if we're inside a table definition
        if let Some(Ctx::Table(_, _)) = stack.last() {
            if line == "}" {
                if let Some(Ctx::Table(table_name, table_columns)) = stack.pop() {
                    let id = tables.len();
                    name_to_id.insert(table_name.clone(), id);
                    tables.push(Table {
                        name: table_name,
                        columns: table_columns,
                    });
                    let elem = Element::TableRef(id);
                    if let Some(Ctx::Group(_, children)) = stack.last_mut() {
                        children.push(elem);
                    } else {
                        top_level.push(elem);
                    }
                }
                continue;
            }
            // Parse column: "name [PK|FK]" or "* name"
            if let Some(Ctx::Table(_, cols)) = stack.last_mut() {
                if let Some(pk_name) = line.strip_prefix("* ") {
                    cols.push(Column {
                        name: pk_name.trim().to_string(),
                        is_pk: true,
                    });
                } else {
                    // Check for "name PK" suffix
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[parts.len() - 1] == "PK" {
                        cols.push(Column {
                            name: parts[..parts.len() - 1].join(" "),
                            is_pk: true,
                        });
                    } else if parts.len() >= 2 && parts[parts.len() - 1] == "FK" {
                        // FK columns are not PK
                        cols.push(Column {
                            name: parts[..parts.len() - 1].join(" "),
                            is_pk: false,
                        });
                    } else {
                        cols.push(Column {
                            name: line.to_string(),
                            is_pk: false,
                        });
                    }
                }
            }
            continue;
        }

        if line == "}" {
            if let Some(Ctx::Group(gidx, children)) = stack.pop() {
                groups[gidx].children = children;
                let elem = Element::GroupRef(gidx);
                if let Some(Ctx::Group(_, parent_children)) = stack.last_mut() {
                    parent_children.push(elem);
                } else {
                    top_level.push(elem);
                }
            } else {
                return Err("Unexpected }".to_string());
            }
            continue;
        }

        // Group: group "name" {
        if line.starts_with("group ") {
            let rest = line.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().trim_matches('"').to_string();
                let gidx = groups.len();
                groups.push(Group {
                    name,
                    children: Vec::new(),
                });
                stack.push(Ctx::Group(gidx, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", line));
        }

        // Table/entity: "table Name {" or "entity Name {"
        if line.starts_with("table ") || line.starts_with("entity ") {
            let rest = if line.starts_with("table ") {
                line.strip_prefix("table ").unwrap()
            } else {
                line.strip_prefix("entity ").unwrap()
            };
            if let Some(name) = rest.strip_suffix(" {") {
                let name = name.trim().to_string();
                stack.push(Ctx::Table(name, Vec::new()));
                continue;
            }
            return Err(format!("Invalid table/entity syntax: {}", line));
        }

        // Relation: Users 1--* Orders
        if let Some(rel) = parse_relation(line, &name_to_id) {
            relations.push(rel?);
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    if !stack.is_empty() {
        return Err("Unclosed block".to_string());
    }

    Ok(Diagram {
        tables,
        groups,
        top_level,
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

const LINE_HEIGHT: f64 = 18.0;
const PADDING: f64 = 40.0;

const TBL_H_PAD: f64 = 16.0;
const TBL_HEADER_H: f64 = 28.0;
const TBL_MIN_W: f64 = 140.0;
const TBL_COL_GAP: f64 = 12.0;
const TBL_MAX_ROWS: usize = 10;

const GROUP_H_PAD: f64 = 16.0;
const GROUP_V_PAD: f64 = 12.0;
const GROUP_HEADER_H: f64 = 28.0;

const COLOR_DARK: &str = "#333";
const COLOR_EDGE: &str = "#666";
const COLOR_HEADER_BG: &str = "#e8f5e9";
const COLOR_HEADER_TEXT: &str = "#2e7d32";
const COLOR_BODY_BG: &str = "#fff";
const COLOR_BODY_STROKE: &str = "#aaa";
const COLOR_PK: &str = "#c8a415";
const COLOR_GROUP_FILL: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#bbb";

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
// Layout graph construction
// ---------------------------------------------------------------------------

fn build_layout_graph(diagram: &Diagram) -> mdd_layout::LayoutGraph {
    let mut graph = mdd_layout::LayoutGraph::new();

    // Add nodes (tables) with computed sizes
    for table in &diagram.tables {
        let (w, h) = table_size(table);
        graph.nodes.push(mdd_layout::LayoutNode {
            name: table.name.clone(),
            width: w,
            height: h,
        });
    }

    // Add groups (recursively convert children)
    for group in &diagram.groups {
        let children = group
            .children
            .iter()
            .map(|e| match e {
                Element::TableRef(i) => mdd_layout::LayoutElement::NodeRef(*i),
                Element::GroupRef(i) => mdd_layout::LayoutElement::GroupRef(*i),
            })
            .collect();
        graph.groups.push(mdd_layout::LayoutGroup {
            name: group.name.clone(),
            children,
        });
    }

    // Copy top_level elements
    graph.top_level = diagram
        .top_level
        .iter()
        .map(|e| match e {
            Element::TableRef(i) => mdd_layout::LayoutElement::NodeRef(*i),
            Element::GroupRef(i) => mdd_layout::LayoutElement::GroupRef(*i),
        })
        .collect();

    // Convert relations to layout edges (no labels, but the layout needs edge info)
    for rel in &diagram.relations {
        if rel.from != rel.to {
            graph.edges.push(mdd_layout::LayoutEdge {
                from: diagram.tables[rel.from].name.clone(),
                to: diagram.tables[rel.to].name.clone(),
                label: String::new(),
            });
        }
    }

    graph
}

// ---------------------------------------------------------------------------
// Edge helper: point near start/end of path for cardinality labels
// ---------------------------------------------------------------------------

fn point_near_end(points: &[(f64, f64)], from_start: bool, dist: f64) -> (f64, f64) {
    let samples = mdd_layout::edge::sample_smooth_path(points, 64);
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

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(diagram: &Diagram) -> String {
    let graph = build_layout_graph(diagram);
    let config = mdd_layout::LayoutConfig {
        padding: PADDING,
        group_h_pad: GROUP_H_PAD,
        group_v_pad: GROUP_V_PAD,
        group_header_h: GROUP_HEADER_H,
        ..mdd_layout::LayoutConfig::default()
    };
    let result = mdd_layout::layout(&graph, &config);
    let positions = result.positions;
    let edge_waypoints = result.edge_waypoints;

    // SVG dimensions: use actual table sizes (may differ from layout-reported sizes)
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for table in &diagram.tables {
        if let Some(&(x, y, _w, _h)) = positions.get(&table.name) {
            let (tw, th) = table_size(table);
            max_x = max_x.max(x + tw);
            max_y = max_y.max(y + th);
        }
    }
    // Also account for group bounding boxes from layout
    for (_, (x, y, w, h)) in &positions {
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let svg_width = max_x + PADDING;
    let svg_height = max_y + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: sans-serif; font-size: 13px; fill: {}; }}</style>",
        COLOR_DARK
    ));

    // Render groups and tables (back to front)
    render_elements_recursive(&mut svg, &diagram.top_level, &diagram.tables, &diagram.groups, &positions);

    // Build node bounds for edge routing
    let node_bounds: Vec<(String, f64, f64, f64, f64)> = diagram
        .tables
        .iter()
        .filter_map(|tbl| {
            positions.get(&tbl.name).map(|(x, y, w, h)| {
                (tbl.name.clone(), x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)
            })
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
        let from_name = &diagram.tables[rel.from].name;
        let to_name = &diagram.tables[rel.to].name;
        let from_pos = positions.get(from_name);
        let to_pos = positions.get(to_name);
        if from_pos.is_none() || to_pos.is_none() {
            continue;
        }

        let (fx, fy, fw, fh) = *from_pos.unwrap();
        let (tx, ty, tw, th) = *to_pos.unwrap();

        let cx1 = fx + fw / 2.0;
        let cy1 = fy + fh / 2.0;
        let cx2 = tx + tw / 2.0;
        let cy2 = ty + th / 2.0;

        // Self-referencing relation: draw a loop on the right side
        if rel.from == rel.to {
            let rx = fx + fw;
            let ry_top = fy + fh * 0.3;
            let ry_bot = fy + fh * 0.7;
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

        // Use virtual node waypoints if available, otherwise route around nodes
        let edge_key = format!("{}→{}", from_name, to_name);
        let route = if let Some(waypoints) = edge_waypoints.get(&edge_key) {
            let mut r = vec![(cx1, cy1)];
            r.extend(waypoints);
            r.push((cx2, cy2));
            r
        } else {
            mdd_layout::edge::route_around_nodes(cx1, cy1, cx2, cy2, from_name, to_name, &node_bounds, offset)
        };

        let start_target = if route.len() > 1 { route[1] } else { (cx2, cy2) };
        let end_target = if route.len() > 1 { route[route.len() - 2] } else { (cx1, cy1) };
        let (ax1, ay1) = mdd_layout::edge::clip_to_rect(cx1, cy1, start_target.0, start_target.1, fw / 2.0, fh / 2.0);
        let (ax2, ay2) = mdd_layout::edge::clip_to_rect(cx2, cy2, end_target.0, end_target.1, tw / 2.0, th / 2.0);

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
            mdd_layout::edge::build_smooth_path(&clipped_route)
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

fn render_elements_recursive(
    svg: &mut String,
    elements: &[Element],
    tables: &[Table],
    groups: &[Group],
    positions: &HashMap<String, (f64, f64, f64, f64)>,
) {
    for elem in elements {
        match elem {
            Element::GroupRef(gi) => {
                let g = &groups[*gi];
                if let Some(&(x, y, w, h)) = positions.get(&g.name) {
                    // Group background (dashed border)
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6,4\"/>",
                        x, y, w, h, COLOR_GROUP_FILL, COLOR_GROUP_STROKE
                    ));
                    // Group label
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\">{}</text>",
                        x + 8.0, y + GROUP_HEADER_H * 0.7, escape_xml(&g.name)
                    ));
                }
                // Recurse into children
                render_elements_recursive(svg, &g.children, tables, groups, positions);
            }
            Element::TableRef(ti) => {
                let table = &tables[*ti];
                if let Some(&(x, y, _w, _h)) = positions.get(&table.name) {
                    render_table(svg, x, y, table);
                }
            }
        }
    }
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

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-er - Render an entity-relationship diagram as SVG

Usage: mdd-er < input.er

Define tables/entities with columns (prefix \"* \" for primary keys, or suffix PK/FK),
then relate them with cardinality notation: 1--1, 1--*, *--1, *--*.
Use \"group\" to group related entities together.

Example:
  group \"Users\" {
    entity User {
      id PK
      name
      email
    }
    entity Profile {
      user_id FK
      bio
    }
  }
  entity Posts {
    * id
    user_id
    title
  }
  User 1--* Posts
  User 1--1 Profile
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
    fn parse_entity_keyword() {
        let input = "entity User {\n  id PK\n  name\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tables.len(), 1);
        assert_eq!(d.tables[0].name, "User");
        assert!(d.tables[0].columns[0].is_pk);
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
    fn parse_group() {
        let input = "group \"Users\" {\n  entity User {\n    id PK\n    name\n  }\n  entity Profile {\n    user_id FK\n    bio\n  }\n}\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].name, "Users");
        assert_eq!(d.groups[0].children.len(), 2);
        assert_eq!(d.tables.len(), 2);
    }

    #[test]
    fn parse_group_with_relations() {
        let input = "group \"Core\" {\n  entity User {\n    id PK\n    name\n  }\n  entity Post {\n    id PK\n    user_id FK\n  }\n}\nUser 1--* Post\n";
        let d = parse(input).unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.tables.len(), 2);
        assert_eq!(d.relations.len(), 1);
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
    fn render_with_groups_produces_svg() {
        let input = "group \"Test\" {\n  entity A {\n    id PK\n  }\n  entity B {\n    id PK\n  }\n}\nA 1--* B\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        // Should contain group rectangle
        assert!(svg.contains("stroke-dasharray"));
    }
}
