use std::io::Write;
use std::path::Path;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Block {
    Text(String),
    Svg(String),
}

#[derive(Debug)]
struct Page {
    title: String,
    blocks: Vec<Block>,
}

// ---------------------------------------------------------------------------
// Markdown splitter
// ---------------------------------------------------------------------------

fn split_pages(input: &str) -> Vec<Page> {
    let mut pages: Vec<Page> = Vec::new();
    let mut current_title = String::new();
    let mut current_blocks: Vec<Block> = Vec::new();
    let mut in_svg = false;
    let mut svg_buf = String::new();

    for line in input.lines() {
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

        if line.starts_with("# ") {
            if !current_title.is_empty() || !current_blocks.is_empty() {
                pages.push(Page { title: current_title, blocks: current_blocks });
            }
            current_title = line[2..].trim().to_string();
            current_blocks = Vec::new();
            continue;
        }

        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Some(Block::Text(text)) = current_blocks.last_mut() {
                text.push('\n');
                text.push_str(trimmed);
            } else {
                current_blocks.push(Block::Text(trimmed.to_string()));
            }
        } else if !current_blocks.is_empty() {
            current_blocks.push(Block::Text(String::new()));
        }
    }

    if !current_title.is_empty() || !current_blocks.is_empty() {
        pages.push(Page { title: current_title, blocks: current_blocks });
    }

    for page in &mut pages {
        page.blocks.retain(|b| match b {
            Block::Text(t) => !t.is_empty(),
            Block::Svg(_) => true,
        });
    }

    pages
}

// ---------------------------------------------------------------------------
// Composite SVG builder
// ---------------------------------------------------------------------------

const PAGE_PAD: f64 = 40.0;
const TITLE_FONT_SIZE: f64 = 28.0;
const BODY_FONT_SIZE: f64 = 14.0;
const BODY_LINE_HEIGHT: f64 = 22.0;
const BLOCK_GAP: f64 = 20.0;
const FIXED_PAGE_W: f64 = 800.0;
const MIN_PAGE_H: f64 = 680.0;
const TITLE_BOTTOM_PAD: f64 = 16.0;

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn svg_dimensions(svg: &str) -> (f64, f64) {
    let parse_attr = |attr: &str| -> Option<f64> {
        let pattern = format!("{}=\"", attr);
        svg.find(&pattern).and_then(|start| {
            let rest = &svg[start + pattern.len()..];
            rest.find('"').and_then(|end| rest[..end].parse::<f64>().ok())
        })
    };
    (parse_attr("width").unwrap_or(400.0), parse_attr("height").unwrap_or(300.0))
}

fn svg_inner(svg: &str) -> &str {
    let start = svg.find('>').map(|i| i + 1).unwrap_or(0);
    let end = svg.rfind("</svg>").unwrap_or(svg.len());
    &svg[start..end]
}

fn build_page_svg(page: &Page, fixed_width: f64) -> String {
    let page_w = fixed_width;
    let content_area = page_w - PAGE_PAD * 2.0;

    // Compute content height
    let mut content_h: f64 = 0.0;
    if !page.title.is_empty() {
        content_h += TITLE_FONT_SIZE + TITLE_BOTTOM_PAD;
    }
    for block in &page.blocks {
        content_h += BLOCK_GAP;
        match block {
            Block::Text(text) => { content_h += text.lines().count().max(1) as f64 * BODY_LINE_HEIGHT; }
            Block::Svg(svg) => {
                let (sw, sh) = svg_dimensions(svg);
                let scale = if sw > content_area { content_area / sw } else { 1.0 };
                content_h += sh * scale;
            }
        }
    }

    let natural_h = content_h + PAGE_PAD * 2.0;
    let page_h = natural_h.max(MIN_PAGE_H);

    // Vertical centering: if page is taller than content, offset y
    let y_offset = if page_h > natural_h {
        (page_h - natural_h) / 2.0
    } else {
        0.0
    };

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        page_w, page_h, page_w, page_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: -apple-system, BlinkMacSystemFont, sans-serif; }</style>");

    let mut y = PAGE_PAD + y_offset;

    if !page.title.is_empty() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"#1a1a1a\">{}</text>",
            PAGE_PAD, y + TITLE_FONT_SIZE * 0.85, TITLE_FONT_SIZE, escape_xml(&page.title)
        ));
        y += TITLE_FONT_SIZE + TITLE_BOTTOM_PAD;
        svg.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
            PAGE_PAD, y, page_w - PAGE_PAD, y
        ));
        y += BLOCK_GAP;
    }

    for block in &page.blocks {
        match block {
            Block::Text(text) => {
                for line in text.lines() {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#333\">{}</text>",
                        PAGE_PAD, y + BODY_FONT_SIZE * 0.85, BODY_FONT_SIZE, escape_xml(line)
                    ));
                    y += BODY_LINE_HEIGHT;
                }
                y += BLOCK_GAP;
            }
            Block::Svg(raw_svg) => {
                let (sw, sh) = svg_dimensions(raw_svg);
                let inner = svg_inner(raw_svg);
                let scale = if sw > content_area { content_area / sw } else { 1.0 };
                let scaled_w = sw * scale;
                let scaled_h = sh * scale;
                let offset_x = (page_w - scaled_w) / 2.0;

                if (scale - 1.0).abs() < 0.001 {
                    svg.push_str(&format!("<g transform=\"translate({},{})\">", offset_x, y));
                } else {
                    svg.push_str(&format!(
                        "<g transform=\"translate({},{}) scale({})\">",
                        offset_x, y, scale
                    ));
                }
                svg.push_str(inner);
                svg.push_str("</g>");
                y += scaled_h + BLOCK_GAP;
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// SVG → PDF
// ---------------------------------------------------------------------------

fn render_svg_to_pixels(svg_data: &str, scale: f64) -> Option<(Vec<u8>, u32, u32)> {
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let opts = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg_data, &opts).ok()?;
    let size = tree.size();
    let w = (size.width() as f64 * scale) as u32;
    let h = (size.height() as f64 * scale) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = pixmap.data();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for chunk in rgba.chunks(4) {
        let a = chunk[3] as f64 / 255.0;
        rgb.push((chunk[0] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[1] as f64 * a + 255.0 * (1.0 - a)) as u8);
        rgb.push((chunk[2] as f64 * a + 255.0 * (1.0 - a)) as u8);
    }

    Some((rgb, w, h))
}

