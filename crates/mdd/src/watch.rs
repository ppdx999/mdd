use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

use pulldown_cmark::{Options, Parser, html};

/// Watch directory and output HTML files (markdown → processed HTML).
pub fn watch(dir: &Path) {
    if !dir.is_dir() {
        eprintln!("mdd: {} is not a directory", dir.display());
        std::process::exit(1);
    }

    let md_files = find_md_files(dir);
    let file_count = md_files.len();
    let max_parallel = 8;
    eprintln!("mdd: Building {} HTML files ({} at a time)...", file_count, max_parallel);

    for chunk in md_files.chunks(max_parallel) {
        thread::scope(|s| {
            for path in chunk {
                s.spawn(|| {
                    build_html(path);
                });
            }
        });
    }

    let mut timestamps: HashMap<String, SystemTime> = HashMap::new();
    for path in &md_files {
        if let Ok(modified) = fs::metadata(path).and_then(|m| m.modified()) {
            timestamps.insert(path.clone(), modified);
        }
    }

    generate_index_html(dir, "html");

    eprintln!(
        "mdd: Watching {} for .md changes... (Ctrl+C to stop)",
        dir.display()
    );

    loop {
        thread::sleep(Duration::from_secs(1));

        let mut any_changed = false;
        for path in find_md_files(dir) {
            let modified = match fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let changed = match timestamps.get(&path) {
                Some(prev) => *prev != modified,
                None => true,
            };

            if changed {
                timestamps.insert(path.clone(), modified);
                build_html(&path);
                any_changed = true;
            }
        }

        if any_changed {
            generate_index_html(dir, "html");
        }
    }
}

/// Watch directory and output slide PDFs.
pub fn slide_watch(dir: &Path) {
    if !dir.is_dir() {
        eprintln!("mdd: {} is not a directory", dir.display());
        std::process::exit(1);
    }

    // Initial build of all .md files (parallel, limited concurrency)
    let md_files = find_md_files(dir);
    let file_count = md_files.len();
    let max_parallel = 8;
    eprintln!("mdd: Building {} files ({} at a time)...", file_count, max_parallel);

    for chunk in md_files.chunks(max_parallel) {
        thread::scope(|s| {
            for path in chunk {
                s.spawn(|| {
                    build_slide_pdf(path);
                });
            }
        });
    }

    let mut timestamps: HashMap<String, SystemTime> = HashMap::new();
    for path in &md_files {
        if let Ok(modified) = fs::metadata(path).and_then(|m| m.modified()) {
            timestamps.insert(path.clone(), modified);
        }
    }

    generate_index_html(dir, "pdf");

    eprintln!(
        "mdd: Watching {} for .md changes... (Ctrl+C to stop)",
        dir.display()
    );

    loop {
        thread::sleep(Duration::from_secs(1));

        let mut any_changed = false;
        for path in find_md_files(dir) {
            let modified = match fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let changed = match timestamps.get(&path) {
                Some(prev) => *prev != modified,
                None => true,
            };

            if changed {
                timestamps.insert(path.clone(), modified);
                build_slide_pdf(&path);
                any_changed = true;
            }
        }

        if any_changed {
            generate_index_html(dir, "pdf");
        }
    }
}

fn find_md_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    collect_md_files(dir, &mut files);
    files
}

fn collect_md_files(dir: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path.to_string_lossy().to_string());
        }
    }
}

fn generate_index_html(dir: &Path, ext: &str) {
    let mut files: Vec<String> = Vec::new();
    collect_files_by_ext(dir, dir, ext, &mut files);
    files.sort();
    let pdf_files = files;

    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mdd");

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"ja\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str(&format!("<title>{} - mdd</title>\n", dir_name));
    html.push_str("<style>\n");
    html.push_str("body { font-family: 'SF Mono', 'Menlo', 'Consolas', monospace; max-width: 800px; margin: 40px auto; padding: 0 20px; color: #333; font-size: 14px; }\n");
    html.push_str("h1 { font-family: -apple-system, sans-serif; border-bottom: 2px solid #e8eaf6; padding-bottom: 8px; }\n");
    html.push_str(".count { color: #666; font-size: 13px; font-family: -apple-system, sans-serif; }\n");
    html.push_str(".tree { line-height: 1.8; white-space: pre; }\n");
    html.push_str(".tree a { color: #283593; text-decoration: none; }\n");
    html.push_str(".tree a:hover { text-decoration: underline; }\n");
    html.push_str(".dir { color: #1565c0; font-weight: bold; }\n");
    html.push_str(".branch { color: #999; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str(&format!("<h1>{}</h1>\n", dir_name));
    html.push_str(&format!(
        "<p class=\"count\">{} documents</p>\n",
        pdf_files.len()
    ));
    html.push_str("<div class=\"tree\">");

    // Build a tree structure from sorted paths
    let tree = build_tree(&pdf_files);
    render_tree_html(&tree, "", &mut html, true);

    html.push_str("</div>\n");
    html.push_str("</body>\n</html>\n");

    let index_path = dir.join("index.html");
    match fs::write(&index_path, &html) {
        Ok(_) => eprintln!("mdd: Updated {}", index_path.display()),
        Err(e) => eprintln!("mdd: Failed to write index.html: {}", e),
    }
}

enum TreeEntry {
    File { name: String, path: String },
    Dir { name: String, children: Vec<TreeEntry> },
}

fn build_tree(paths: &[String]) -> Vec<TreeEntry> {
    let mut root: Vec<TreeEntry> = Vec::new();

    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        insert_into_tree(&mut root, &parts, path);
    }

    root
}

