mod plugin;
mod preview;
pub(crate) mod process;
mod slide;
mod watch;

use std::fs;
use std::path::Path;

fn find_plugins() -> Vec<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut plugins: Vec<String> = Vec::new();

    for dir in std::env::split_paths(&path_var) {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("mdd-") && !plugins.contains(&name) {
                    plugins.push(name);
                }
            }
        }
    }

    plugins.sort();
    plugins
}

fn print_help() {
    eprintln!("Usage: mdd <file.md>");
    eprintln!("       mdd html <file.md>");
    eprintln!("       mdd build <dir> -o <outdir>");
    eprintln!("       mdd preview <file.md>");
    eprintln!("       mdd slide <file.md> > output.pdf");
    eprintln!("       mdd slide-preview <file.md>");
    eprintln!("       mdd watch <dir>");
    eprintln!("       mdd slide-watch <dir>");
    eprintln!();

    let plugins = find_plugins();
    if !plugins.is_empty() {
        eprintln!("Installed plugins ({}):", plugins.len());
        for name in &plugins {
            eprintln!("  {}", name);
        }
        eprintln!();
        eprintln!("Run `<plugin> --help` for plugin-specific DSL syntax.");
    }

    eprintln!();
    eprintln!("For more information, visit https://github.com/ppdx999/mdd");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    match args.len() {
        2 => {
            let path = Path::new(&args[1]);
            let input = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("mdd: Failed to read {}: {}", path.display(), e);
                std::process::exit(1);
            });
            match process::process(&input, path) {
                Ok(result) => print!("{}", result),
                Err(e) => {
                    eprintln!("mdd: {}", e);
                    std::process::exit(1);
                }
            }
        }
        3 if args[1] == "html" => {
            let path = Path::new(&args[2]);
            let input = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("mdd: Failed to read {}: {}", path.display(), e);
                std::process::exit(1);
            });
            let processed = match process::process(&input, path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("mdd: {}", e);
                    std::process::exit(1);
                }
            };
            let title = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("mdd");
            print!("{}", watch::markdown_to_html(&processed, title));
        }
        5 if args[1] == "build" && args[3] == "-o" => {
            let dir = Path::new(&args[2]);
            let outdir = Path::new(&args[4]);
            watch::build(dir, outdir);
        }
        3 if args[1] == "preview" => {
            let path = Path::new(&args[2]);
            preview::preview(path);
        }
        3 if args[1] == "slide" => {
            let path = Path::new(&args[2]);
            slide::generate_slide(path);
        }
        3 if args[1] == "slide-preview" => {
            let path = Path::new(&args[2]);
            slide::preview_slide(path);
        }
        3 if args[1] == "watch" => {
            let path = Path::new(&args[2]);
            watch::watch(path);
        }
        3 if args[1] == "slide-watch" => {
            let path = Path::new(&args[2]);
            watch::slide_watch(path);
        }
        _ => {
            print_help();
            std::process::exit(1);
        }
    }
}
