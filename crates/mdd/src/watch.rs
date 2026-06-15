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

    // Initial build of all .md files
    let mut timestamps: HashMap<String, SystemTime> = HashMap::new();
    for path in find_md_files(dir) {
        build_slide_pdf(&path);
        if let Ok(modified) = fs::metadata(&path).and_then(|m| m.modified()) {
            timestamps.insert(path, modified);
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
    html.push_str("body { font-family: -apple-system, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; color: #333; }\n");
    html.push_str("h1 { border-bottom: 2px solid #e8eaf6; padding-bottom: 8px; }\n");
    html.push_str("ul { list-style: none; padding: 0; }\n");
    html.push_str("li { padding: 8px 0; border-bottom: 1px solid #eee; }\n");
    html.push_str("a { color: #283593; text-decoration: none; font-size: 16px; }\n");
    html.push_str("a:hover { text-decoration: underline; }\n");
    html.push_str(".path { color: #999; font-size: 13px; margin-left: 8px; }\n");
    html.push_str(".count { color: #666; font-size: 14px; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str(&format!("<h1>{}</h1>\n", dir_name));
    html.push_str(&format!(
        "<p class=\"count\">{} documents</p>\n",
        pdf_files.len()
    ));
    html.push_str("<ul>\n");

    for pdf_rel in &pdf_files {
        let name = Path::new(pdf_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(pdf_rel);
        let parent = Path::new(pdf_rel)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        html.push_str(&format!(
            "  <li><a href=\"{}\">{}</a>",
            pdf_rel, name
        ));
        if !parent.is_empty() {
            html.push_str(&format!("<span class=\"path\">{}/</span>", parent));
        }
        html.push_str("</li>\n");
    }

    html.push_str("</ul>\n");
    html.push_str("</body>\n</html>\n");

    let index_path = dir.join("index.html");
    match fs::write(&index_path, &html) {
        Ok(_) => eprintln!("mdd: Updated {}", index_path.display()),
        Err(e) => eprintln!("mdd: Failed to write index.html: {}", e),
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
