use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum OpKind {
    Create,
    Read,
    Update,
    Delete,
}

impl OpKind {
    fn label(&self) -> &'static str {
        match self {
            OpKind::Create => "C",
            OpKind::Read => "R",
            OpKind::Update => "U",
            OpKind::Delete => "D",
        }
    }

    fn colors(&self) -> (&'static str, &'static str) {
        match self {
            OpKind::Create => ("#e8f5e9", "#2e7d32"),
            OpKind::Read => ("#e3f2fd", "#1565c0"),
            OpKind::Update => ("#fff8e1", "#f57f17"),
            OpKind::Delete => ("#ffebee", "#c62828"),
        }
    }
}

#[derive(Debug)]
struct Column {
    name: String,
    col_type: String,
    is_pk: bool,
    is_fk: bool,
}

#[derive(Debug)]
struct Operation {
    kind: OpKind,
    path: String,
    description: String,
    columns: Vec<String>, // affected column names
}

#[derive(Debug)]
struct Lifecycle {
    table_name: String,
    columns: Vec<Column>,
    operations: Vec<Operation>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Lifecycle, String> {
    let mut table_name = String::new();
    let mut columns: Vec<Column> = Vec::new();
    let mut operations: Vec<Operation> = Vec::new();

    enum State {
        Top,
        InTable,
        InOp(OpKind, String, String), // kind, path, description
    }
    let mut state = State::Top;
    let mut op_columns: Vec<String> = Vec::new();

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }

        match &state {
            State::InTable => {
                if t == "}" {
                    state = State::Top;
                    continue;
                }
                columns.push(parse_table_column(t));
                continue;
            }
            State::InOp(kind, path, desc) => {
                if t == "}" {
                    operations.push(Operation {
                        kind: *kind,
                        path: path.clone(),
                        description: desc.clone(),
                        columns: op_columns.clone(),
                    });
                    op_columns.clear();
                    state = State::Top;
                    continue;
                }
                op_columns.push(t.to_string());
                continue;
            }
            State::Top => {}
        }

        // table Name {
        if t.starts_with("table ") {
            let rest = t.strip_prefix("table ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                table_name = name.trim().to_string();
                state = State::InTable;
                continue;
            }
            return Err(format!("Invalid table syntax: {}", t));
        }

        // CRUD operations
        let op_prefixes: &[(&str, OpKind)] = &[
            ("create ", OpKind::Create),
            ("read ", OpKind::Read),
            ("update ", OpKind::Update),
            ("delete ", OpKind::Delete),
        ];

        let mut matched = false;
        for (prefix, kind) in op_prefixes {
            if t.starts_with(prefix) {
                let rest = t.strip_prefix(prefix).unwrap();

                // With block: create path : "desc" { ... }
                if let Some(before_brace) = rest.strip_suffix(" {") {
                    let (path, desc) = if let Some((p, d)) = before_brace.split_once(" : ") {
                        (p.trim().to_string(), d.trim().trim_matches('"').to_string())
                    } else {
                        (before_brace.trim().to_string(), String::new())
                    };
                    state = State::InOp(*kind, path, desc);
                    op_columns.clear();
                    matched = true;
                    break;
                }

                // Single line (no columns, e.g. delete): delete path : "desc"
                let (path, desc) = if let Some((p, d)) = rest.split_once(" : ") {
                    (p.trim().to_string(), d.trim().trim_matches('"').to_string())
                } else {
                    (rest.trim().to_string(), String::new())
                };
                operations.push(Operation {
                    kind: *kind,
                    path,
                    description: desc,
                    columns: Vec::new(),
                });
                matched = true;
                break;
            }
        }
        if matched { continue; }

        return Err(format!("Unknown syntax: {}", t));
    }

    if table_name.is_empty() {
        return Err("Missing table definition".to_string());
    }

    Ok(Lifecycle { table_name, columns, operations })
}

