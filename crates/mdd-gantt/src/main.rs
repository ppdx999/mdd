use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Unit {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone)]
struct Task {
    name: String,
    start: Date,
    duration_days: u32,
    section: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Date {
    year: i32,
    month: u32,
    day: u32,
}

#[derive(Debug)]
struct Section {
    name: String,
}

#[derive(Debug)]
struct Diagram {
    unit: Unit,
    sections: Vec<Section>,
    tasks: Vec<Task>,
}

// ---------------------------------------------------------------------------
// Date arithmetic
// ---------------------------------------------------------------------------

impl Date {
    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    fn to_day_offset(&self) -> i64 {
        // Simple day count from a reference point (2000-01-01)
        let mut days: i64 = 0;
        for y in 2000..self.year {
            days += if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                366
            } else {
                365
            };
        }
        if self.year < 2000 {
            for y in self.year..2000 {
                days -= if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                    366
                } else {
                    365
                };
            }
        }
        for m in 1..self.month {
            days += Self::days_in_month(self.year, m) as i64;
        }
        days += self.day as i64 - 1;
        days
    }

    fn add_days(&self, n: u32) -> Date {
        let mut y = self.year;
        let mut m = self.month;
        let mut d = self.day + n;
        loop {
            let dim = Self::days_in_month(y, m);
            if d <= dim {
                break;
            }
            d -= dim;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        Date {
            year: y,
            month: m,
            day: d,
        }
    }

    fn format_short(&self) -> String {
        format!("{}/{}", self.month, self.day)
    }

    fn format_month(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    fn first_of_month(&self) -> Date {
        Date {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }

    fn next_month(&self) -> Date {
        if self.month == 12 {
            Date {
                year: self.year + 1,
                month: 1,
                day: 1,
            }
        } else {
            Date {
                year: self.year,
                month: self.month + 1,
                day: 1,
            }
        }
    }

    /// Monday-based: Mon=0, Sun=6
    fn weekday(&self) -> u32 {
        let offset = self.to_day_offset();
        // 2000-01-01 is Saturday (5)
        ((offset % 7 + 7 + 5) % 7) as u32
    }

}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse_date(s: &str) -> Result<Date, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date: {}", s));
    }
    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| format!("Invalid year: {}", parts[0]))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid month: {}", parts[1]))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid day: {}", parts[2]))?;
    Ok(Date { year, month, day })
}

fn parse_duration(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('d') {
        n.trim()
            .parse::<u32>()
            .map_err(|_| format!("Invalid duration: {}", s))
    } else if let Some(n) = s.strip_suffix('w') {
        n.trim()
            .parse::<u32>()
            .map(|v| v * 7)
            .map_err(|_| format!("Invalid duration: {}", s))
    } else if let Some(n) = s.strip_suffix('m') {
        n.trim()
            .parse::<u32>()
            .map(|v| v * 30)
            .map_err(|_| format!("Invalid duration: {}", s))
    } else {
        Err(format!(
            "Invalid duration '{}': use 'd', 'w', or 'm' suffix",
            s
        ))
    }
}

