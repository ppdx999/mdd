use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::fs;

pub fn run_plugin(name: &str, input: &str) -> Result<String, String> {
    let cmd_name = format!("mdd-{}", name);

    // Try to find the plugin next to the mdd binary first, then fall back to PATH
    let cmd_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&cmd_name)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(&cmd_name));

    let mut child = Command::new(&cmd_path)
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

/// Extract only the normal path components, stripping root ("/"), drive
/// prefix ("C:\"), and UNC prefix ("\\?\") via the standard library's
/// Component decomposition.
fn relative_components(path: &Path) -> PathBuf {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(PathBuf::from(s)),
            _ => None, // skip RootDir, Prefix, CurDir, ParentDir
        })
        .collect()
}

pub fn cache_dir(source_path: &Path) -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    let abs_path = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let relative = relative_components(&abs_path);
    home.join(".cache").join("mdd").join("svgs").join(relative)
}

pub fn save_svg(dir: &Path, lang: &str, svg: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let hash = content_hash(svg);
    let filename = format!("{}-{}.svg", lang, hash);
    let path = dir.join(&filename);
    fs::write(&path, svg).map_err(|e| format!("Failed to write SVG file: {}", e))?;
    Ok(path)
}
