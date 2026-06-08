use std::io::{self, Read};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Repo {
    owner: String,
    name: String,
    description: Option<String>,
    language: Option<String>,
    stars: Option<String>,
    forks: Option<String>,
    license: Option<String>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse(input: &str) -> Result<Repo, String> {
    let mut owner = String::new();
    let mut name = String::new();
    let mut description = None;
    let mut language = None;
    let mut stars = None;
    let mut forks = None;
    let mut license = None;

    for line in input.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        if t.starts_with("repo ") {
            let rest = sq(t.strip_prefix("repo ").unwrap().trim());
            if let Some((o, n)) = rest.split_once('/') {
                owner = o.trim().to_string();
                name = n.trim().to_string();
            } else {
                return Err(format!("Expected owner/name format: {}", rest));
            }
            continue;
        }
        if t.starts_with("desc ") {
            description = Some(sq(t.strip_prefix("desc ").unwrap().trim()).to_string());
            continue;
        }
        if t.starts_with("lang ") {
            language = Some(t.strip_prefix("lang ").unwrap().trim().to_string());
            continue;
        }
        if t.starts_with("stars ") {
            stars = Some(t.strip_prefix("stars ").unwrap().trim().to_string());
            continue;
        }
        if t.starts_with("forks ") {
            forks = Some(t.strip_prefix("forks ").unwrap().trim().to_string());
            continue;
        }
        if t.starts_with("license ") {
            license = Some(t.strip_prefix("license ").unwrap().trim().to_string());
            continue;
        }

        return Err(format!("Unknown syntax: {}", t));
    }

    if name.is_empty() {
        return Err("Missing 'repo owner/name'".to_string());
    }

    Ok(Repo {
        owner,
        name,
        description,
        language,
        stars,
        forks,
        license,
    })
}

fn sq(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CW: f64 = 8.0;
const CJK: f64 = 14.0;
const CARD_W: f64 = 480.0;
const CARD_H_PAD: f64 = 24.0;
const CARD_V_PAD: f64 = 20.0;
const CARD_RADIUS: f64 = 8.0;

const LANG_COLORS: &[(&str, &str)] = &[
    ("Rust", "#dea584"),
    ("JavaScript", "#f1e05a"),
    ("TypeScript", "#3178c6"),
    ("Python", "#3572a5"),
    ("Go", "#00add8"),
    ("Java", "#b07219"),
    ("C", "#555555"),
    ("C++", "#f34b7d"),
    ("Ruby", "#701516"),
    ("Swift", "#f05138"),
    ("Kotlin", "#a97bff"),
    ("Shell", "#89e051"),
    ("HTML", "#e34c26"),
    ("CSS", "#563d7c"),
];

fn lang_color(lang: &str) -> &str {
    LANG_COLORS
        .iter()
        .find(|(l, _)| l.eq_ignore_ascii_case(lang))
        .map(|(_, c)| *c)
        .unwrap_or("#999")
}

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

fn tw(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CW } else { CJK })
        .sum()
}