fn build_pdf(pages: &[Page], scale: f64) -> Vec<u8> {
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};

    let rendered: Vec<(Vec<u8>, u32, u32, f64, f64)> = pages
        .iter()
        .map(|page| {
            let svg = build_page_svg(page, FIXED_PAGE_W);
            let (sw, sh) = svg_dimensions(&svg);
            let (pixels, pw, ph) = render_svg_to_pixels(&svg, scale)
                .expect("Failed to render SVG");
            (pixels, pw, ph, sw, sh)
        })
        .collect();

    let mut pdf = Pdf::new();
    let catalog_ref = Ref::new(1);
    let pages_ref = Ref::new(2);

    let mut next_ref = 3u32;
    let page_data: Vec<(Ref, Ref, Ref)> = rendered
        .iter()
        .map(|_| {
            let r = (Ref::new(next_ref as i32), Ref::new(next_ref as i32 + 1), Ref::new(next_ref as i32 + 2));
            next_ref += 3;
            r
        })
        .collect();

    pdf.catalog(catalog_ref).pages(pages_ref);

    let page_refs: Vec<Ref> = page_data.iter().map(|(p, _, _)| *p).collect();
    let mut pages_obj = pdf.pages(pages_ref);
    pages_obj.count(rendered.len() as i32);
    pages_obj.kids(page_refs.iter().copied());
    pages_obj.finish();

    for (i, (pixels, pw, ph, svg_w, svg_h)) in rendered.iter().enumerate() {
        let (page_ref, content_ref, image_ref) = page_data[i];
        let pdf_w = *svg_w as f32;
        let pdf_h = *svg_h as f32;

        let mut page = pdf.page(page_ref);
        page.parent(pages_ref);
        page.media_box(Rect::new(0.0, 0.0, pdf_w, pdf_h));
        page.contents(content_ref);
        let image_name = Name(b"Im1");
        page.resources().x_objects().pair(image_name, image_ref);
        page.finish();

        let mut content = Content::new();
        content.save_state();
        content.transform([pdf_w, 0.0, 0.0, pdf_h, 0.0, 0.0]);
        content.x_object(image_name);
        content.restore_state();
        pdf.stream(content_ref, &content.finish());

        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(pixels, 6);
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
// Public entry points
// ---------------------------------------------------------------------------

fn build_slide_pdf(path: &Path) -> Vec<u8> {
    let input = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to read {}: {}", path.display(), e);
        std::process::exit(1);
    });

    let processed = crate::process::process(&input, path).unwrap_or_else(|e| {
        eprintln!("mdd: {}", e);
        std::process::exit(1);
    });

    let pages = split_pages(&processed);

    if pages.is_empty() {
        eprintln!("mdd: No slides found (use '# Title' to create slides)");
        std::process::exit(1);
    }

    build_pdf(&pages, 2.0)
}

pub fn generate_slide(path: &Path) {
    let pdf_bytes = build_slide_pdf(path);
    std::io::stdout()
        .write_all(&pdf_bytes)
        .expect("Failed to write PDF");
}

pub fn preview_slide(path: &Path) {
    use std::thread;
    use std::time::Duration;

    let pdf_path = path.with_extension("pdf");

    let pdf_bytes = build_slide_pdf(path);
    std::fs::write(&pdf_path, &pdf_bytes).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to write {}: {}", pdf_path.display(), e);
        std::process::exit(1);
    });
    eprintln!("mdd: Built {}", pdf_path.display());

    if let Err(e) = open::that(&pdf_path) {
        eprintln!("mdd: Failed to open PDF viewer: {}", e);
    }

    let mut last_modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    eprintln!(
        "mdd: Watching {} for changes... (Ctrl+C to stop)",
        path.display()
    );

    loop {
        thread::sleep(Duration::from_secs(1));
        let current = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        if current != last_modified {
            last_modified = current;
            let pdf_bytes = build_slide_pdf(path);
            if let Err(e) = std::fs::write(&pdf_path, &pdf_bytes) {
                eprintln!("mdd: Failed to write {}: {}", pdf_path.display(), e);
            } else {
                eprintln!("mdd: Rebuilt {}", pdf_path.display());
            }
        }
    }
}