fn parse(input: &str) -> Result<Diagram, String> {
    let mut unit = Unit::Week;
    let mut sections: Vec<Section> = Vec::new();
    let mut tasks: Vec<Task> = Vec::new();
    let mut current_section: Option<usize> = None;
    let mut task_names: HashMap<String, usize> = HashMap::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(u) = line.strip_prefix("unit ") {
            unit = match u.trim() {
                "day" => Unit::Day,
                "week" => Unit::Week,
                "month" => Unit::Month,
                other => return Err(format!("Unknown unit: {}", other)),
            };
            continue;
        }

        if let Some(s) = line.strip_prefix("section ") {
            let name = s.trim().to_string();
            current_section = Some(sections.len());
            sections.push(Section { name });
            continue;
        }

        // Task line: "name : start, duration"
        if let Some((name_part, rest)) = line.split_once(':') {
            let name = name_part.trim().to_string();
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() < 2 {
                return Err(format!("Task needs at least start and duration: {}", line));
            }

            let start_str = parts[0].trim();
            let start = if let Some(after_name) = start_str.strip_prefix("after ") {
                let dep_name = after_name.trim();
                let dep_idx = task_names
                    .get(dep_name)
                    .ok_or_else(|| format!("Unknown task '{}' in after", dep_name))?;
                let dep = &tasks[*dep_idx];
                dep.start.add_days(dep.duration_days)
            } else {
                parse_date(start_str)?
            };

            let duration_days = parse_duration(parts[1])?;

            let idx = tasks.len();
            task_names.insert(name.clone(), idx);
            tasks.push(Task {
                name,
                start,
                duration_days,
                section: current_section,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", line));
    }

    Ok(Diagram {
        unit,
        sections,
        tasks,
    })
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FONT_SIZE: f64 = 12.0;
const SMALL_FONT_SIZE: f64 = 10.0;

const PADDING: f64 = 24.0;
const HEADER_HEIGHT: f64 = 28.0;
const ROW_HEIGHT: f64 = 36.0;
const BAR_HEIGHT: f64 = 18.0;
const LABEL_AREA_WIDTH: f64 = 200.0;
const SECTION_LABEL_WIDTH: f64 = 100.0;
const SECTION_GAP: f64 = 12.0;
const DAY_WIDTH: f64 = 30.0;
const WEEK_WIDTH: f64 = 40.0;
const MONTH_WIDTH: f64 = 60.0;

const COLOR_HEADER_TEXT: &str = "#888";
const COLOR_GRID: &str = "#ebebeb";
const COLOR_BAR: &str = "#5b9bd5";
const COLOR_SECTION_TEXT: &str = "#555";
const COLOR_TASK_TEXT: &str = "#444";

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

/// Count section transitions in `tasks[0..row]`.
/// A gap exists before row `i` when `tasks[i].section != tasks[i-1].section`.
fn section_gaps_before(tasks: &[Task], row: usize) -> usize {
    let mut gaps = 0;
    let end = row.min(tasks.len());
    for i in 1..end {
        if tasks[i].section != tasks[i - 1].section {
            gaps += 1;
        }
    }
    // Also check the transition at `row` itself (the gap is drawn above this row)
    if row > 0 && row < tasks.len() && tasks[row].section != tasks[row - 1].section {
        gaps += 1;
    }
    gaps
}

/// Y position of a task row, accounting for section gaps.
fn row_y(chart_y: f64, row: usize, tasks: &[Task]) -> f64 {
    let gaps = section_gaps_before(tasks, row);
    chart_y + row as f64 * ROW_HEIGHT + gaps as f64 * SECTION_GAP
}

fn render_svg(diagram: &Diagram) -> String {
    if diagram.tasks.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"40\"><text x=\"10\" y=\"25\" font-family=\"sans-serif\" font-size=\"13\">No tasks</text></svg>".to_string();
    }

    let has_sections = !diagram.sections.is_empty();

    // Compute date range
    let min_date = diagram.tasks.iter().map(|t| t.start).min().unwrap();
    let max_date = diagram
        .tasks
        .iter()
        .map(|t| t.start.add_days(t.duration_days))
        .max()
        .unwrap();

    // Build time grid
    let (grid_start, grid_end, col_width, grid_labels) =
        build_grid(diagram.unit, min_date, max_date);
    let grid_total_days = (grid_end.to_day_offset() - grid_start.to_day_offset()) as f64;
    let day_px = col_width
        / match diagram.unit {
            Unit::Day => 1.0,
            Unit::Week => 7.0,
            Unit::Month => 30.0,
        };

    let chart_width = grid_total_days * day_px;

    let label_offset = if has_sections {
        SECTION_LABEL_WIDTH + LABEL_AREA_WIDTH
    } else {
        LABEL_AREA_WIDTH
    };

    let chart_x = PADDING + label_offset;
    let chart_y = PADDING + HEADER_HEIGHT;
    let num_rows = diagram.tasks.len();
    let total_section_gaps = section_gaps_before(&diagram.tasks, num_rows);
    let chart_height =
        num_rows as f64 * ROW_HEIGHT + total_section_gaps as f64 * SECTION_GAP;

    let svg_width = chart_x + chart_width + PADDING;
    let svg_height = chart_y + chart_height + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        svg_width, svg_height, svg_width, svg_height
    );
    svg.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"white\"/><style>text {{ font-family: -apple-system, 'Segoe UI', sans-serif; font-size: {}px; }}</style>",
        FONT_SIZE
    ));

    // Background
    svg.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"#fff\"/>",
        svg_width, svg_height
    ));

    // Header labels (just text, no background box)
    let header_y = PADDING;
    let mut col_x = 0.0;
    for (label, width) in &grid_labels {
        let lx = chart_x + col_x + width / 2.0;
        let ly = header_y + HEADER_HEIGHT / 2.0 + SMALL_FONT_SIZE * 0.35;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"{}\" font-size=\"{}\">{}</text>",
            lx, ly, COLOR_HEADER_TEXT, SMALL_FONT_SIZE, escape_xml(label)
        ));
        col_x += width;
    }

    // Separator line below header
    let sep_y = header_y + HEADER_HEIGHT - 1.0;
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        chart_x, sep_y, chart_x + chart_width, sep_y, COLOR_GRID
    ));

    // Vertical grid lines (thin, subtle)
    col_x = 0.0;
    for (_, width) in &grid_labels {
        let lx = chart_x + col_x;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"0.5\" opacity=\"0.6\"/>",
            lx, sep_y, lx, chart_y + chart_height, COLOR_GRID
        ));
        col_x += width;
    }

    // Compute section spans for divider lines and labels
    struct SectionSpan {
        section_idx: usize,
        start_row: usize,
        end_row: usize, // exclusive
    }
    let mut section_spans: Vec<SectionSpan> = Vec::new();
    if has_sections {
        let mut cur_sec: Option<usize> = None;
        let mut start = 0;
        for (i, task) in diagram.tasks.iter().enumerate() {
            if task.section != cur_sec {
                if let Some(s) = cur_sec {
                    section_spans.push(SectionSpan {
                        section_idx: s,
                        start_row: start,
                        end_row: i,
                    });
                }
                cur_sec = task.section;
                start = i;
            }
        }
        if let Some(s) = cur_sec {
            section_spans.push(SectionSpan {
                section_idx: s,
                start_row: start,
                end_row: num_rows,
            });
        }
    }

    // Divider line between section labels and task names (with gaps between sections)
    if has_sections {
        let line_x = PADDING + SECTION_LABEL_WIDTH;
        for span in &section_spans {
            let y_top = row_y(chart_y, span.start_row, &diagram.tasks);
            let y_bot = row_y(chart_y, span.end_row - 1, &diagram.tasks) + ROW_HEIGHT;
            svg.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                line_x, y_top, line_x, y_bot, COLOR_GRID
            ));
        }
    }

    // Section labels
    for span in &section_spans {
        draw_section_label(
            &mut svg,
            &diagram.sections[span.section_idx].name,
            PADDING,
            row_y(chart_y, span.start_row, &diagram.tasks),
            span.end_row - span.start_row,
        );
    }

    // Task labels + bars
    for (i, task) in diagram.tasks.iter().enumerate() {
        let ry = row_y(chart_y, i, &diagram.tasks);

        // Task label
        let label_x = if has_sections {
            PADDING + SECTION_LABEL_WIDTH + 10.0
        } else {
            PADDING + 10.0
        };
        let label_y = ry + ROW_HEIGHT / 2.0 + FONT_SIZE * 0.35;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",
            label_x, label_y, COLOR_TASK_TEXT, escape_xml(&task.name)
        ));

        // Task bar
        let task_start_offset =
            (task.start.to_day_offset() - grid_start.to_day_offset()) as f64;
        let bar_x = chart_x + task_start_offset * day_px;
        let bar_w = task.duration_days as f64 * day_px;
        let bar_y = ry + (ROW_HEIGHT - BAR_HEIGHT) / 2.0;
        let bar_r = BAR_HEIGHT / 2.0;

        // Bar (pill shape)
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
            bar_x, bar_y, bar_w, BAR_HEIGHT, bar_r, COLOR_BAR
        ));
    }

    svg.push_str("</svg>");
    svg
}

