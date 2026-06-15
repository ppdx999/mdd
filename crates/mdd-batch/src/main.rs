use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Cron {
    minute: String,
    hour: String,
    dom: String, // day of month
    month: String,
    dow: String, // day of week
}

impl Cron {
    fn to_human(&self) -> String {
        fn dow_name(s: &str) -> &str {
            match s {
                "0" | "7" => "日",
                "1" => "月",
                "2" => "火",
                "3" => "水",
                "4" => "木",
                "5" => "金",
                "6" => "土",
                _ => s,
            }
        }

        // Every N minutes: */N * * * *
        if self.minute.starts_with("*/") && self.hour == "*" && self.dom == "*" && self.month == "*" && self.dow == "*" {
            let n = &self.minute[2..];
            return format!("{}分毎", n);
        }

        // Every N hours: 0 */N * * *
        if self.minute == "0" && self.hour.starts_with("*/") && self.dom == "*" && self.month == "*" && self.dow == "*" {
            let n = &self.hour[2..];
            return format!("{}時間毎", n);
        }

        // Hourly: 0 * * * * or M * * * *
        if self.hour == "*" && self.dom == "*" && self.month == "*" && self.dow == "*" {
            if self.minute == "0" {
                return "毎時 0分".to_string();
            }
            return format!("毎時 {}分", self.minute);
        }

        // Weekly: M H * * D
        if self.dom == "*" && self.month == "*" && self.dow != "*" {
            let day = dow_name(&self.dow);
            return format!("毎週{} {}:{:0>2}", day, self.hour, self.minute);
        }

        // Monthly: M H D * *
        if self.dom != "*" && self.month == "*" && self.dow == "*" {
            return format!("毎月{}日 {}:{:0>2}", self.dom, self.hour, self.minute);
        }

        // Daily: M H * * *
        if self.dom == "*" && self.month == "*" && self.dow == "*" {
            return format!("毎日 {}:{:0>2}", self.hour, self.minute);
        }

        // Yearly: M H D Mo *
        if self.month != "*" && self.dow == "*" {
            return format!("{}月{}日 {}:{:0>2}", self.month, self.dom, self.hour, self.minute);
        }

        // Fallback: raw expression
        format!("{} {} {} {} {}", self.minute, self.hour, self.dom, self.month, self.dow)
    }

    fn color(&self) -> (&'static str, &'static str) {
        // Color based on frequency
        if self.minute.starts_with("*/") && self.hour == "*" {
            return ("#ffebee", "#c62828"); // sub-hourly: red
        }
        if self.hour == "*" || self.hour.starts_with("*/") {
            return ("#e8f5e9", "#2e7d32"); // hourly: green
        }
        if self.dow != "*" {
            return ("#f3e5f5", "#7b1fa2"); // weekly: purple
        }
        if self.dom != "*" && self.month != "*" {
            return ("#e0f2f1", "#00695c"); // yearly: teal
        }
        if self.dom != "*" {
            return ("#fff8e1", "#f57f17"); // monthly: amber
        }
        ("#e3f2fd", "#1565c0") // daily: blue
    }
}

#[derive(Debug)]
struct Job {
    cron: Cron,
    name: String,
    description: String,
}

#[derive(Debug)]
struct Group {
    label: String,
    jobs: Vec<Job>,
}

#[derive(Debug)]
enum Item {
    Job(Job),
    Group(Group),
}

#[derive(Debug)]
struct BatchSpec {
    items: Vec<Item>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<BatchSpec, String> {
    let mut items: Vec<Item> = Vec::new();
    let mut current_group: Option<(String, Vec<Job>)> = None;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        if t.starts_with("group ") {
            let rest = t.strip_prefix("group ").unwrap();
            if let Some(name) = rest.strip_suffix(" {") {
                let label = strip_quotes(name.trim()).to_string();
                current_group = Some((label, Vec::new()));
                continue;
            }
            return Err(format!("Invalid group syntax: {}", t));
        }

        if t == "}" {
            if let Some((label, jobs)) = current_group.take() {
                items.push(Item::Group(Group { label, jobs }));
                continue;
            }
            return Err("Unexpected }".to_string());
        }

        if let Some(job) = parse_job(t)? {
            if let Some((_, ref mut jobs)) = current_group {
                jobs.push(job);
            } else {
                items.push(Item::Job(job));
            }
            continue;
        }

        return Err(format!("Unknown syntax: {}", t));
    }

