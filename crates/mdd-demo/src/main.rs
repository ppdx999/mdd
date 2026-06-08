use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Demo {
    code: Vec<String>,
    label: String,
    output: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Demo, String> {
    let mut code: Vec<String> = Vec::new();
    let mut label = "mdd".to_string();
    let mut output: Vec<String> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    let mut section = "code"; // "code", "label", "output"

    while i < lines.len() {
        let t = lines[i].trim();

        if t == "---" {
            if section == "code" {
                section = "output";
            }
            i += 1;
            continue;
        }

        if t.starts_with("arrow ") {
            label = sq(t.strip_prefix("arrow ").unwrap().trim()).to_string();
            section = "output";
            i += 1;
            continue;
        }

        if section == "code" {
            // Preserve original indentation for code display
            code.push(lines[i].to_string());
        } else {
            output.push(lines[i].to_string());
        }

        i += 1;
    }

    // Remove leading/trailing empty lines
    while code.first().is_some_and(|l| l.trim().is_empty()) { code.remove(0); }
    while code.last().is_some_and(|l| l.trim().is_empty()) { code.pop(); }
    while output.first().is_some_and(|l| l.trim().is_empty()) { output.remove(0); }
    while output.last().is_some_and(|l| l.trim().is_empty()) { output.pop(); }

    if code.is_empty() && output.is_empty() {
        return Err("Need code and/or output content".to_string());
    }

    Ok(Demo { code, label, output })
}

fn sq(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const MONO_CW: f64 = 8.0;
const CODE_FONT_SIZE: f64 = 12.0;
const CODE_LINE_H: f64 = 18.0;
const CODE_H_PAD: f64 = 16.0;
const CODE_V_PAD: f64 = 14.0;
const CODE_RADIUS: f64 = 8.0;
const OUTPUT_FONT_SIZE: f64 = 12.0;
const OUTPUT_LINE_H: f64 = 18.0;
const OUTPUT_H_PAD: f64 = 16.0;
const OUTPUT_V_PAD: f64 = 14.0;
const OUTPUT_RADIUS: f64 = 8.0;
const ARROW_H: f64 = 60.0;
const ARROW_LABEL_SIZE: f64 = 14.0;
const PADDING: f64 = 24.0;
const MIN_W: f64 = 400.0;
const CODE_TITLE_H: f64 = 28.0;

const COLOR_CODE_BG: &str = "#1e1e1e";
const COLOR_CODE_TEXT: &str = "#d4d4d4";
const COLOR_CODE_TITLE_BG: &str = "#2d2d2d";
const COLOR_CODE_TITLE_TEXT: &str = "#999";
const COLOR_CODE_KEYWORD: &str = "#569cd6";
const COLOR_CODE_STRING: &str = "#ce9178";
const COLOR_CODE_COMMENT: &str = "#6a9955";
const COLOR_OUTPUT_BG: &str = "#f8fafb";
const COLOR_OUTPUT_BORDER: &str = "#d0d7de";
const COLOR_OUTPUT_TEXT: &str = "#333";
const COLOR_ARROW: &str = "#1565c0";
const COLOR_ARROW_LABEL: &str = "#1565c0";

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

fn tw(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CW } else { CJK }).sum()
}

fn mono_tw(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { MONO_CW } else { CJK }).sum()
}

fn ex(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// Simple syntax coloring: keywords, strings, comments
fn render_code_line(svg: &mut String, x: f64, y: f64, line: &str) {
    let trimmed = line.trim();

    // Comment
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
            x, y, CODE_FONT_SIZE, COLOR_CODE_COMMENT, ex(line)
        ));
        return;
    }

    // Try to find keyword at start
    let keywords = ["actor", "step", "stage", "level", "node", "link", "commit", "branch",
        "checkout", "merge", "title", "center", "spoke", "lane", "table", "column",
        "card", "group", "layer", "member", "slice", "axis", "data", "set", "overlap",
        "start", "end", "process", "decision", "edge", "state", "entity", "datastore",
        "strengths", "weaknesses", "opportunities", "threats", "before", "after",
        "option", "plan", "item", "metric", "msg", "post", "quote", "release", "repo"];

    let mut rendered = false;
    for kw in &keywords {
        if trimmed.starts_with(kw) && (trimmed.len() == kw.len() || !trimmed.as_bytes()[kw.len()].is_ascii_alphanumeric()) {
            let leading_spaces = line.len() - line.trim_start().len();
            let prefix = &line[..leading_spaces];
            let rest = &trimmed[kw.len()..];

            // Render: spaces + keyword(blue) + rest(check for strings)
            let mut parts = String::new();
            parts.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", COLOR_CODE_TEXT, ex(prefix)));
            parts.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", COLOR_CODE_KEYWORD, ex(kw)));

            // Color strings in rest
            let colored_rest = color_strings(rest);
            parts.push_str(&colored_rest);

            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\">{}</text>",
                x, y, CODE_FONT_SIZE, parts
            ));
            rendered = true;
            break;
        }
    }

    if !rendered {
        let colored = color_strings(line);
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\">{}</text>",
            x, y, CODE_FONT_SIZE, colored
        ));
    }
}

