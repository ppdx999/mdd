use std::collections::HashMap;
use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Commit {
    id: String,
    message: String,
    branch: String,
    tag: Option<String>,
}

#[derive(Debug)]
struct Merge {
    from: String, // commit id
    to: String,   // commit id
}

#[derive(Debug)]
struct GitGraph {
    branches: Vec<String>,
    commits: Vec<Commit>,
    merges: Vec<Merge>,
    #[allow(dead_code)]
    current_branch: String,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<GitGraph, String> {
    let mut branches: Vec<String> = vec!["main".to_string()];
    let mut commits: Vec<Commit> = Vec::new();
    let mut merges: Vec<Merge> = Vec::new();
    let mut current_branch = "main".to_string();
    let mut commit_counter = 0;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }

        // branch name
        if t.starts_with("branch ") {
            let name = t.strip_prefix("branch ").unwrap().trim().to_string();
            if !branches.contains(&name) {
                branches.push(name);
            }
            continue;
        }

        // checkout name
        if t.starts_with("checkout ") {
            let name = t.strip_prefix("checkout ").unwrap().trim().to_string();
            if !branches.contains(&name) {
                return Err(format!("Unknown branch: {}", name));
            }
            current_branch = name;
            continue;
        }

        // commit "message"
        // commit "message" tag "v1.0"
        if t.starts_with("commit ") {
            let rest = t.strip_prefix("commit ").unwrap().trim();
            let (message, tag) = if let Some(tag_pos) = rest.find(" tag ") {
                let msg = strip_quotes(rest[..tag_pos].trim());
                let tag = strip_quotes(rest[tag_pos + 5..].trim());
                (msg.to_string(), Some(tag.to_string()))
            } else {
                (strip_quotes(rest).to_string(), None)
            };

            let id = format!("c{}", commit_counter);
            commit_counter += 1;
            commits.push(Commit {
                id,
                message,
                branch: current_branch.clone(),
                tag,
            });
            continue;
        }

        // commit (no message)
        if t == "commit" {
            let id = format!("c{}", commit_counter);
            commit_counter += 1;
            commits.push(Commit {
                id,
                message: String::new(),
                branch: current_branch.clone(),
                tag: None,
            });
            continue;
        }

        // merge branch_name
        if t.starts_with("merge ") {
            let from_branch = t.strip_prefix("merge ").unwrap().trim().to_string();
            // Find last commit on from_branch
            let from_commit = commits
                .iter()
                .rev()
                .find(|c| c.branch == from_branch)
                .ok_or_else(|| format!("No commits on branch: {}", from_branch))?
                .id
                .clone();

            // Create merge commit on current branch
            let id = format!("c{}", commit_counter);
            commit_counter += 1;
            let message = format!("Merge {} into {}", from_branch, current_branch);
            commits.push(Commit {
                id: id.clone(),
                message,
                branch: current_branch.clone(),
                tag: None,
            });
            merges.push(Merge {
                from: from_commit,
                to: id,
            });
            continue;
        }

        return Err(format!("Unknown syntax: {}", t));
    }

    if commits.is_empty() {
        return Err("At least 1 commit is required".to_string());
    }

    Ok(GitGraph {
        branches,
        commits,
        merges,
        current_branch,
    })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CHAR_W: f64 = 8.0;
const CJK_W: f64 = 14.0;
const COMMIT_RADIUS: f64 = 8.0;
const COMMIT_GAP_Y: f64 = 50.0;
const BRANCH_GAP_X: f64 = 60.0;
const PADDING: f64 = 40.0;
const MSG_OFFSET_X: f64 = 20.0;
const TAG_H: f64 = 20.0;
const TAG_H_PAD: f64 = 8.0;
const TAG_RADIUS: f64 = 4.0;

