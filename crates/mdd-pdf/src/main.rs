use std::io::{self, Read, Write};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Block {
    Text(String),
    Svg(String), // raw SVG content
}

#[derive(Debug)]
struct Page {
    title: String,
    blocks: Vec<Block>,
}

// ---------------------------------------------------------------------------
// Markdown splitter: split processed markdown into pages on "# " headings
// ---------------------------------------------------------------------------

fn split_pages(input: &str) -> Vec<Page> {
    let mut pages: Vec<Page> = Vec::new();
    let mut current_title = String::new();
    let mut current_blocks: Vec<Block> = Vec::new();
    let mut in_svg = false;
    let mut svg_buf = String::new();

    for line in input.lines() {
        // Track SVG blocks
        if !in_svg && line.trim_start().starts_with("<svg") {
            in_svg = true;
            svg_buf.clear();
            svg_buf.push_str(line);
            svg_buf.push('\n');
            if line.contains("</svg>") {
                in_svg = false;
                current_blocks.push(Block::Svg(svg_buf.clone()));
                svg_buf.clear();
            }
            continue;
        }
        if in_svg {
            svg_buf.push_str(line);
            svg_buf.push('\n');
            if line.contains("</svg>") {
                in_svg = false;
                current_blocks.push(Block::Svg(svg_buf.clone()));
                svg_buf.clear();
            }
            continue;
        }

        // Page break on "# " heading
        if line.starts_with("# ") {
            // Save previous page
            if !current_title.is_empty() || !current_blocks.is_empty() {
                pages.push(Page {
                    title: current_title,
                    blocks: current_blocks,
                });
            }
            current_title = line[2..].trim().to_string();
            current_blocks = Vec::new();
            continue;
        }

        // Regular text
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            // Merge consecutive text lines
            if let Some(Block::Text(text)) = current_blocks.last_mut() {
                text.push('\n');
                text.push_str(trimmed);
            } else {
                current_blocks.push(Block::Text(trimmed.to_string()));
            }
        } else if !current_blocks.is_empty() {
            // Empty line = paragraph break, start new text block
            current_blocks.push(Block::Text(String::new()));
        }
    }

    // Save last page
    if !current_title.is_empty() || !current_blocks.is_empty() {
        pages.push(Page {
            title: current_title,
            blocks: current_blocks,
        });
    }

    // Remove empty text blocks
    for page in &mut pages {
        page.blocks.retain(|b| match b {
            Block::Text(t) => !t.is_empty(),
            Block::Svg(_) => true,
        });
    }

    pages
}

// ---------------------------------------------------------------------------
// Composite SVG builder: lay out a page as a single SVG
// ---------------------------------------------------------------------------

const PAGE_PAD: f64 = 40.0;
const TITLE_FONT_SIZE: f64 = 28.0;
const BODY_FONT_SIZE: f64 = 14.0;
const BODY_LINE_HEIGHT: f64 = 22.0;
const BLOCK_GAP: f64 = 20.0;
const MIN_PAGE_W: f64 = 600.0;
const TITLE_BOTTOM_PAD: f64 = 16.0;

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract width and height from an SVG string
fn svg_dimensions(svg: &str) -> (f64, f64) {
    let parse_attr = |attr: &str| -> Option<f64> {
        let pattern = format!("{}=\"", attr);
        svg.find(&pattern).and_then(|start| {
            let rest = &svg[start + pattern.len()..];
            rest.find('"').and_then(|end| rest[..end].parse::<f64>().ok())
        })
    };

    let w = parse_attr("width").unwrap_or(400.0);
    let h = parse_attr("height").unwrap_or(300.0);
    (w, h)
}

/// Extract the inner content of an SVG (everything between <svg ...> and </svg>)
fn svg_inner(svg: &str) -> &str {
    let start = svg.find('>').map(|i| i + 1).unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    &svg[start..end]
}

