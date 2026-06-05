use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::plugin::run_plugin;

pub fn build_html(input: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(input, opts);

    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();

    let events: Vec<Event> = parser
        .flat_map(|event| match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang)))
                if !lang.is_empty() =>
            {
                code_block_lang = Some(lang.to_string());
                code_block_content.clear();
                vec![]
            }
            Event::Text(ref text) if code_block_lang.is_some() => {
                code_block_content.push_str(text);
                vec![]
            }
            Event::End(TagEnd::CodeBlock) if code_block_lang.is_some() => {
                let lang = code_block_lang.take().unwrap();
                match run_plugin(&lang, &code_block_content) {
                    Ok(svg) => {
                        vec![Event::Html(svg.into())]
                    }
                    Err(_) => {
                        vec![
                            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang.into()))),
                            Event::Text(code_block_content.clone().into()),
                            Event::End(TagEnd::CodeBlock),
                        ]
                    }
                }
            }
            other => vec![other],
        })
        .collect();

    let mut html_body = String::new();
    pulldown_cmark::html::push_html(&mut html_body, events.into_iter());
    html_body
}

pub fn build_html_file(path: &Path) -> PathBuf {
    let input = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to read {}: {}", path.display(), e);
        std::process::exit(1);
    });

    let html_body = build_html(&input);

    let title = path.file_name().unwrap_or_default().to_string_lossy();
    let html = format!(
        r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<title>{title}</title>
<style>
body {{ max-width: 900px; margin: 40px auto; padding: 0 20px; font-family: sans-serif; line-height: 1.6; color: #333; }}
h1, h2, h3 {{ border-bottom: 1px solid #eee; padding-bottom: 0.3em; }}
img {{ max-width: 100%; }}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid #ddd; padding: 8px 12px; text-align: left; }}
th {{ background: #f5f5f5; font-weight: bold; }}
blockquote {{ border-left: 4px solid #ddd; margin: 1em 0; padding: 0.5em 1em; color: #666; }}
code {{ background: #f5f5f5; padding: 2px 6px; border-radius: 3px; font-size: 0.9em; }}
pre {{ background: #f5f5f5; padding: 16px; border-radius: 6px; overflow-x: auto; }}
pre code {{ background: none; padding: 0; }}
hr {{ border: none; border-top: 1px solid #ddd; margin: 2em 0; }}
</style>
</head><body>
{html_body}
</body></html>"#
    );

    let html_path = path.with_extension("html");
    fs::write(&html_path, &html).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to write {}: {}", html_path.display(), e);
        std::process::exit(1);
    });
    html_path
}

fn open_browser(path: &Path) {
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(path).spawn();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(path).spawn();
}

pub fn preview(path: &Path) {
    let html_path = build_html_file(path);
    eprintln!("mdd: Built {}", html_path.display());

    open_browser(&html_path);

    let mut last_modified = fs::metadata(path).and_then(|m| m.modified()).ok();
    eprintln!(
        "mdd: Watching {} for changes... (Ctrl+C to stop)",
        path.display()
    );

    loop {
        thread::sleep(Duration::from_secs(1));
        let current = fs::metadata(path).and_then(|m| m.modified()).ok();
        if current != last_modified {
            last_modified = current;
            build_html_file(path);
            eprintln!("mdd: Rebuilt {}", html_path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Headings
    // -----------------------------------------------------------------------

    #[test]
    fn heading_levels() {
        let html = build_html("# H1\n\n## H2\n\n### H3\n\n#### H4\n");
        assert!(html.contains("<h1>H1</h1>"));
        assert!(html.contains("<h2>H2</h2>"));
        assert!(html.contains("<h3>H3</h3>"));
        assert!(html.contains("<h4>H4</h4>"));
    }

    // -----------------------------------------------------------------------
    // Inline formatting
    // -----------------------------------------------------------------------

    #[test]
    fn bold_and_italic() {
        let html = build_html("**bold** and *italic*\n");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn inline_code() {
        let html = build_html("Use `cargo build` here.\n");
        assert!(html.contains("<code>cargo build</code>"));
    }

    #[test]
    fn strikethrough() {
        let html = build_html("~~deleted~~\n");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn link() {
        let html = build_html("[example](https://example.com)\n");
        assert!(html.contains("<a href=\"https://example.com\">example</a>"));
    }

    #[test]
    fn image() {
        let html = build_html("![alt](image.png)\n");
        assert!(html.contains("<img src=\"image.png\" alt=\"alt\""));
    }

    // -----------------------------------------------------------------------
    // Lists
    // -----------------------------------------------------------------------

    #[test]
    fn unordered_list() {
        let html = build_html("- one\n- two\n- three\n");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>one</li>"));
        assert!(html.contains("<li>two</li>"));
        assert!(html.contains("<li>three</li>"));
    }

    #[test]
    fn ordered_list() {
        let html = build_html("1. first\n2. second\n3. third\n");
        assert!(html.contains("<ol>"));
        assert!(html.contains("<li>first</li>"));
        assert!(html.contains("<li>second</li>"));
    }

    #[test]
    fn nested_list() {
        let html = build_html("- a\n  - b\n  - c\n- d\n");
        assert!(html.contains("<li>a"));
        assert!(html.contains("<li>b</li>"));
        assert!(html.contains("<li>d"));
        // Nested list should have inner <ul>
        let ul_count = html.matches("<ul>").count();
        assert!(ul_count >= 2);
    }

    #[test]
    fn tasklist() {
        let html = build_html("- [x] done\n- [ ] todo\n");
        assert!(html.contains("checked"));
        assert!(html.contains("type=\"checkbox\""));
    }

    // -----------------------------------------------------------------------
    // Table
    // -----------------------------------------------------------------------

    #[test]
    fn table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        let html = build_html(input);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<td>1</td>"));
        assert!(html.contains("<td>4</td>"));
    }

    #[test]
    fn table_with_alignment() {
        let input = "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |\n";
        let html = build_html(input);
        assert!(html.contains("<table>"));
        assert!(html.contains("Left"));
        assert!(html.contains("Center"));
        assert!(html.contains("Right"));
    }

    #[test]
    fn table_empty_cells() {
        let input = "| A | B |\n|---|---|\n|   |   |\n";
        let html = build_html(input);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td></td>"));
    }

    // -----------------------------------------------------------------------
    // Blockquote
    // -----------------------------------------------------------------------

    #[test]
    fn blockquote() {
        let html = build_html("> This is quoted.\n");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is quoted."));
    }

    #[test]
    fn nested_blockquote() {
        let html = build_html("> outer\n>\n>> inner\n");
        let bq_count = html.matches("<blockquote>").count();
        assert!(bq_count >= 2);
    }

    // -----------------------------------------------------------------------
    // Code blocks
    // -----------------------------------------------------------------------

    #[test]
    fn fenced_code_block() {
        let input = "```rust\nfn main() {}\n```\n";
        let html = build_html(input);
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn unlabeled_code_block() {
        let input = "```\nplain code\n```\n";
        let html = build_html(input);
        assert!(html.contains("<pre>"));
        assert!(html.contains("plain code"));
    }

    // -----------------------------------------------------------------------
    // Horizontal rule
    // -----------------------------------------------------------------------

    #[test]
    fn horizontal_rule() {
        let html = build_html("---\n");
        assert!(html.contains("<hr"));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn empty_input() {
        let html = build_html("");
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn only_whitespace() {
        let html = build_html("   \n\n   \n");
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn special_characters_in_text() {
        // &, < in inline code are escaped; bare < is parsed as HTML tag by pulldown-cmark
        let html = build_html("Use `<div>` & `\"quotes\"` in text.\n");
        assert!(html.contains("&lt;div&gt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("\"quotes\""));
    }

    #[test]
    fn japanese_text() {
        let html = build_html("# 日本語の見出し\n\nこれは段落です。\n");
        assert!(html.contains("日本語の見出し"));
        assert!(html.contains("これは段落です。"));
    }

    #[test]
    fn mixed_content() {
        let input = "# Title\n\nText with **bold**.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n- item\n\n> quote\n\n```js\nconsole.log('hi');\n```\n";
        let html = build_html(input);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<li>item</li>"));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("console.log"));
    }

    #[test]
    fn multiple_paragraphs() {
        let html = build_html("First paragraph.\n\nSecond paragraph.\n");
        assert!(html.contains("<p>First paragraph.</p>"));
        assert!(html.contains("<p>Second paragraph.</p>"));
    }

    #[test]
    fn hard_line_break() {
        let html = build_html("line one  \nline two\n");
        assert!(html.contains("<br"));
    }
}
