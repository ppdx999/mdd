use std::io::{self, Read};

#[derive(Debug)]
struct Card {
    text: String,
    label: Option<String>,
}

#[derive(Debug)]
struct Column {
    name: String,
    cards: Vec<Card>,
}

#[derive(Debug)]
struct Board {
    columns: Vec<Column>,
}

fn parse(input: &str) -> Result<Board, String> {
    let mut columns: Vec<Column> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if trimmed.starts_with("column ") {
            let name = strip_quotes(trimmed.strip_prefix("column ").unwrap().trim()).to_string();
            columns.push(Column { name, cards: Vec::new() });
            continue;
        }
        if trimmed.starts_with("card ") {
            let rest = trimmed.strip_prefix("card ").unwrap().trim();
            if let Some(col) = columns.last_mut() {
                if let Some((text, label)) = rest.split_once(" : ") {
                    col.cards.push(Card {
                        text: strip_quotes(text.trim()).to_string(),
                        label: Some(strip_quotes(label.trim()).to_string()),
                    });
                } else {
                    col.cards.push(Card {
                        text: strip_quotes(rest).to_string(),
                        label: None,
                    });
                }
            } else {
                return Err("Card before any column".to_string());
            }
            continue;
        }
        return Err(format!("Unknown syntax: {}", trimmed));
    }
    if columns.is_empty() { return Err("At least 1 column required".to_string()); }
    Ok(Board { columns })
}

fn strip_quotes(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len()-1] } else { s }
}

const CHAR_W: f64 = 8.0;
const CJK_W: f64 = 14.0;
const COL_W: f64 = 180.0;
const COL_GAP: f64 = 12.0;
const COL_PAD: f64 = 10.0;
const COL_HEADER_H: f64 = 32.0;
const CARD_H: f64 = 40.0;
const CARD_LABEL_H: f64 = 52.0;
const CARD_GAP: f64 = 8.0;
const PADDING: f64 = 24.0;

const COL_COLORS: &[&str] = &["#f0f0f0", "#f0f0f0", "#f0f0f0", "#f0f0f0"];
const LABEL_COLORS: &[(&str, &str)] = &[
    ("#e3f2fd", "#1565c0"), ("#e8f5e9", "#2e7d32"), ("#fff8e1", "#f57f17"),
    ("#fce4ec", "#c62828"), ("#f3e5f5", "#7b1fa2"), ("#e0f2f1", "#00695c"),
];

fn text_width(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii() { CHAR_W } else { CJK_W }).sum()
}
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn render_svg(board: &Board) -> String {
    let n = board.columns.len();
    let max_cards = board.columns.iter().map(|c| c.cards.len()).max().unwrap_or(0);
    let col_w = board.columns.iter()
        .flat_map(|c| c.cards.iter().map(|card| text_width(&card.text) + 24.0))
        .fold(COL_W, f64::max);

    let max_col_h = COL_HEADER_H + COL_PAD + max_cards as f64 * (CARD_LABEL_H + CARD_GAP);
    let total_w = PADDING * 2.0 + n as f64 * col_w + (n - 1) as f64 * COL_GAP;
    let total_h = PADDING * 2.0 + max_col_h + COL_PAD;

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        total_w, total_h, total_w, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: sans-serif; font-size: 13px; fill: #333; }</style>");

    let content_y = PADDING;

    let mut x = PADDING;
    for (ci, col) in board.columns.iter().enumerate() {
        let col_h = COL_HEADER_H + COL_PAD + col.cards.len() as f64 * (CARD_LABEL_H + CARD_GAP) + COL_PAD;
        let bg = COL_COLORS[ci % COL_COLORS.len()];
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\"/>",
            x, content_y, col_w, col_h, bg
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-weight=\"bold\" font-size=\"13\">{}</text>",
            x + COL_PAD, content_y + COL_HEADER_H / 2.0 + 5.0, escape_xml(&col.name)
        ));

        let mut cy = content_y + COL_HEADER_H + COL_PAD;
        for (j, card) in col.cards.iter().enumerate() {
            let ch = if card.label.is_some() { CARD_LABEL_H } else { CARD_H };
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"white\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
                x + COL_PAD, cy, col_w - COL_PAD * 2.0, ch
            ));
            if let Some(ref label) = card.label {
                let (lbg, lfg) = LABEL_COLORS[j % LABEL_COLORS.len()];
                let lw = text_width(label) * 0.85 + 12.0;
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"14\" rx=\"3\" fill=\"{}\"/>",
                    x + COL_PAD + 8.0, cy + 6.0, lw, lbg
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"{}\">{}</text>",
                    x + COL_PAD + 14.0, cy + 16.0, lfg, escape_xml(label)
                ));
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>",
                    x + COL_PAD + 8.0, cy + 36.0, escape_xml(&card.text)
                ));
            } else {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>",
                    x + COL_PAD + 8.0, cy + ch / 2.0 + 4.0, escape_xml(&card.text)
                ));
            }
            cy += ch + CARD_GAP;
        }
        x += col_w + COL_GAP;
    }
    svg.push_str("</svg>");
    svg
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");
    match parse(&input) {
        Ok(b) => print!("{}", render_svg(&b)),
        Err(e) => { eprintln!("mdd-kanban: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let input = "column Todo\ncard Task1\ncolumn Done\ncard Task2\n";
        let b = parse(input).unwrap();
        assert_eq!(b.columns.len(), 2);
        assert_eq!(b.columns[0].cards[0].text, "Task1");
    }
    #[test]
    fn parse_with_labels() {
        let input = "column Todo\ncard Fix bug : \"urgent\"\n";
        let b = parse(input).unwrap();
        assert_eq!(b.columns[0].cards[0].label.as_deref(), Some("urgent"));
    }
    #[test]
    fn render_svg_output() {
        let input = "column A\ncard X\n";
        let b = parse(input).unwrap();
        let svg = render_svg(&b);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