fn parse_table_column(line: &str) -> Column {
    // "name : TYPE PK FK" or just "name"
    let segments: Vec<&str> = line.splitn(2, " : ").collect();
    if segments.len() == 2 {
        let name = segments[0].trim().to_string();
        let rest = segments[1].trim();
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let mut type_parts = Vec::new();
        let mut is_pk = false;
        let mut is_fk = false;
        for p in &parts {
            match *p {
                "PK" => is_pk = true,
                "FK" => is_fk = true,
                _ => type_parts.push(*p),
            }
        }
        Column { name, col_type: type_parts.join(" "), is_pk, is_fk }
    } else {
        Column { name: line.trim().to_string(), col_type: String::new(), is_pk: false, is_fk: false }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const MONO_CW: f64 = 7.8;
const FONT_SIZE: f64 = 13.0;
const LINE_HEIGHT: f64 = 18.0;
const PAD: f64 = 30.0;

// Table card
const TBL_HEADER_H: f64 = 32.0;
const TBL_H_PAD: f64 = 16.0;
const TBL_BODY_PAD: f64 = 8.0;

// Operation card
const OP_ROW_H: f64 = 24.0;
const OP_HEADER_H: f64 = 30.0;
const OP_H_PAD: f64 = 12.0;
const OP_GAP: f64 = 10.0;
const BADGE_W: f64 = 24.0;
const BADGE_H: f64 = 18.0;

const COLOR_DARK: &str = "#333";
const COLOR_TBL_HEADER_BG: &str = "#e8f5e9";
const COLOR_TBL_HEADER_TEXT: &str = "#2e7d32";
const COLOR_TBL_BG: &str = "#fff";
const COLOR_TBL_STROKE: &str = "#2e7d32";
const COLOR_OP_BG: &str = "#fafafa";
const COLOR_OP_STROKE: &str = "#ddd";
const COLOR_PK: &str = "#c8a415";
const COLOR_FK: &str = "#7b1fa2";
const COLOR_HIGHLIGHT: &str = "#fff9c4"; // highlight for affected columns

// ---------------------------------------------------------------------------
// Sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum()
}

fn mono_width(s: &str) -> f64 {
    s.len() as f64 * MONO_CW
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn op_card_size(op: &Operation) -> (f64, f64) {
    let path_w = mono_width(&op.path);
    let desc_w = if op.description.is_empty() { 0.0 } else { text_width(&op.description) };
    let col_w = op.columns.iter().map(|c| text_width(c)).fold(0.0_f64, f64::max);

    let header_w = BADGE_W + 8.0 + path_w
        + if desc_w > 0.0 { 12.0 + desc_w } else { 0.0 }
        + OP_H_PAD * 2.0;
    let body_w = col_w + OP_H_PAD * 2.0 + 20.0;
    let w = header_w.max(body_w).max(180.0);

    let body_h = if op.columns.is_empty() { 0.0 } else { op.columns.len() as f64 * OP_ROW_H + 8.0 };
    let h = OP_HEADER_H + body_h;
    (w, h)
}

fn render_svg(lc: &Lifecycle) -> String {
    // Table card dimensions
    let tbl_col_w = lc.columns.iter().map(|c| text_width(&c.name)).fold(0.0_f64, f64::max);
    let tbl_w = (text_width(&lc.table_name) + TBL_H_PAD * 2.0)
        .max(tbl_col_w + TBL_H_PAD * 2.0 + 30.0)
        .max(200.0);
    let tbl_body_h = TBL_BODY_PAD + lc.columns.len() as f64 * LINE_HEIGHT + TBL_BODY_PAD;
    let tbl_h = TBL_HEADER_H + tbl_body_h;

    // Compute op card sizes
    let op_sizes: Vec<(f64, f64)> = lc.operations.iter().map(|op| op_card_size(op)).collect();
    let max_op_dim = op_sizes.iter()
        .map(|(w, h)| w.max(*h))
        .fold(0.0_f64, f64::max);

    // Radial layout: table at center, ops around it
    let n_ops = lc.operations.len();
    let radius = (tbl_w.max(tbl_h) / 2.0 + max_op_dim / 2.0 + 80.0)
        .max(200.0)
        * (1.0 + (n_ops as f64 / 8.0).sqrt() * 0.3);

    // Compute op center positions (clockwise from top)
    let start_angle = -std::f64::consts::FRAC_PI_2; // start from top
    let op_centers: Vec<(f64, f64)> = (0..n_ops).map(|i| {
        let angle = start_angle + 2.0 * std::f64::consts::PI * i as f64 / n_ops as f64;
        (radius * angle.cos(), radius * angle.sin())
    }).collect();

    // Compute bounding box
    let mut min_x = -tbl_w / 2.0;
    let mut min_y = -tbl_h / 2.0;
    let mut max_x = tbl_w / 2.0;
    let mut max_y = tbl_h / 2.0;
    for (i, (cx, cy)) in op_centers.iter().enumerate() {
        let (ow, oh) = op_sizes[i];
        min_x = min_x.min(cx - ow / 2.0);
        min_y = min_y.min(cy - oh / 2.0);
        max_x = max_x.max(cx + ow / 2.0);
        max_y = max_y.max(cy + oh / 2.0);
    }

    let total_w = (max_x - min_x) + PAD * 2.0;
    let total_h = (max_y - min_y) + PAD * 2.0;
    let offset_x = PAD - min_x;
    let offset_y = PAD - min_y;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let tbl_cx = offset_x;
    let tbl_cy = offset_y;
    let tbl_x = tbl_cx - tbl_w / 2.0;
    let tbl_y = tbl_cy - tbl_h / 2.0;

    // Draw connection lines first (behind cards)
    for (i, op) in lc.operations.iter().enumerate() {
        let (ocx, ocy) = op_centers[i];
        let (ow, oh) = op_sizes[i];
        let ox = ocx + offset_x - ow / 2.0;
        let oy = ocy + offset_y - oh / 2.0;

        let op_conn_x = ocx + offset_x;
        let op_conn_y = ocy + offset_y;

        // Bezier from op center toward table center
        let dx = tbl_cx - op_conn_x;
        let dy = tbl_cy - op_conn_y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let ctrl_dist = len * 0.3;

        let (_, accent) = op.kind.colors();

        svg.push_str(&format!(
            "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" opacity=\"0.5\"/>",
            op_conn_x, op_conn_y,
            op_conn_x + dx / len * ctrl_dist, op_conn_y + dy / len * ctrl_dist,
            tbl_cx - dx / len * ctrl_dist, tbl_cy - dy / len * ctrl_dist,
            tbl_cx, tbl_cy,
            accent
        ));
    }

    // Render table card (center)
    render_table(&mut svg, tbl_x, tbl_y, tbl_w, tbl_h, lc);

    // Render operation cards (radial)
    for (i, op) in lc.operations.iter().enumerate() {
        let (ocx, ocy) = op_centers[i];
        let (ow, oh) = op_sizes[i];
        let ox = ocx + offset_x - ow / 2.0;
        let oy = ocy + offset_y - oh / 2.0;
        render_operation(&mut svg, ox, oy, ow, oh, op, &lc.columns);
    }

    svg.push_str("</svg>");
    svg
}

fn render_table(svg: &mut String, x: f64, y: f64, w: f64, h: f64, lc: &Lifecycle) {
    // Background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"none\"/>",
        x, y, w, h, COLOR_TBL_BG
    ));
    // Top/bottom lines
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y, x + w, y, COLOR_TBL_STROKE
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x, y + h, x + w, y + h, COLOR_TBL_STROKE
    ));
    // Header
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        x, y, w, TBL_HEADER_H, COLOR_TBL_HEADER_BG
    ));
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" stroke-dasharray=\"3,3\"/>",
        x, y + TBL_HEADER_H, x + w, y + TBL_HEADER_H, COLOR_TBL_STROKE
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        x + w / 2.0, y + TBL_HEADER_H * 0.7, COLOR_TBL_HEADER_TEXT, escape_xml(&lc.table_name)
    ));

    // Columns
    for (i, col) in lc.columns.iter().enumerate() {
        let cy = y + TBL_HEADER_H + TBL_BODY_PAD + (i as f64 + 0.75) * LINE_HEIGHT;
        let mut cx = x + TBL_H_PAD;

        if col.is_pk {
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"18\" height=\"11\" rx=\"2\" fill=\"#fff8e1\"/>", cx, cy - 8.0));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"7\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">PK</text>",
                cx + 9.0, cy - 1.0, COLOR_PK));
            cx += 22.0;
        }
        if col.is_fk {
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"18\" height=\"11\" rx=\"2\" fill=\"#f3e5f5\"/>", cx, cy - 8.0));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"7\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">FK</text>",
                cx + 9.0, cy - 1.0, COLOR_FK));
            cx += 22.0;
        }

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#555\">{}</text>",
            cx, cy, escape_xml(&col.name)
        ));
    }
}