fn draw_section_label(svg: &mut String, name: &str, x: f64, y: f64, row_count: usize) {
    let height = row_count as f64 * ROW_HEIGHT;
    let tx = x + SECTION_LABEL_WIDTH / 2.0;
    let ty = y + height / 2.0 + FONT_SIZE * 0.35;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-weight=\"600\" font-size=\"11\" fill=\"{}\">{}</text>",
        tx, ty, COLOR_SECTION_TEXT, escape_xml(name)
    ));
}

fn build_grid(unit: Unit, min_date: Date, max_date: Date) -> (Date, Date, f64, Vec<(String, f64)>) {
    match unit {
        Unit::Day => {
            // Pad 1 day on each side
            let start = min_date;
            let total = (max_date.to_day_offset() - start.to_day_offset()) as u32;
            let end = start.add_days(total);
            let mut labels = Vec::new();
            let mut d = start;
            while d < end {
                labels.push((d.format_short(), DAY_WIDTH));
                d = d.add_days(1);
            }
            (start, end, DAY_WIDTH, labels)
        }
        Unit::Week => {
            // Find the Monday on or before min_date
            let wd = min_date.weekday();
            let start = if wd == 0 {
                min_date
            } else {
                date_from_offset(min_date.to_day_offset() - wd as i64)
            };

            let end_offset = max_date.to_day_offset();
            let end_wd = max_date.weekday();
            let end = if end_wd == 0 {
                max_date.add_days(7)
            } else {
                date_from_offset(end_offset + (7 - end_wd as i64))
            };

            let mut labels = Vec::new();
            let mut d = start;
            while d < end {
                labels.push((d.format_short(), WEEK_WIDTH));
                d = d.add_days(7);
            }
            (start, end, WEEK_WIDTH, labels)
        }
        Unit::Month => {
            let start = min_date.first_of_month();
            let end = max_date.next_month();

            let mut labels = Vec::new();
            let mut d = start;
            while d < end {
                let next = d.next_month();
                let days_in = (next.to_day_offset() - d.to_day_offset()) as f64;
                let width = days_in * (MONTH_WIDTH / 30.0);
                labels.push((d.format_month(), width));
                d = next;
            }
            (start, end, MONTH_WIDTH, labels)
        }
    }
}

