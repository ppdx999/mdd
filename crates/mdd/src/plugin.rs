use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs};

pub fn run_plugin(name: &str, input: &str) -> Result<String, String> {
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

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .expect("HOME or USERPROFILE not set")
}

fn strip_root(path: &Path) -> PathBuf {
    // Unix: strip leading "/"
    if let Ok(stripped) = path.strip_prefix("/") {
        return stripped.to_path_buf();
    }
    // Windows: canonicalize returns "\\?\C:\..." UNC paths; strip that prefix first
    let mut s = path.to_string_lossy().into_owned();
    if s.starts_with(r"\\?\") {
        s = s[4..].to_string();
    }
    // Strip drive prefix like "C:\" or "C:/"
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        return PathBuf::from(&s[3..]);
    }
    path.to_path_buf()
}

pub fn cache_dir(source_path: &Path) -> PathBuf {
    let home = home_dir();
    let abs_path = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let stripped = strip_root(&abs_path);
    home.join(".cache").join("mdd").join("svgs").join(stripped)
}

pub fn save_svg(dir: &Path, lang: &str, svg: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let hash = content_hash(svg);
    let filename = format!("{}-{}.svg", lang, hash);
    let path = dir.join(&filename);
    fs::write(&path, svg).map_err(|e| format!("Failed to write SVG file: {}", e))?;
    Ok(path)
}
