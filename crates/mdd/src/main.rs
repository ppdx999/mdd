mod pdf;
mod plugin;
mod preview;
pub(crate) mod process;

use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

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
        3 if args[1] == "preview" => {
            let path = Path::new(&args[2]);
            preview::preview(path);
        }
        3 if args[1] == "pdf" => {
            let path = Path::new(&args[2]);
            pdf::generate_pdf(path);
        }
        _ => {
            eprintln!("Usage: mdd <file.md>");
            eprintln!("       mdd preview <file.md>");
            eprintln!("       mdd pdf <file.md> > output.pdf");
            std::process::exit(1);
        }
    }
}