    if current_group.is_some() {
        return Err("Unclosed group block".to_string());
    }
    if items.is_empty() {
        return Err("At least 1 job required".to_string());
    }

    Ok(BatchSpec { items })
}

/// Parse a cron job line: "M H DOM MON DOW name : description"
fn parse_job(line: &str) -> Result<Option<Job>, String> {
    let parts: Vec<&str> = line.splitn(6, ' ').collect();
    if parts.len() < 6 {
        return Ok(None);
    }

    // Validate that first 5 parts look like cron fields
    let cron_chars = |s: &str| s.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '/' || c == ',' || c == '-');
    if !parts[..5].iter().all(|p| cron_chars(p)) {
        return Ok(None);
    }

    let cron = Cron {
        minute: parts[0].to_string(),
        hour: parts[1].to_string(),
        dom: parts[2].to_string(),
        month: parts[3].to_string(),
        dow: parts[4].to_string(),
    };

    let rest = parts[5].trim();
    let (name, description) = if let Some((n, d)) = rest.split_once(" : ") {
        (n.trim().to_string(), strip_quotes(d.trim()).to_string())
    } else {
        (rest.to_string(), String::new())
    };

    Ok(Some(Job { cron, name, description }))
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;

const ROW_H: f64 = 36.0;
const ROW_GAP: f64 = 4.0;
const BADGE_H: f64 = 22.0;
const BADGE_MARGIN: f64 = 12.0;
const NAME_MARGIN: f64 = 12.0;
const DESC_MARGIN: f64 = 16.0;
const PAD: f64 = 24.0;
const GROUP_HEADER_H: f64 = 32.0;
const GROUP_GAP: f64 = 16.0;
const GROUP_PAD: f64 = 12.0;
const COL_GAP: f64 = 20.0;
const NUM_COLS: usize = 2;

const COLOR_DARK: &str = "#333";
const COLOR_NAME: &str = "#333";
const COLOR_DESC: &str = "#666";
const COLOR_GROUP_BG: &str = "#fafafa";
const COLOR_GROUP_STROKE: &str = "#e0e0e0";
const COLOR_GROUP_TEXT: &str = "#555";
const COLOR_ROW_BG: &str = "#ffffff";
const COLOR_ROW_STROKE: &str = "#eee";

// ---------------------------------------------------------------------------
// Sizing
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CW } else { CJK })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn badge_width(label: &str) -> f64 {
    text_width(label) + 16.0
}

fn max_badge_width(spec: &BatchSpec) -> f64 {
    let mut max_w: f64 = 0.0;
    for item in &spec.items {
        let jobs = match item {
            Item::Job(j) => std::slice::from_ref(j),
            Item::Group(g) => &g.jobs,
        };
        for j in jobs {
            max_w = max_w.max(badge_width(&j.cron.to_human()));
        }
    }
    max_w
}

fn item_content_width(item: &Item, max_badge_w: f64) -> f64 {
    let jobs = match item {
        Item::Job(j) => std::slice::from_ref(j),
        Item::Group(g) => &g.jobs,
    };

    let mut max_name: f64 = 0.0;
    let mut max_desc: f64 = 0.0;
    for j in jobs {
        max_name = max_name.max(text_width(&j.name));
        max_desc = max_desc.max(if j.description.is_empty() {
            0.0
        } else {
            text_width(&j.description)
        });
    }

    let label_w = match item {
        Item::Group(g) => text_width(&g.label) + 24.0,
        _ => 0.0,
    };

    let row_w = BADGE_MARGIN + max_badge_w + NAME_MARGIN + max_name
        + if max_desc > 0.0 { DESC_MARGIN + max_desc } else { 0.0 }
        + BADGE_MARGIN;

    row_w.max(label_w)
}

