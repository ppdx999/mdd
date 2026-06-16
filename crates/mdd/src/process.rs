use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::path::Path;

use crate::plugin::{cache_dir, run_plugin, save_svg};

pub fn process(input: &str, source_path: &Path) -> Result<String, String> {
    let parser = Parser::new_ext(input, Options::empty());
    let svg_dir = cache_dir(source_path);

    let mut output = String::new();
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();
    let mut in_code_block = false;
    let mut list_ordered = false;
    let mut list_index: u64 = 0;
    let mut in_item = false;
    let mut link_stack: Vec<(String, String)> = Vec::new();

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
                    match run_plugin(&lang, &code_block_content) {
                        Ok(svg) => {
                            let _ = save_svg(&svg_dir, &lang, &svg);
                            output.push_str(&svg);
                            output.push('\n');
                        }
                        Err(_) => {
                            // Plugin not found — pass through as a normal code block
                            output.push_str(&format!("```{}\n", lang));
                            output.push_str(&code_block_content);
                            output.push_str("```\n");
                        }
                    }
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
                if in_item {
                    output.push('\n');
                } else {
                    output.push_str("\n\n");
                }
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
            Event::Start(Tag::List(None)) => { list_ordered = false; }
            Event::Start(Tag::List(Some(start))) => { list_ordered = true; list_index = start; }
            Event::End(TagEnd::List(_)) => {
                output.push('\n');
            }
            Event::Start(Tag::Item) => {
                in_item = true;
                if list_ordered {
                    output.push_str(&format!("{}. ", list_index));
                    list_index += 1;
                } else {
                    output.push_str("- ");
                }
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                output.push('\n');
            }
            Event::Code(text) => {
                output.push('`');
                output.push_str(&text);
                output.push('`');
            }
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                output.push('[');
                link_stack.push((dest_url.to_string(), title.to_string()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((url, title)) = link_stack.pop() {
                    if title.is_empty() {
                        output.push_str(&format!("]({})", url));
                    } else {
                        output.push_str(&format!("]({} \"{}\")", url, title));
                    }
                } else {
                    output.push(']');
                }
            }
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                output.push_str("![");
                link_stack.push((dest_url.to_string(), title.to_string()));
            }
            Event::End(TagEnd::Image) => {
                if let Some((url, title)) = link_stack.pop() {
                    if title.is_empty() {
                        output.push_str(&format!("]({})", url));
                    } else {
                        output.push_str(&format!("]({} \"{}\")", url, title));
                    }
                } else {
                    output.push(']');
                }
            }
            Event::Rule => {
                output.push_str("---\n");
            }
            _ => {}
        }
    }

    Ok(output)
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

    #[test]
    fn preserves_inline_code() {
        let input = "Use `cargo build` to compile.\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("`cargo build`"));
    }

    #[test]
    fn preserves_headings_all_levels() {
        let input = "# H1\n\n## H2\n\n### H3\n\n#### H4\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("# H1"));
        assert!(result.contains("## H2"));
        assert!(result.contains("### H3"));
        assert!(result.contains("#### H4"));
    }

    #[test]
    fn preserves_horizontal_rule() {
        let input = "before\n\n---\n\nafter\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("---"));
    }

    #[test]
    fn unknown_plugin_passes_through_code_block() {
        let input = "```rust\nfn main() {}\n```\n";
        let result = process(input, Path::new("/tmp/test.md")).unwrap();
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main() {}"));
        assert!(result.contains("```"));
    }

    #[test]
    fn empty_input() {
        let result = process("", Path::new("/tmp/test.md")).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn only_whitespace() {
        let result = process("   \n\n  \n", Path::new("/tmp/test.md")).unwrap();
        assert_eq!(result.trim(), "");
    }
}
