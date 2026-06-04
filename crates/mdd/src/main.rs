use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn run_plugin(name: &str, input: &str) -> Result<String, String> {
    let cmd_name = format!("mdd-{}", name);
    let mut child = Command::new(&cmd_name)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute {}: {}", cmd_name, e))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("Failed to write to {} stdin: {}", cmd_name, e))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for {}: {}", cmd_name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} exited with {}: {}",
            cmd_name, output.status, stderr
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 output from {}: {}", cmd_name, e))
}

fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cache_dir(source_path: &Path) -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    let abs_path = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let stripped = abs_path
        .strip_prefix("/")
        .unwrap_or(&abs_path);
    Path::new(&home).join(".cache/mdd/svgs").join(stripped)
}

fn save_svg(dir: &Path, lang: &str, svg: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let hash = content_hash(svg);
    let filename = format!("{}-{}.svg", lang, hash);
    let path = dir.join(&filename);
    fs::write(&path, svg).map_err(|e| format!("Failed to write SVG file: {}", e))?;
    Ok(path)
}

fn process(input: &str, source_path: &Path) -> Result<String, String> {
    let parser = Parser::new_ext(input, Options::empty());
    let svg_dir = cache_dir(source_path);

    let mut output = String::new();
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                let lang = lang.to_string();
                if lang.is_empty() {
                    output.push_str("```\n");
                    in_code_block = true;
                } else {
                    code_block_lang = Some(lang);
                    code_block_content.clear();
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                output.push_str("    ");
                in_code_block = true;
            }
            Event::Text(text) if code_block_lang.is_some() => {
                code_block_content.push_str(&text);
            }
            Event::Text(text) if in_code_block => {
                output.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(lang) = code_block_lang.take() {
                    let svg = run_plugin(&lang, &code_block_content)?;
                    let svg_path = save_svg(&svg_dir, &lang, &svg)?;
                    output.push_str(&format!(
                        "![{}]({})\n",
                        lang,
                        svg_path.display()
                    ));
                } else {
                    in_code_block = false;
                    output.push_str("```\n");
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                output.push_str(&format!("{} ", "#".repeat(level as usize)));
            }
            Event::End(TagEnd::Heading(_)) => {
                output.push('\n');
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push('\n');
            }
            Event::Text(text) => {
                output.push_str(&text);
            }
            Event::SoftBreak => {
                output.push('\n');
            }
            Event::HardBreak => {
                output.push_str("  \n");
            }
            Event::Start(Tag::Strong) => {
                output.push_str("**");
            }
            Event::End(TagEnd::Strong) => {
                output.push_str("**");
            }
            Event::Start(Tag::Emphasis) => {
                output.push('*');
            }
            Event::End(TagEnd::Emphasis) => {
                output.push('*');
            }
            Event::Start(Tag::List(None)) | Event::End(TagEnd::List(false)) => {}
            Event::Start(Tag::List(Some(_))) | Event::End(TagEnd::List(true)) => {}
            Event::Start(Tag::Item) => {
                output.push_str("- ");
            }
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }
            Event::Code(text) => {
                output.push('`');
                output.push_str(&text);
                output.push('`');
            }
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                output.push('[');
                // Store url for end tag - handled by text events in between
                let _ = (dest_url, title);
            }
            Event::Rule => {
                output.push_str("---\n");
            }
            _ => {}
        }
    }

    Ok(output)
}

fn build_html(path: &Path) -> PathBuf {
    let input = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to read {}: {}", path.display(), e);
        std::process::exit(1);
    });

    let md_output = match process(&input, path) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("mdd: {}", e);
            std::process::exit(1);
        }
    };

    let parser = Parser::new_ext(&md_output, Options::empty());
    let mut html_body = String::new();
    pulldown_cmark::html::push_html(&mut html_body, parser);

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
code {{ background: #f5f5f5; padding: 2px 6px; border-radius: 3px; font-size: 0.9em; }}
pre {{ background: #f5f5f5; padding: 16px; border-radius: 6px; overflow-x: auto; }}
pre code {{ background: none; padding: 0; }}
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

fn preview(path: &Path) {
    let html_path = build_html(path);
    eprintln!("mdd: Built {}", html_path.display());

    open_browser(&html_path);

    let mut last_modified = fs::metadata(path).and_then(|m| m.modified()).ok();
    eprintln!("mdd: Watching {} for changes... (Ctrl+C to stop)", path.display());

    loop {
        thread::sleep(Duration::from_secs(1));
        let current = fs::metadata(path).and_then(|m| m.modified()).ok();
        if current != last_modified {
            last_modified = current;
            build_html(path);
            eprintln!("mdd: Rebuilt {}", html_path.display());
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        2 => {
            let path = Path::new(&args[1]);
            let input = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("mdd: Failed to read {}: {}", path.display(), e);
                std::process::exit(1);
            });
            match process(&input, path) {
                Ok(result) => print!("{}", result),
                Err(e) => {
                    eprintln!("mdd: {}", e);
                    std::process::exit(1);
                }
            }
        }
        3 if args[1] == "preview" => {
            let path = Path::new(&args[2]);
            preview(path);
        }
        _ => {
            eprintln!("Usage: mdd <file.md>");
            eprintln!("       mdd preview <file.md>");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn passthrough_plain_markdown() {
        let input = "# Hello\n\nSome text.\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("# Hello"));
        assert!(result.contains("Some text."));
    }

    #[test]
    fn passthrough_unlabeled_code_block() {
        let input = "```\nsome code\n```\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("```"));
        assert!(result.contains("some code"));
    }

    #[test]
    fn preserves_emphasis() {
        let input = "This is **bold** and *italic*.\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("**bold**"));
        assert!(result.contains("*italic*"));
    }

    #[test]
    fn preserves_list() {
        let input = "- item one\n- item two\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("- item one"));
        assert!(result.contains("- item two"));
    }
}