fn item_height(item: &Item) -> f64 {
    match item {
        Item::Job(_) => ROW_H,
        Item::Group(g) => {
            let rows_h = g.jobs.len() as f64 * ROW_H
                + (g.jobs.len().saturating_sub(1)) as f64 * ROW_GAP;
            GROUP_HEADER_H + GROUP_PAD * 2.0 + rows_h
        }
    }
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(spec: &BatchSpec) -> String {
    let mbw = max_badge_width(spec);
    let item_widths: Vec<f64> = spec.items.iter().map(|i| item_content_width(i, mbw)).collect();
    let item_heights: Vec<f64> = spec.items.iter().map(|i| item_height(i)).collect();

    let n = spec.items.len();
    let use_cols = if n <= 1 { 1 } else { NUM_COLS.min(n) };

    let mut col_items: Vec<Vec<usize>> = vec![vec![]; use_cols];
    let mut col_heights: Vec<f64> = vec![0.0; use_cols];

    for i in 0..n {
        let min_col = col_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        if !col_items[min_col].is_empty() {
            col_heights[min_col] += GROUP_GAP;
        }
        col_items[min_col].push(i);
        col_heights[min_col] += item_heights[i];
    }

    let col_widths: Vec<f64> = col_items
        .iter()
        .map(|items| {
            items
                .iter()
                .map(|&i| item_widths[i])
                .fold(0.0_f64, f64::max)
        })
        .collect();

    let total_w = PAD * 2.0
        + col_widths.iter().sum::<f64>()
        + (use_cols.saturating_sub(1)) as f64 * COL_GAP;
    let max_col_h = col_heights.iter().copied().fold(0.0_f64, f64::max);
    let total_h = PAD * 2.0 + max_col_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(&format!(
        "<style>text {{ font-family: sans-serif; font-size: {}px; fill: {}; }}</style>",
        FONT_SIZE, COLOR_DARK
    ));

    let mut col_x = PAD;
    for (ci, items) in col_items.iter().enumerate() {
        let cw = col_widths[ci];
        let mut cur_y = PAD;

        for (idx, &item_idx) in items.iter().enumerate() {
            if idx > 0 {
                cur_y += GROUP_GAP;
            }

            let item = &spec.items[item_idx];
            match item {
                Item::Job(j) => {
                    render_job_row(&mut svg, j, col_x, cur_y, cw, mbw);
                    cur_y += ROW_H;
                }
                Item::Group(g) => {
                    let h = item_heights[item_idx];
                    render_group(&mut svg, g, col_x, cur_y, cw, h, mbw);
                    cur_y += h;
                }
            }
        }

        col_x += cw + COL_GAP;
    }

    svg.push_str("</svg>");
    svg
}

fn render_group(svg: &mut String, g: &Group, x: f64, y: f64, width: f64, height: f64, mbw: f64) {
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x, y, width, height, COLOR_GROUP_BG, COLOR_GROUP_STROKE
    ));

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\" \
         fill=\"{}\">{}</text>",
        x + 12.0,
        y + GROUP_HEADER_H * 0.7,
        COLOR_GROUP_TEXT,
        escape_xml(&g.label)
    ));

    let mut row_y = y + GROUP_HEADER_H + GROUP_PAD;
    for j in &g.jobs {
        render_job_row(svg, j, x, row_y, width, mbw);
        row_y += ROW_H + ROW_GAP;
    }
}

fn render_job_row(svg: &mut String, job: &Job, x: f64, y: f64, width: f64, mbw: f64) {
    let human = job.cron.to_human();
    let (badge_bg, badge_fg) = job.cron.color();
    let bw = badge_width(&human);

    // Row background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
        x + 4.0, y, width - 8.0, ROW_H, COLOR_ROW_BG, COLOR_ROW_STROKE
    ));

    // Schedule badge
    let badge_x = x + BADGE_MARGIN;
    let badge_y = y + (ROW_H - BADGE_H) / 2.0;
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" \
         fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        badge_x, badge_y, bw, BADGE_H, badge_bg, badge_fg
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" \
         font-weight=\"bold\" fill=\"{}\">{}</text>",
        badge_x + bw / 2.0,
        badge_y + BADGE_H / 2.0 + 4.0,
        badge_fg,
        escape_xml(&human)
    ));

    // Job name
    let name_x = x + BADGE_MARGIN + mbw + NAME_MARGIN;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
        name_x,
        y + ROW_H / 2.0 + FONT_SIZE * 0.35,
        COLOR_NAME,
        escape_xml(&job.name)
    ));

    // Description
    if !job.description.is_empty() {
        let desc_x = name_x + text_width(&job.name) + DESC_MARGIN;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">{}</text>",
            desc_x,
            y + ROW_H / 2.0 + 4.0,
            COLOR_DESC,
            escape_xml(&job.description)
        ));
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const HELP: &str = "\
mdd-batch - Render a batch job reference as SVG

