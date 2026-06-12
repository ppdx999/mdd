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

    eprintln!(
        "mdd: Watching {} for .md changes... (Ctrl+C to stop)",
        dir.display()
    );

    loop {
        thread::sleep(Duration::from_secs(1));

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
            }
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