fn insert_into_tree(entries: &mut Vec<TreeEntry>, parts: &[&str], full_path: &str) {
    if parts.len() == 1 {
        let name = Path::new(parts[0])
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(parts[0])
            .to_string();
        entries.push(TreeEntry::File {
            name,
            path: full_path.to_string(),
        });
        return;
    }

    let dir_name = parts[0];
    // Find existing dir entry
    let dir_idx = entries.iter().position(|e| matches!(e, TreeEntry::Dir { name, .. } if name == dir_name));

    if let Some(idx) = dir_idx {
        if let TreeEntry::Dir { children, .. } = &mut entries[idx] {
            insert_into_tree(children, &parts[1..], full_path);
        }
    } else {
        let mut children = Vec::new();
        insert_into_tree(&mut children, &parts[1..], full_path);
        entries.push(TreeEntry::Dir {
            name: dir_name.to_string(),
            children,
        });
    }
}

fn render_tree_html(entries: &[TreeEntry], prefix: &str, html: &mut String, is_root: bool) {
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        let connector = if is_root {
            ""
        } else if is_last {
            "\u{2514}\u{2500}\u{2500} " // └──
        } else {
            "\u{251C}\u{2500}\u{2500} " // ├──
        };
        let child_prefix = if is_root {
            prefix.to_string()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}\u{2502}   ", prefix) // │
        };

        match entry {
            TreeEntry::Dir { name, children } => {
                html.push_str(&format!(
                    "<span class=\"branch\">{}{}</span><span class=\"dir\">{}/</span>\n",
                    prefix, connector, name
                ));
                render_tree_html(children, &child_prefix, html, false);
            }
            TreeEntry::File { name, path } => {
                html.push_str(&format!(
                    "<span class=\"branch\">{}{}</span><a href=\"{}\" target=\"_blank\">{}</a>\n",
                    prefix, connector, path,
                    Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or(&name)
                ));
            }
        }
    }
}

fn collect_files_by_ext(base: &Path, dir: &Path, ext: &str, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_by_ext(base, &path, ext, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            if let Ok(rel) = path.strip_prefix(base) {
                files.push(rel.to_string_lossy().to_string());
            }
        }
    }
}

fn build_html(md_path: &str) {
    let path = Path::new(md_path);
    let html_path = path.with_extension("html");

    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd: Failed to read {}: {}", md_path, e);
            return;
        }
    };

    // Step 1: Process plugins (replace mdd code blocks with SVGs)
    let processed = match crate::process::process(&input, path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd: {}: {}", md_path, e);
            return;
        }
    };

    // Step 2: Extract SVGs and replace with placeholders before Markdown→HTML conversion
    let mut svgs: Vec<String> = Vec::new();
    let mut md_with_placeholders = String::new();
    let mut remaining = processed.as_str();
    while let Some(start) = remaining.find("<svg") {
        md_with_placeholders.push_str(&remaining[..start]);
        if let Some(end) = remaining[start..].find("</svg>") {
            let svg = &remaining[start..start + end + 6];
            let placeholder = format!("<!--SVG_PLACEHOLDER_{}-->", svgs.len());
            svgs.push(svg.to_string());
            md_with_placeholders.push_str(&placeholder);
            remaining = &remaining[start + end + 6..];
        } else {
            md_with_placeholders.push_str(&remaining[start..]);
            remaining = "";
        }
    }
    md_with_placeholders.push_str(remaining);

    // Step 3: Convert Markdown to HTML
    let parser = Parser::new_ext(&md_with_placeholders, Options::all());
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    // Step 4: Restore SVGs
    for (i, svg) in svgs.iter().enumerate() {
        let placeholder = format!("<!--SVG_PLACEHOLDER_{}-->", i);
        html_body = html_body.replace(&placeholder, svg);
    }

    // Step 5: Wrap in full HTML document
    let title = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mdd");
    let full_html = format!(
        "<!DOCTYPE html>\n<html lang=\"ja\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n\
         <style>\n\
         body {{ font-family: -apple-system, sans-serif; max-width: 900px; margin: 40px auto; padding: 0 20px; color: #333; line-height: 1.6; }}\n\
         h1 {{ border-bottom: 2px solid #e8eaf6; padding-bottom: 8px; }}\n\
         h2 {{ border-bottom: 1px solid #eee; padding-bottom: 4px; }}\n\
         code {{ background: #f5f5f5; padding: 2px 6px; border-radius: 3px; font-size: 0.9em; }}\n\
         pre {{ background: #f5f5f5; padding: 16px; border-radius: 6px; overflow-x: auto; }}\n\
         pre code {{ background: none; padding: 0; }}\n\
         table {{ border-collapse: collapse; margin: 16px 0; }}\n\
         th, td {{ border: 1px solid #ddd; padding: 8px 12px; }}\n\
         th {{ background: #f5f5f5; }}\n\
         svg {{ max-width: 100%; height: auto; display: block; margin: 16px 0; }}\n\
         </style>\n\
         </head>\n<body>\n{}\n</body>\n</html>",
        title, html_body
    );

    match fs::write(&html_path, &full_html) {
        Ok(_) => eprintln!("mdd: Built {}", html_path.display()),
        Err(e) => eprintln!("mdd: Failed to write {}: {}", html_path.display(), e),
    }
}

fn build_slide_pdf(md_path: &str) {
    let path = Path::new(md_path);
    let pdf_path = path.with_extension("pdf");

    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd: Failed to read {}: {}", md_path, e);
            return;
        }
    };

    let processed = match crate::process::process(&input, path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdd: {}: {}", md_path, e);
            return;
        }
    };

    let pdf_bytes = crate::slide::build_slide_pdf_from_processed(&processed);

    match fs::write(&pdf_path, &pdf_bytes) {
        Ok(_) => eprintln!("mdd: Built {}", pdf_path.display()),
        Err(e) => eprintln!("mdd: Failed to write {}: {}", pdf_path.display(), e),
    }
}
