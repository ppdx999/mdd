const CHAR_WIDTH: f64 = 8.0;
const CJK_CHAR_WIDTH: f64 = 14.0;

/// Calculate the approximate pixel width of a string,
/// using CHAR_WIDTH for ASCII and CJK_CHAR_WIDTH for CJK characters.
pub fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH } else { CJK_CHAR_WIDTH })
        .sum()
}

/// Escape special XML characters for safe embedding in SVG.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_width() {
        assert!((text_width("abc") - 24.0).abs() < 0.01);
    }

    #[test]
    fn cjk_width() {
        assert!((text_width("あ") - 14.0).abs() < 0.01);
    }

    #[test]
    fn escape() {
        assert_eq!(escape_xml("<a>&\"b\"</a>"), "&lt;a&gt;&amp;&quot;b&quot;&lt;/a&gt;");
    }
}