fn date_from_offset(offset: i64) -> Date {
    // Reconstruct a Date from day offset (relative to 2000-01-01)
    let mut remaining = offset;
    let mut year = 2000;

    if remaining >= 0 {
        loop {
            let days_in_year = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                366
            } else {
                365
            };
            if remaining < days_in_year {
                break;
            }
            remaining -= days_in_year;
            year += 1;
        }
    } else {
        loop {
            year -= 1;
            let days_in_year = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                366
            } else {
                365
            };
            remaining += days_in_year;
            if remaining >= 0 {
                break;
            }
        }
    }

    let mut month = 1u32;
    loop {
        let dim = Date::days_in_month(year, month) as i64;
        if remaining < dim {
            break;
        }
        remaining -= dim;
        month += 1;
    }

    Date {
        year,
        month,
        day: remaining as u32 + 1,
    }
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
            eprintln!("mdd-gantt: {}", e);
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
    fn parse_date_valid() {
        let d = parse_date("2025-01-15").unwrap();
        assert_eq!(d.year, 2025);
        assert_eq!(d.month, 1);
        assert_eq!(d.day, 15);
    }

    #[test]
    fn parse_duration_days() {
        assert_eq!(parse_duration("5d").unwrap(), 5);
    }

    #[test]
    fn parse_duration_weeks() {
        assert_eq!(parse_duration("2w").unwrap(), 14);
    }

    #[test]
    fn parse_duration_months() {
        assert_eq!(parse_duration("1m").unwrap(), 30);
    }

    #[test]
    fn parse_simple_diagram() {
        let input = "unit day\nsection Dev\n  Task1 : 2025-01-06, 5d\n";
        let d = parse(input).unwrap();
        assert_eq!(d.unit, Unit::Day);
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.tasks.len(), 1);
        assert_eq!(d.tasks[0].name, "Task1");
        assert_eq!(d.tasks[0].duration_days, 5);
    }

    #[test]
    fn parse_after_dependency() {
        let input = "Task1 : 2025-01-06, 5d\nTask2 : after Task1, 3d\n";
        let d = parse(input).unwrap();
        assert_eq!(d.tasks[1].start, Date { year: 2025, month: 1, day: 11 });
    }

    #[test]
    fn date_add_days() {
        let d = Date { year: 2025, month: 1, day: 30 };
        let d2 = d.add_days(5);
        assert_eq!(d2, Date { year: 2025, month: 2, day: 4 });
    }

    #[test]
    fn date_add_days_year_boundary() {
        let d = Date { year: 2025, month: 12, day: 28 };
        let d2 = d.add_days(7);
        assert_eq!(d2, Date { year: 2026, month: 1, day: 4 });
    }

    #[test]
    fn date_offset_roundtrip() {
        let d = Date { year: 2025, month: 6, day: 15 };
        let offset = d.to_day_offset();
        let d2 = date_from_offset(offset);
        assert_eq!(d, d2);
    }

    #[test]
    fn render_produces_svg() {
        let input = "unit day\nsection Dev\nTask1 : 2025-01-06, 5d\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Task1"));
    }

    #[test]
    fn render_empty_tasks() {
        let input = "";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.contains("No tasks"));
    }

    #[test]
    fn multiple_sections() {
        let input = "section A\n  T1 : 2025-01-06, 5d\nsection B\n  T2 : 2025-01-13, 3d\n";
        let d = parse(input).unwrap();
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.tasks[0].section, Some(0));
        assert_eq!(d.tasks[1].section, Some(1));
    }
}