fn ex(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// SVG rendering
// ---------------------------------------------------------------------------

fn render_svg(repo: &Repo) -> String {
    let full_name = format!("{}/{}", repo.owner, repo.name);
    let desc_h = if repo.description.is_some() { 24.0 } else { 0.0 };
    let meta_h = 24.0;
    let card_h = CARD_V_PAD + 28.0 + desc_h + 16.0 + meta_h + CARD_V_PAD;

    let card_w = {
        let name_w = tw(&full_name) + 40.0;
        let desc_w = repo
            .description
            .as_ref()
            .map(|d| tw(d) * 0.93 + 40.0)
            .unwrap_or(0.0);
        name_w.max(desc_w).max(CARD_W)
    };

    let total_w = card_w + 2.0;
    let total_h = card_h + 2.0;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }</style>");

    let cx = 1.0;
    let cy = 1.0;

    // Card background
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"white\" stroke=\"#d0d7de\" stroke-width=\"1\"/>",
        cx, cy, card_w, card_h, CARD_RADIUS
    ));

    let mut y = cy + CARD_V_PAD;

    // Repo icon (book icon)
    let icon_x = cx + CARD_H_PAD;
    svg.push_str(&format!(
        "<g transform=\"translate({},{})\">",
        icon_x,
        y - 2.0
    ));
    svg.push_str("<rect x=\"0\" y=\"0\" width=\"16\" height=\"18\" rx=\"2\" fill=\"none\" stroke=\"#57606a\" stroke-width=\"1.5\"/>");
    svg.push_str("<line x1=\"5\" y1=\"0\" x2=\"5\" y2=\"18\" stroke=\"#57606a\" stroke-width=\"1.5\"/>");
    svg.push_str("</g>");

    // Owner / Name
    let name_x = icon_x + 22.0;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"16\"><tspan fill=\"#0969da\">{}</tspan><tspan fill=\"#57606a\"> / </tspan><tspan fill=\"#0969da\" font-weight=\"bold\">{}</tspan></text>",
        name_x,
        y + 14.0,
        ex(&repo.owner),
        ex(&repo.name)
    ));
    y += 28.0;

    // Description
    if let Some(ref desc) = repo.description {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"13\" fill=\"#57606a\">{}</text>",
            cx + CARD_H_PAD,
            y + 14.0,
            ex(desc)
        ));
        y += desc_h;
    }

    y += 16.0;

    // Meta line: language, stars, forks, license
    let mut mx = cx + CARD_H_PAD;

    if let Some(ref lang) = repo.language {
        let color = lang_color(lang);
        // Language dot
        svg.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>",
            mx + 5.0,
            y + 8.0,
            color
        ));
        mx += 14.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#57606a\">{}</text>",
            mx,
            y + 12.0,
            ex(lang)
        ));
        mx += tw(lang) * 0.92 + 16.0;
    }

    if let Some(ref stars) = repo.stars {
        // Star icon
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\">",
            mx,
            y + 2.0
        ));
        svg.push_str("<polygon points=\"6,0 7.8,4.5 12,4.5 8.7,7.2 9.9,12 6,9.3 2.1,12 3.3,7.2 0,4.5 4.2,4.5\" fill=\"none\" stroke=\"#57606a\" stroke-width=\"1\"/>");
        svg.push_str("</g>");
        mx += 16.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#57606a\">{}</text>",
            mx,
            y + 12.0,
            ex(stars)
        ));
        mx += tw(stars) * 0.92 + 16.0;
    }

    if let Some(ref forks) = repo.forks {
        // Fork icon
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\" fill=\"none\" stroke=\"#57606a\" stroke-width=\"1.2\">",
            mx, y + 2.0
        ));
        svg.push_str("<circle cx=\"4\" cy=\"3\" r=\"2\"/><circle cx=\"10\" cy=\"3\" r=\"2\"/><circle cx=\"7\" cy=\"11\" r=\"2\"/>");
        svg.push_str("<line x1=\"4\" y1=\"5\" x2=\"4\" y2=\"7\"/><line x1=\"10\" y1=\"5\" x2=\"10\" y2=\"7\"/>");
        svg.push_str("<path d=\"M4,7 Q4,9 7,9 Q10,9 10,7\"/>");
        svg.push_str("</g>");
        mx += 16.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#57606a\">{}</text>",
            mx,
            y + 12.0,
            ex(forks)
        ));
        mx += tw(forks) * 0.92 + 16.0;
    }

    if let Some(ref lic) = repo.license {
        // License icon
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\" fill=\"none\" stroke=\"#57606a\" stroke-width=\"1.2\">",
            mx, y + 1.0
        ));
        svg.push_str("<rect x=\"0\" y=\"2\" width=\"12\" height=\"10\" rx=\"2\"/>");
        svg.push_str("<line x1=\"3\" y1=\"5\" x2=\"9\" y2=\"5\"/><line x1=\"3\" y1=\"8\" x2=\"7\" y2=\"8\"/>");
        svg.push_str("</g>");
        mx += 16.0;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#57606a\">{}</text>",
            mx,
            y + 12.0,
            ex(lic)
        ));
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

    match parse(&input) {
        Ok(repo) => print!("{}", render_svg(&repo)),
        Err(e) => {
            eprintln!("mdd-github: {}", e);
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let input = "repo ppdx999/mdd\ndesc \"Markdown with Diagrams\"\n";
        let r = parse(input).unwrap();
        assert_eq!(r.owner, "ppdx999");
        assert_eq!(r.name, "mdd");
        assert_eq!(r.description.as_deref(), Some("Markdown with Diagrams"));
    }

    #[test]
    fn parse_full() {
        let input = "repo user/repo\ndesc \"A tool\"\nlang Rust\nstars 100\nforks 20\nlicense MIT\n";
        let r = parse(input).unwrap();
        assert_eq!(r.language.as_deref(), Some("Rust"));
        assert_eq!(r.stars.as_deref(), Some("100"));
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn render_output() {
        let input = "repo a/b\ndesc \"test\"\nlang Rust\nstars 10\n";
        let r = parse(input).unwrap();
        let svg = render_svg(&r);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains(">a<"));
        assert!(svg.contains(">b<"));
    }
}