const BRANCH_COLORS: &[&str] = &[
    "#1565c0", "#2e7d32", "#f57f17", "#7b1fa2",
    "#00695c", "#c62828", "#283593", "#e65100",
];

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_W } else { CJK_W })
        .sum()
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(graph: &GitGraph) -> String {
    let n_branches = graph.branches.len();
    let n_commits = graph.commits.len();

    // Map branch name -> lane index
    let branch_lane: HashMap<&str, usize> = graph
        .branches
        .iter()
        .enumerate()
        .map(|(i, b)| (b.as_str(), i))
        .collect();

    // Map commit id -> index
    let commit_idx: HashMap<&str, usize> = graph
        .commits
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.as_str(), i))
        .collect();

    // Compute message widths for total width
    let max_msg_w = graph
        .commits
        .iter()
        .map(|c| text_width(&c.message))
        .fold(0.0_f64, f64::max);

    let graph_w = n_branches as f64 * BRANCH_GAP_X;
    let total_w = PADDING + graph_w + MSG_OFFSET_X + max_msg_w + PADDING + 40.0;
    let total_h = PADDING + n_commits as f64 * COMMIT_GAP_Y + PADDING;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str(
        "<style>text { font-family: monospace, sans-serif; font-size: 12px; fill: #333; }</style>",
    );

    // Branch labels at top
    for (i, branch) in graph.branches.iter().enumerate() {
        let bx = PADDING + i as f64 * BRANCH_GAP_X;
        let color = BRANCH_COLORS[i % BRANCH_COLORS.len()];
        let lw = text_width(branch) + 12.0;
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"20\" rx=\"4\" fill=\"{}\" opacity=\"0.15\"/>",
            bx - lw / 2.0,
            PADDING - 28.0,
            lw,
            color
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{}</text>",
            bx,
            PADDING - 14.0,
            color,
            escape_xml(branch)
        ));
    }

    // Draw branch lines (vertical lines for each lane that has commits)
    for (i, branch) in graph.branches.iter().enumerate() {
        let bx = PADDING + i as f64 * BRANCH_GAP_X;
        let color = BRANCH_COLORS[i % BRANCH_COLORS.len()];

        // Find first and last commit on this branch
        let first = graph
            .commits
            .iter()
            .enumerate()
            .find(|(_, c)| c.branch == *branch);
        let last = graph
            .commits
            .iter()
            .enumerate()
            .rev()
            .find(|(_, c)| c.branch == *branch);

        if let (Some((fi, _)), Some((li, _))) = (first, last) {
            let y1 = PADDING + fi as f64 * COMMIT_GAP_Y;
            let y2 = PADDING + li as f64 * COMMIT_GAP_Y;
            if y1 < y2 {
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"3\" opacity=\"0.4\"/>",
                    bx, y1, bx, y2, color
                ));
            }
        }
    }

    // Draw merge lines
    for merge in &graph.merges {
        if let (Some(&from_idx), Some(&to_idx)) =
            (commit_idx.get(merge.from.as_str()), commit_idx.get(merge.to.as_str()))
        {
            let from_commit = &graph.commits[from_idx];
            let to_commit = &graph.commits[to_idx];
            let from_lane = branch_lane[from_commit.branch.as_str()];
            let to_lane = branch_lane[to_commit.branch.as_str()];
            let from_x = PADDING + from_lane as f64 * BRANCH_GAP_X;
            let from_y = PADDING + from_idx as f64 * COMMIT_GAP_Y;
            let to_x = PADDING + to_lane as f64 * BRANCH_GAP_X;
            let to_y = PADDING + to_idx as f64 * COMMIT_GAP_Y;

            let color = BRANCH_COLORS[from_lane % BRANCH_COLORS.len()];
            svg.push_str(&format!(
                "<path d=\"M{},{} C{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"6,3\"/>",
                from_x, from_y,
                from_x, (from_y + to_y) / 2.0,
                to_x, (from_y + to_y) / 2.0,
                to_x, to_y,
                color
            ));
        }
    }

    // Draw commits
    for (i, commit) in graph.commits.iter().enumerate() {
        let lane = branch_lane[commit.branch.as_str()];
        let cx = PADDING + lane as f64 * BRANCH_GAP_X;
        let cy = PADDING + i as f64 * COMMIT_GAP_Y;
        let color = BRANCH_COLORS[lane % BRANCH_COLORS.len()];

        // Commit dot
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"white\" stroke-width=\"3\"/>",
            cx, cy, COMMIT_RADIUS, color
        ));

        // Message
        if !commit.message.is_empty() {
            let msg_x = PADDING + graph_w + MSG_OFFSET_X;
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>",
                msg_x,
                cy + 4.0,
                escape_xml(&commit.message)
            ));
        }

        // Tag
        if let Some(ref tag) = commit.tag {
            let tag_x = cx + COMMIT_RADIUS + 6.0;
            let tw = text_width(tag) + TAG_H_PAD * 2.0;
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" opacity=\"0.15\"/>",
                tag_x,
                cy - TAG_H / 2.0,
                tw,
                TAG_H,
                TAG_RADIUS,
                color
            ));
            // Tag icon (small label shape)
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"10\" font-weight=\"bold\" fill=\"{}\">\u{1F3F7} {}</text>",
                tag_x + 4.0,
                cy + 4.0,
                color,
                escape_xml(tag)
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let graph = match parse(&input) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("mdd-gitgraph: {}", e);
            std::process::exit(1);
        }
    };

    print!("{}", render_svg(&graph));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "commit \"Initial commit\"\ncommit \"Add feature\"\n";
        let g = parse(input).unwrap();
        assert_eq!(g.commits.len(), 2);
        assert_eq!(g.commits[0].branch, "main");
    }

    #[test]
    fn parse_branches() {
        let input = "commit \"Init\"\nbranch feature\ncheckout feature\ncommit \"Feature work\"\n";
        let g = parse(input).unwrap();
        assert_eq!(g.branches.len(), 2);
        assert_eq!(g.commits[1].branch, "feature");
    }

    #[test]
    fn parse_merge() {
        let input =
            "commit \"Init\"\nbranch feature\ncheckout feature\ncommit \"Work\"\ncheckout main\nmerge feature\n";
        let g = parse(input).unwrap();
        assert_eq!(g.merges.len(), 1);
        assert_eq!(g.commits.len(), 3); // init + work + merge commit
    }

    #[test]
    fn parse_tag() {
        let input = "commit \"Release\" tag \"v1.0\"\n";
        let g = parse(input).unwrap();
        assert_eq!(g.commits[0].tag.as_deref(), Some("v1.0"));
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_produces_svg() {
        let input = "commit \"Init\"\nbranch dev\ncheckout dev\ncommit \"Work\"\ncheckout main\nmerge dev\n";
        let g = parse(input).unwrap();
        let svg = render_svg(&g);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Init"));
    }
}
