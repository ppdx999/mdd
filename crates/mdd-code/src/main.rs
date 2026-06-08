use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CodeBlock {
    title: Option<String>,
    lang: Option<String>,
    lines: Vec<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<CodeBlock, String> {
    let mut title = None;
    let mut lang = None;
    let mut lines: Vec<String> = Vec::new();
    let mut in_body = false;

    for line in input.lines() {
        let t = line.trim();

        if !in_body {
            if t.starts_with("title ") {
                title = Some(sq(t.strip_prefix("title ").unwrap().trim()).to_string());
                continue;
            }
            if t.starts_with("lang ") {
                lang = Some(t.strip_prefix("lang ").unwrap().trim().to_string());
                continue;
            }
            if t == "---" {
                in_body = true;
                continue;
            }
        }

        // Everything after metadata (or all lines if no ---) is code
        // Preserve original indentation
        if in_body || (!t.starts_with("title ") && !t.starts_with("lang ")) {
            in_body = true;
            lines.push(line.to_string());
        }
    }

    // Remove leading/trailing empty lines
    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return Err("No code content".to_string());
    }

    Ok(CodeBlock { title, lang, lines })
}

fn sq(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MONO_CW: f64 = 7.8;
const CJK_W: f64 = 14.0;
const FONT_SIZE: f64 = 13.0;
const LINE_H: f64 = 20.0;
const H_PAD: f64 = 16.0;
const V_PAD: f64 = 14.0;
const RADIUS: f64 = 8.0;
const LINE_NUM_W: f64 = 36.0;
const MIN_W: f64 = 500.0;

const COLOR_BG: &str = "#1e1e1e";
const COLOR_TEXT: &str = "#d4d4d4";
const COLOR_LINE_NUM: &str = "#555";
const COLOR_LINE_BORDER: &str = "#333";
const COLOR_KEYWORD: &str = "#569cd6";
const COLOR_STRING: &str = "#ce9178";
const COLOR_COMMENT: &str = "#6a9955";

// Keywords for syntax highlighting
const KEYWORDS: &[&str] = &[
    "actor", "step", "stage", "level", "node", "link", "commit", "branch",
    "checkout", "merge", "title", "center", "spoke", "lane", "table", "column",
    "card", "group", "layer", "member", "slice", "axis", "data", "set", "overlap",
    "start", "end", "process", "decision", "edge", "state", "entity", "datastore",
    "strengths", "weaknesses", "opportunities", "threats", "before", "after",
    "option", "plan", "item", "metric", "msg", "post", "quote", "release", "repo",
    "usecase", "package", "color", "type", "bar", "subtotal",
    "fn", "let", "mut", "const", "pub", "struct", "enum", "impl", "use", "mod",
    "if", "else", "for", "while", "match", "return", "true", "false",
    "import", "from", "export", "class", "function", "var", "def", "self",
];

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

fn mono_tw(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { MONO_CW } else { CJK_W })
        .sum()
}

fn ex(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Syntax coloring
// ---------------------------------------------------------------------------

fn render_code_line(svg: &mut String, x: f64, y: f64, line: &str) {
    let trimmed = line.trim();

    // Comment lines
    if trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.starts_with("--") {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
            x, y, FONT_SIZE, COLOR_COMMENT, ex(line)
        ));
        return;
    }

    // Try to color keyword at start of trimmed line
    let leading = line.len() - line.trim_start().len();
    let prefix = &line[..leading];

    for kw in KEYWORDS {
        if trimmed.starts_with(kw)
            && (trimmed.len() == kw.len()
                || !trimmed.as_bytes()[kw.len()].is_ascii_alphanumeric())
        {
            let rest = &trimmed[kw.len()..];
            let colored_rest = color_strings(rest);
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\">\
                <tspan fill=\"{}\">{}</tspan>\
                <tspan fill=\"{}\">{}</tspan>\
                {}</text>",
                x, y, FONT_SIZE,
                COLOR_TEXT, ex(prefix),
                COLOR_KEYWORD, ex(kw),
                colored_rest
            ));
            return;
        }
    }

    // No keyword match — color strings only
    let colored = color_strings(line);
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\">{}</text>",
        x, y, FONT_SIZE, colored
    ));
}

fn color_strings(s: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut buf = String::new();

    for ch in s.chars() {
        if ch == '"' {
            if in_string {
                buf.push('"');
                result.push_str(&format!(
                    "<tspan fill=\"{}\">{}</tspan>",
                    COLOR_STRING,
                    ex(&buf)
                ));
                buf.clear();
                in_string = false;
            } else {
                if !buf.is_empty() {
                    result.push_str(&format!(
                        "<tspan fill=\"{}\">{}</tspan>",
                        COLOR_TEXT,
                        ex(&buf)
                    ));
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
        let color = if in_string { COLOR_STRING } else { COLOR_TEXT };
        result.push_str(&format!("<tspan fill=\"{}\">{}</tspan>", color, ex(&buf)));
    }
    result
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(block: &CodeBlock) -> String {
    let _ = (&block.title, &block.lang); // metadata reserved for future use
    let n = block.lines.len();
    let max_line_w = block
        .lines
        .iter()
        .map(|l| mono_tw(l))
        .fold(0.0_f64, f64::max);

    let content_w = (LINE_NUM_W + H_PAD + max_line_w + H_PAD).max(MIN_W);
    let code_h = V_PAD + n as f64 * LINE_H + V_PAD;
    let total_w = content_w;
    let total_h = code_h;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );

    // Background
    svg.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"/>",
        total_w, total_h, RADIUS, COLOR_BG
    ));

    // Line number separator
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        LINE_NUM_W, 0.0, LINE_NUM_W, code_h, COLOR_LINE_BORDER
    ));

    // Code lines
    let mut cy = V_PAD;
    for (i, line) in block.lines.iter().enumerate() {
        let text_y = cy + FONT_SIZE * 0.85;

        // Line number
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"'SF Mono','Menlo','Consolas',monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
            LINE_NUM_W - 8.0,
            text_y,
            FONT_SIZE,
            COLOR_LINE_NUM,
            i + 1
        ));

        // Code content with syntax coloring
        render_code_line(&mut svg, LINE_NUM_W + H_PAD, text_y, line);

        cy += LINE_H;
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    match parse(&input) {
        Ok(block) => print!("{}", render_svg(&block)),
        Err(e) => {
            eprintln!("mdd-code: {}", e);
            std::process::exit(1);
        }
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
        let input = "title \"example\"\n---\nactor Alice\nactor Bob\n";
        let b = parse(input).unwrap();
        assert_eq!(b.title.as_deref(), Some("example"));
        assert_eq!(b.lines.len(), 2);
    }

    #[test]
    fn parse_no_metadata() {
        let input = "actor Alice\nactor Bob\n";
        let b = parse(input).unwrap();
        assert!(b.title.is_none());
        assert_eq!(b.lines.len(), 2);
    }

    #[test]
    fn parse_with_lang() {
        let input = "lang usecase\n---\nactor A\n";
        let b = parse(input).unwrap();
        assert_eq!(b.lang.as_deref(), Some("usecase"));
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_output() {
        let input = "actor Alice\nactor Bob\n";
        let b = parse(input).unwrap();
        let svg = render_svg(&b);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Alice"));
    }
}