Usage: mdd-batch < input.txt

Define batch jobs with cron expressions, job name, and optional description.
The cron expression is automatically converted to a human-readable label.

Syntax:
  M H DOM MON DOW  job-name : \"description\"

  0 3 * * *   backup-db    : \"毎日3時にDBバックアップ\"
  */10 * * * * health-check : \"10分毎のヘルスチェック\"
  0 0 1 * *   monthly-calc : \"月次集計\"

Group jobs by category:
  group \"Category\" {
    0 * * * *  job-a : \"description\"
    0 3 * * *  job-b : \"description\"
  }
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

    let spec = match parse(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd-batch: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&spec));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let input = "0 3 * * * backup-db : \"DBバックアップ\"\n";
        let s = parse(input).unwrap();
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            Item::Job(j) => {
                assert_eq!(j.name, "backup-db");
                assert_eq!(j.description, "DBバックアップ");
                assert_eq!(j.cron.minute, "0");
                assert_eq!(j.cron.hour, "3");
            }
            _ => panic!("Expected Job"),
        }
    }

    #[test]
    fn parse_no_description() {
        let input = "0 * * * * health-check\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Job(j) => {
                assert_eq!(j.name, "health-check");
                assert!(j.description.is_empty());
            }
            _ => panic!("Expected Job"),
        }
    }

    #[test]
    fn parse_group() {
        let input = "group \"ETL\" {\n  0 3 * * * extract\n  0 4 * * * load\n}\n";
        let s = parse(input).unwrap();
        match &s.items[0] {
            Item::Group(g) => {
                assert_eq!(g.label, "ETL");
                assert_eq!(g.jobs.len(), 2);
            }
            _ => panic!("Expected Group"),
        }
    }

    #[test]
    fn parse_unclosed_group() {
        assert!(parse("group \"X\" {\n  0 0 * * * x\n").is_err());
    }

    #[test]
    fn cron_to_human_daily() {
        let c = Cron { minute: "0".into(), hour: "3".into(), dom: "*".into(), month: "*".into(), dow: "*".into() };
        assert_eq!(c.to_human(), "毎日 3:00");
    }

    #[test]
    fn cron_to_human_hourly() {
        let c = Cron { minute: "0".into(), hour: "*".into(), dom: "*".into(), month: "*".into(), dow: "*".into() };
        assert_eq!(c.to_human(), "毎時 0分");
    }

    #[test]
    fn cron_to_human_every_n_min() {
        let c = Cron { minute: "*/10".into(), hour: "*".into(), dom: "*".into(), month: "*".into(), dow: "*".into() };
        assert_eq!(c.to_human(), "10分毎");
    }

    #[test]
    fn cron_to_human_weekly() {
        let c = Cron { minute: "0".into(), hour: "9".into(), dom: "*".into(), month: "*".into(), dow: "1".into() };
        assert_eq!(c.to_human(), "毎週月 9:00");
    }

    #[test]
    fn cron_to_human_monthly() {
        let c = Cron { minute: "0".into(), hour: "0".into(), dom: "1".into(), month: "*".into(), dow: "*".into() };
        assert_eq!(c.to_human(), "毎月1日 0:00");
    }

    #[test]
    fn render_produces_svg() {
        let input = "0 3 * * * test : \"テスト\"\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("毎日"));
    }

    #[test]
    fn render_grouped() {
        let input = "group \"ETL\" {\n  0 3 * * * extract : \"抽出\"\n  0 4 * * * load : \"ロード\"\n}\n";
        let s = parse(input).unwrap();
        let svg = render_svg(&s);
        assert!(svg.contains("ETL"));
        assert!(svg.contains("毎日"));
    }
}