fn render_operation(svg: &mut String, x: f64, y: f64, w: f64, h: f64, op: &Operation, _table_cols: &[Column]) {
    let (badge_bg, badge_fg) = op.kind.colors();

    // Card background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x, y, w, h, COLOR_OP_BG, COLOR_OP_STROKE
    ));

    // CRUD badge
    let bx = x + OP_H_PAD;
    let by = y + (OP_HEADER_H - BADGE_H) / 2.0;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        bx, by, BADGE_W, BADGE_H, badge_bg, badge_fg
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        bx + BADGE_W / 2.0, by + BADGE_H / 2.0 + 4.0, badge_fg, op.kind.label()
    ));

    // Path (monospace)
    let path_x = bx + BADGE_W + 8.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
        path_x, y + OP_HEADER_H / 2.0 + 4.0, COLOR_DARK, escape_xml(&op.path)
    ));

    // Description
    if !op.description.is_empty() {
        let desc_x = path_x + mono_width(&op.path) + 12.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#888\">{}</text>",
            desc_x, y + OP_HEADER_H / 2.0 + 4.0, escape_xml(&op.description)
        ));
    }

    // Separator
    if !op.columns.is_empty() {
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            x, y + OP_HEADER_H, x + w, y + OP_HEADER_H, COLOR_OP_STROKE
        ));
    }

    // Affected columns
    for (i, col) in op.columns.iter().enumerate() {
        let cy = y + OP_HEADER_H + 4.0 + (i as f64 + 0.5) * OP_ROW_H;
        // Bullet dot
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\"/>",
            x + OP_H_PAD + 4.0, cy, badge_fg
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"11\" fill=\"#555\">{}</text>",
            x + OP_H_PAD + 12.0, cy + 4.0, escape_xml(col)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-data-lifecycle - Render a data lifecycle diagram as SVG

Usage: mdd-data-lifecycle < input.txt

Define a table and the CRUD operations that affect it.

Syntax:
  table name {
    column_name : TYPE PK
    column_name : TYPE FK
  }

  create path : \"description\" {
    affected_column
  }

  read path : \"description\" {
    affected_column
  }

  update path : \"description\" {
    affected_column
  }

  delete path : \"description\"