fn color_strings(s: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut buf = String::new();

    for ch in s.chars() {
        if ch == '"' {
            if in_string {
                buf.push('"');
                result.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", COLOR_CODE_STRING, ex(&buf)));
                buf.clear();
                in_string = false;
            } else {
                if !buf.is_empty() {
                    result.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", COLOR_CODE_TEXT, ex(&buf)));
                    buf.clear();
                }
                buf.push('"');
                in_string = true;
            }
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        let color = if in_string { COLOR_CODE_STRING } else { COLOR_CODE_TEXT };
        result.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", color, ex(&buf)));
    }
    result
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(demo: &Demo) -> String {
    // Compute widths
    let code_max_w = demo.code.iter().map(|l| mono_tw(l)).fold(0.0_f64, f64::max);
    let output_max_w = demo.output.iter().map(|l| mono_tw(l)).fold(0.0_f64, f64::max);
    let content_w = (code_max_w + CODE_H_PAD * 2.0)
        .max(output_max_w + OUTPUT_H_PAD * 2.0)
        .max(MIN_W);

    let code_h = if demo.code.is_empty() { 0.0 } else {
        CODE_TITLE_H + CODE_V_PAD + demo.code.len() as f64 * CODE_LINE_H + CODE_V_PAD
    };
    let output_h = if demo.output.is_empty() { 0.0 } else {
        OUTPUT_V_PAD + demo.output.len() as f64 * OUTPUT_LINE_H + OUTPUT_V_PAD
    };
    let arrow_section = if !demo.code.is_empty() && !demo.output.is_empty() { ARROW_H } else { 0.0 };

    let total_w = PADDING * 2.0 + content_w;
    let total_h = PADDING * 2.0 + code_h + arrow_section + output_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let cx = PADDING;
    let mut y = PADDING;

    // Code block (dark editor style)
    if !demo.code.is_empty() {
        // Background
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
            cx, y, content_w, code_h, CODE_RADIUS, COLOR_CODE_BG
        ));
        // Title bar
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
            cx, y, content_w, CODE_TITLE_H, CODE_RADIUS, COLOR_CODE_TITLE_BG
        ));
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"10\" fill=\"{}\"/>",
            cx, y + CODE_TITLE_H - 10.0, content_w, COLOR_CODE_TITLE_BG
        ));
        // Traffic lights
        for (j, color) in ["#ff5f57","#febc2e","#28c840"].iter().enumerate() {
            svg.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>",
                cx + 16.0 + j as f64 * 16.0, y + CODE_TITLE_H / 2.0, color
            ));
        }
        // Title text
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">input.md</text>",
            cx + content_w / 2.0, y + CODE_TITLE_H / 2.0 + 4.0, COLOR_CODE_TITLE_TEXT
        ));

        let mut code_y = y + CODE_TITLE_H + CODE_V_PAD;
        for line in &demo.code {
            render_code_line(&mut svg, cx + CODE_H_PAD, code_y + CODE_FONT_SIZE * 0.85, line);
            code_y += CODE_LINE_H;
        }
        y += code_h;
    }

    // Arrow
    if !demo.code.is_empty() && !demo.output.is_empty() {
        let arrow_cx = cx + content_w / 2.0;
        let arrow_top = y + 10.0;
        let arrow_bot = y + ARROW_H - 10.0;

        // Arrow line
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
            arrow_cx, arrow_top, arrow_cx, arrow_bot - 8.0, COLOR_ARROW
        ));
        // Arrowhead
        svg.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            arrow_cx, arrow_bot,
            arrow_cx - 6.0, arrow_bot - 10.0,
            arrow_cx + 6.0, arrow_bot - 10.0,
            COLOR_ARROW
        ));
        // Label
        let label_w = tw(&demo.label) * (ARROW_LABEL_SIZE / 13.0) + 16.0;
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"22\" rx=\"4\" fill=\"{}\" opacity=\"0.1\"/>",
            arrow_cx + 12.0, y + ARROW_H / 2.0 - 11.0, label_w, COLOR_ARROW
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"{}\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            arrow_cx + 20.0, y + ARROW_H / 2.0 + 5.0, ARROW_LABEL_SIZE, COLOR_ARROW_LABEL, ex(&demo.label)
        ));

        y += ARROW_H;
    }

    // Output block (light card style)
    if !demo.output.is_empty() {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            cx, y, content_w, output_h, OUTPUT_RADIUS, COLOR_OUTPUT_BG, COLOR_OUTPUT_BORDER
        ));

        let mut out_y = y + OUTPUT_V_PAD;
        for line in &demo.output {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
                cx + OUTPUT_H_PAD, out_y + OUTPUT_FONT_SIZE * 0.85, OUTPUT_FONT_SIZE, COLOR_OUTPUT_TEXT, ex(line)
            ));
            out_y += OUTPUT_LINE_H;
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(d) => print!("{}", render_svg(&d)),
        Err(e) => { eprintln!("mdd-demo: {}", e); std::process::exit(1); }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "actor Alice\nactor Bob\narrow mdd\nAlice --- Bob\n";
        let d = parse(input).unwrap();
        assert_eq!(d.code.len(), 2);
        assert_eq!(d.label, "mdd");
        assert_eq!(d.output.len(), 1);
    }

    #[test]
    fn parse_with_separator() {
        let input = "step A\nstep B\n---\nA -> B\n";
        let d = parse(input).unwrap();
        assert_eq!(d.code.len(), 2);
        assert_eq!(d.output.len(), 1);
    }

    #[test]
    fn render_output() {
        let input = "actor A\narrow mdd\nresult\n";
        let d = parse(input).unwrap();
        let svg = render_svg(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("mdd"));
    }
}