fn build_page_svg(page: &Page) -> String {
    // First pass: compute dimensions
    let mut content_w: f64 = MIN_PAGE_W;
    let mut content_h: f64 = 0.0;

    // Title
    if !page.title.is_empty() {
        content_h += TITLE_FONT_SIZE + TITLE_BOTTOM_PAD;
    }

    for block in &page.blocks {
        content_h += BLOCK_GAP;
        match block {
            Block::Text(text) => {
                let lines = text.lines().count().max(1);
                content_h += lines as f64 * BODY_LINE_HEIGHT;
            }
            Block::Svg(svg) => {
                let (sw, sh) = svg_dimensions(svg);
                content_w = content_w.max(sw);
                content_h += sh;
            }
        }
    }

    let page_w = content_w + PAGE_PAD * 2.0;
    let page_h = content_h + PAGE_PAD * 2.0;

    // Second pass: build SVG
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        page_w, page_h, page_w, page_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: -apple-system, BlinkMacSystemFont, sans-serif; }</style>");

    let mut y = PAGE_PAD;

    // Title
    if !page.title.is_empty() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"#1a1a1a\">{}</text>",
            PAGE_PAD,
            y + TITLE_FONT_SIZE * 0.85,
            TITLE_FONT_SIZE,
            escape_xml(&page.title)
        ));
        y += TITLE_FONT_SIZE + TITLE_BOTTOM_PAD;
        // Title underline
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
            PAGE_PAD, y, page_w - PAGE_PAD, y
        ));
        y += BLOCK_GAP;
    }

    // Blocks
    for block in &page.blocks {
        match block {
            Block::Text(text) => {
                for line in text.lines() {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#333\">{}</text>",
                        PAGE_PAD,
                        y + BODY_FONT_SIZE * 0.85,
                        BODY_FONT_SIZE,
                        escape_xml(line)
                    ));
                    y += BODY_LINE_HEIGHT;
                }
                y += BLOCK_GAP;
            }
            Block::Svg(raw_svg) => {
                let (sw, sh) = svg_dimensions(raw_svg);
                let inner = svg_inner(raw_svg);
                // Center the SVG horizontally
                let offset_x = (page_w - sw) / 2.0;
                svg.push_str(&format!(
                    "<g transform=\"translate({},{})\">",
                    offset_x, y
                ));
                svg.push_str(inner);
                svg.push_str("</g>");
                y += sh + BLOCK_GAP;
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// SVG → PDF using resvg + pdf-writer
// ---------------------------------------------------------------------------

fn render_svg_to_pixels(svg_data: &str, scale: f64) -> Option<(Vec<u8>, u32, u32)> {
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_data, &opts).ok()?;
    let size = tree.size();
    let w = (size.width() as f64 * scale) as u32;
    let h = (size.height() as f64 * scale) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA to RGB (PDF doesn't need alpha on white background)
    let rgba = pixmap.data();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in rgba.chunks(4) {
        let a = chunk[3] as f64 / 255.0;
        // Blend with white background
        rgb.push((chunk[0] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[1] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[2] as f64 * a + 255.0 * (1.0 - a)) as u8);
    }

    Some((rgb, w, h))
}

fn build_pdf(pages: &[Page], scale: f64) -> Vec<u8> {
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};

    // Render all pages to pixels first
    let rendered: Vec<(Vec<u8>, u32, u32, f64, f64)> = pages
        .iter()
        .map(|page| {
            let svg = build_page_svg(page);
            let (sw, sh) = svg_dimensions(&svg);
            let (pixels, pw, ph) = render_svg_to_pixels(&svg, scale)
                .expect("Failed to render SVG");
            (pixels, pw, ph, sw, sh)
        })
        .collect();

    let mut pdf = Pdf::new();
    let catalog_ref = Ref::new(1);
    let pages_ref = Ref::new(2);

    // Reserve refs: for each page we need page_ref, content_ref, image_ref
    let mut next_ref = 3u32;
    let page_data: Vec<(Ref, Ref, Ref)> = rendered
        .iter()
        .map(|_| {
            let page_ref = Ref::new(next_ref as i32);
            let content_ref = Ref::new(next_ref as i32 + 1);
            let image_ref = Ref::new(next_ref as i32 + 2);
            next_ref += 3;
            (page_ref, content_ref, image_ref)
        })
        .collect();

    // Catalog
    pdf.catalog(catalog_ref).pages(pages_ref);

    // Pages
    let page_refs: Vec<Ref> = page_data.iter().map(|(p, _, _)| *p).collect();
    let mut pages_obj = pdf.pages(pages_ref);
    pages_obj.count(rendered.len() as i32);
    pages_obj.kids(page_refs.iter().copied());
    pages_obj.finish();

    // Each page
    for (i, (pixels, pw, ph, svg_w, svg_h)) in rendered.iter().enumerate() {
        let (page_ref, content_ref, image_ref) = page_data[i];

        // Page dimensions in PDF points (1 point = 1/72 inch)
        // SVG dimensions are our "points"
        let pdf_w = *svg_w as f32;
        let pdf_h = *svg_h as f32;

        // Page object
        let mut page = pdf.page(page_ref);
        page.parent(pages_ref);
        page.media_box(Rect::new(0.0, 0.0, pdf_w, pdf_h));
        page.contents(content_ref);

        let image_name = Name(b"Im1");
        page.resources()
            .x_objects()
            .pair(image_name, image_ref);
        page.finish();

        // Content stream: draw image filling the page
        let mut content = Content::new();
        content.save_state();
        // PDF images are drawn in a 1x1 unit square, so we scale to page size
        // Also flip Y axis (PDF origin is bottom-left)
        content.transform([pdf_w, 0.0, 0.0, pdf_h, 0.0, 0.0]);
        content.x_object(image_name);
        content.restore_state();
        let content_data = content.finish();
        pdf.stream(content_ref, &content_data);

        // Image XObject
        let compressed = miniz_oxide::deflate::compress_to_vec(pixels, 6);

        let mut image = pdf.image_xobject(image_ref, &compressed);
        image.filter(pdf_writer::Filter::FlateDecode);
        image.width(*pw as i32);
        image.height(*ph as i32);
        image.color_space().device_rgb();
        image.bits_per_component(8);
        image.finish();
    }

    pdf.finish()
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let pages = split_pages(&input);

    if pages.is_empty() {
        eprintln!("mdd-pdf: No pages found (use '# Title' to create pages)");
        std::process::exit(1);
    }

    // Scale factor for rendering (2.0 = retina quality)
    let scale = 2.0;
    let pdf_bytes = build_pdf(&pages, scale);

    io::stdout()
        .write_all(&pdf_bytes)
        .expect("Failed to write PDF");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_single_page() {
        let input = "# Hello\n\nSome text here.\n";
        let pages = split_pages(input);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Hello");
    }

    #[test]
    fn split_multiple_pages() {
        let input = "# Page 1\n\nText 1\n\n# Page 2\n\nText 2\n\n# Page 3\n\nText 3\n";
        let pages = split_pages(input);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].title, "Page 1");
        assert_eq!(pages[2].title, "Page 3");
    }

    #[test]
    fn split_with_svg() {
        let input = "# Test\n\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"><rect fill=\"red\"/></svg>\n";
        let pages = split_pages(input);
        assert_eq!(pages.len(), 1);
        assert!(matches!(pages[0].blocks[0], Block::Svg(_)));
    }

    #[test]
    fn svg_dimensions_parsing() {
        let svg = "<svg width=\"400\" height=\"300\">";
        let (w, h) = svg_dimensions(svg);
        assert_eq!(w, 400.0);
        assert_eq!(h, 300.0);
    }

    #[test]
    fn build_page_svg_output() {
        let page = Page {
            title: "Test".to_string(),
            blocks: vec![Block::Text("Hello world".to_string())],
        };
        let svg = build_page_svg(&page);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Test"));
        assert!(svg.contains("Hello world"));
    }
}
