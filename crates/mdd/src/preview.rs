use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn preview(path: &Path) {
    let pdf_path = path.with_extension("pdf");

    let pdf_bytes = crate::slide::build_preview_pdf(path);
    fs::write(&pdf_path, &pdf_bytes).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to write {}: {}", pdf_path.display(), e);
        std::process::exit(1);
    });
    eprintln!("mdd: Built {}", pdf_path.display());

    if let Err(e) = open::that(&pdf_path) {
        eprintln!("mdd: Failed to open PDF viewer: {}", e);
    }

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
            let pdf_bytes = crate::slide::build_preview_pdf(path);
            if let Err(e) = fs::write(&pdf_path, &pdf_bytes) {
                eprintln!("mdd: Failed to write {}: {}", pdf_path.display(), e);
            } else {
                eprintln!("mdd: Rebuilt {}", pdf_path.display());
            }
        }
    }
}