Example:
  table users {
    id : BIGINT PK
    name : VARCHAR
    email : VARCHAR
  }

  create api/users : \"ユーザー作成\" {
    name
    email
  }

  read api/users/:id : \"ユーザー取得\" {
    name
    email
  }

  delete api/users/:id : \"ユーザー削除\"
";

fn main() {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprint!("{}", HELP);
        return;
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    let lc = match parse(&input) {
        Ok(l) => l,
        Err(e) => { eprintln!("mdd-data-lifecycle: {}", e); std::process::exit(1); }
    };

    print!("{}", render_svg(&lc));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "table users {\n  id : BIGINT PK\n  name\n}\ncreate api/users : \"create\" {\n  name\n}\n";
        let lc = parse(input).unwrap();
        assert_eq!(lc.table_name, "users");
        assert_eq!(lc.columns.len(), 2);
        assert!(lc.columns[0].is_pk);
        assert_eq!(lc.operations.len(), 1);
        assert_eq!(lc.operations[0].kind, OpKind::Create);
        assert_eq!(lc.operations[0].columns.len(), 1);
    }

    #[test]
    fn parse_delete_no_columns() {
        let input = "table t {\n  id : BIGINT PK\n}\ndelete api/t/:id : \"削除\"\n";
        let lc = parse(input).unwrap();
        assert_eq!(lc.operations[0].kind, OpKind::Delete);
        assert!(lc.operations[0].columns.is_empty());
    }

    #[test]
    fn parse_all_crud() {
        let input = "table t {\n  id : BIGINT PK\n}\ncreate a {\n  id\n}\nread b {\n  id\n}\nupdate c {\n  id\n}\ndelete d\n";
        let lc = parse(input).unwrap();
        assert_eq!(lc.operations.len(), 4);
    }

    #[test]
    fn render_produces_svg() {
        let input = "table t {\n  id : BIGINT PK\n}\ncreate a : \"test\" {\n  id\n}\n";
        let lc = parse(input).unwrap();
        let svg = render_svg(&lc);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
