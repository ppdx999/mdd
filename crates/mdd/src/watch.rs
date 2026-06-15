use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

pub fn watch(dir: &Path) {
    if !dir.is_dir() {
        eprintln!("mdd: {} is not a directory", dir.display());
        std::process::exit(1);
    }

    // Initial build of all .md files (parallel)
    let md_files = find_md_files(dir);
    let file_count = md_files.len();
    eprintln!("mdd: Building {} files in parallel...", file_count);

    thread::scope(|s| {
        for path in &md_files {
            s.spawn(|| {
                build_slide_pdf(path);
            });
        }
    });

    let mut timestamps: HashMap<String, SystemTime> = HashMap::new();
    for path in &md_files {
        if let Ok(modified) = fs::metadata(path).and_then(|m| m.modified()) {
            timestamps.insert(path.clone(), modified);
        }
    }

    // Generate index.html with links to all PDFs
    generate_index_html(dir);

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
                None => true, // new file
            };

            if changed {
                timestamps.insert(path.clone(), modified);
                build_slide_pdf(&path);
                any_changed = true;
            }
        }

        if any_changed {
            generate_index_html(dir);
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

fn generate_index_html(dir: &Path) {
    let mut pdf_files: Vec<String> = Vec::new();
    collect_pdf_files(dir, dir, &mut pdf_files);
    pdf_files.sort();

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
                    "<span class=\"branch\">{}</span><span class=\"dir\">{}/</span>\n",
                    connector, name
                ));
                render_tree_html(children, &child_prefix, html, false);
            }
            TreeEntry::File { name, path } => {
                html.push_str(&format!(
                    "<span class=\"branch\">{}</span><a href=\"{}\">{}.pdf</a>\n",
                    connector, path, name
                ));
            }
        }
    }
}

fn collect_pdf_files(base: &Path, dir: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pdf_files(base, &path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            if let Ok(rel) = path.strip_prefix(base) {
                files.push(rel.to_string_lossy().to_string());
            }
        }
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
