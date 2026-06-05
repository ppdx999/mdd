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

    #[test]
    fn unknown_plugin_falls_through_as_code_block() {
        let input = "```rust\nfn main() {}\n```\n";
        let html = build_html(input);
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn unlabeled_code_block_passes_through() {
        let input = "```\nplain code\n```\n";
        let html = build_html(input);
        assert!(html.contains("<pre>"));
        assert!(html.contains("plain code"));
    }

    #[test]
    fn non_code_content_passes_through() {
        let input = "# Title\n\nSome text.\n";
        let html = build_html(input);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("Some text."));
    }
}
