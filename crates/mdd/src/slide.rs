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
const TITLE_FONT_SIZE: f64 = 36.0;
const BODY_FONT_SIZE: f64 = 14.0;
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
        match block {
            Block::Text(text) => {
                let md_blocks = parse_markdown_blocks(text);
                content_h += md_blocks.iter().map(|b| compute_block_height(b, content_area)).sum::<f64>();
            }
            Block::Svg(svg) => {
                content_h += BLOCK_GAP;
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
                let md_blocks = parse_markdown_blocks(text);
                y = render_doc_blocks(&md_blocks, &mut svg, y, content_area, page_w);
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

pub fn build_slide_pdf_from_processed(processed: &str) -> Vec<u8> {
    let pages = split_pages(processed);
    if pages.is_empty() {
        return build_pdf(&[], 2.0);
    }
    build_pdf(&pages, 2.0)
}

// ---------------------------------------------------------------------------
// Markdown → SVG document renderer (pulldown_cmark-based)
// ---------------------------------------------------------------------------

const H1_FONT_SIZE: f64 = 36.0;
const H2_FONT_SIZE: f64 = 28.0;
const H3_FONT_SIZE: f64 = 18.0;
const H4_FONT_SIZE: f64 = 15.0;
const DOC_LINE_HEIGHT: f64 = 22.0;
const LIST_INDENT: f64 = 20.0;
const BULLET_RADIUS: f64 = 3.0;
const CODE_BG: &str = "#f5f5f5";
const CODE_FONT_SIZE: f64 = 12.0;
const CODE_LINE_HEIGHT: f64 = 18.0;
const CODE_PAD: f64 = 12.0;
const RULE_GAP: f64 = 16.0;
const TABLE_CELL_PAD: f64 = 8.0;
const TABLE_ROW_HEIGHT: f64 = 28.0;
const TABLE_FONT_SIZE: f64 = 12.0;
const TABLE_INNER_LINE_HEIGHT: f64 = 16.0;
const TABLE_HEADER_BG: &str = "#f0f0f0";
const CHAR_WIDTH_DOC: f64 = 8.0;
const CJK_CHAR_WIDTH_DOC: f64 = 14.0;
const BLOCKQUOTE_BAR_WIDTH: f64 = 4.0;
const BLOCKQUOTE_INDENT: f64 = 16.0;

fn doc_text_width(s: &str, font_size: f64) -> f64 {
    let scale = font_size / BODY_FONT_SIZE;
    s.chars()
        .map(|c| if c.is_ascii() { CHAR_WIDTH_DOC } else { CJK_CHAR_WIDTH_DOC })
        .sum::<f64>()
        * scale
}

fn wrap_text_lines(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    if text.is_empty() || max_width <= 0.0 {
        return vec![String::new()];
    }
    let scale = font_size / BODY_FONT_SIZE;
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0.0;

    for ch in text.chars() {
        let ch_width = (if ch.is_ascii() { CHAR_WIDTH_DOC } else { CJK_CHAR_WIDTH_DOC }) * scale;
        if current_width + ch_width > max_width && !current_line.is_empty() {
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0.0;
        }
        current_line.push(ch);
        current_width += ch_width;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

const MIN_COL_WIDTH: f64 = 40.0;

fn compute_col_widths(headers: &[String], rows: &[Vec<String>], content_area: f64, font_size: f64) -> Vec<f64> {
    let num_cols = headers.len().max(1);
    let mut max_widths: Vec<f64> = vec![0.0; num_cols];

    for (j, header) in headers.iter().enumerate() {
        max_widths[j] = max_widths[j].max(doc_text_width(header, font_size));
    }
    for row in rows {
        for (j, cell) in row.iter().enumerate() {
            if j < num_cols {
                max_widths[j] = max_widths[j].max(doc_text_width(cell, font_size));
            }
        }
    }

    let weights: Vec<f64> = max_widths.iter()
        .map(|w| (w + TABLE_CELL_PAD * 2.0).max(MIN_COL_WIDTH))
        .collect();
    let total: f64 = weights.iter().sum();

    weights.iter().map(|w| content_area * w / total).collect()
}

fn table_row_height(cells: &[String], col_widths: &[f64], font_size: f64) -> f64 {
    let max_lines = cells.iter().enumerate()
        .map(|(j, cell)| {
            let cw = col_widths.get(j).copied().unwrap_or(MIN_COL_WIDTH);
            wrap_text_lines(cell, cw - TABLE_CELL_PAD, font_size).len()
        })
        .max()
        .unwrap_or(1);
    if max_lines <= 1 {
        TABLE_ROW_HEIGHT
    } else {
        TABLE_ROW_HEIGHT + (max_lines as f64 - 1.0) * TABLE_INNER_LINE_HEIGHT
    }
}

/// Intermediate block produced from pulldown_cmark events
#[derive(Debug)]
enum DocBlock {
    Heading { level: u8, text: String },
    Paragraph { spans: Vec<Span> },
    List { ordered: bool, items: Vec<Vec<Span>> },
    CodeBlock { _lang: String, code: String },
    EmbeddedSvg(String),
    Rule,
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Blockquote { spans: Vec<Span> },
}

#[derive(Debug, Clone)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    _code: bool,
}

fn extract_svgs(input: &str) -> (String, Vec<String>) {
    let mut cleaned = String::new();
    let mut svgs: Vec<String> = Vec::new();
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
                let idx = svgs.len();
                svgs.push(svg_buf.clone());
                cleaned.push_str(&format!("<!--SVG_PLACEHOLDER_{}-->\n", idx));
                svg_buf.clear();
            }
            continue;
        }
        if in_svg {
            svg_buf.push_str(line);
            svg_buf.push('\n');
            if line.contains("</svg>") {
                in_svg = false;
                let idx = svgs.len();
                svgs.push(svg_buf.clone());
                cleaned.push_str(&format!("<!--SVG_PLACEHOLDER_{}-->\n", idx));
                svg_buf.clear();
            }
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    (cleaned, svgs)
}

fn parse_markdown_blocks(processed: &str) -> Vec<DocBlock> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, HeadingLevel};

    // Extract SVGs before feeding to pulldown_cmark
    let (cleaned, svgs) = extract_svgs(processed);

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(&cleaned, opts);

    let mut blocks: Vec<DocBlock> = Vec::new();
    let mut heading_level: Option<u8> = None;
    let mut heading_text = String::new();
    let mut in_paragraph = false;
    let mut paragraph_spans: Vec<Span> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut list_ordered = false;
    let mut list_items: Vec<Vec<Span>> = Vec::new();
    let mut current_item_spans: Vec<Span> = Vec::new();
    let mut in_item = false;
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut table_cell = String::new();
    let mut in_table_head = false;
    let mut in_blockquote = false;
    let mut blockquote_spans: Vec<Span> = Vec::new();

    for event in parser {
        match event {
            // Headings
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    _ => 4,
                });
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = heading_level.take() {
                    blocks.push(DocBlock::Heading { level, text: heading_text.clone() });
                }
            }

            // Paragraphs
            Event::Start(Tag::Paragraph) => {
                if in_blockquote {
                    // handled inside blockquote
                } else if in_item {
                    // handled inside list item
                } else {
                    in_paragraph = true;
                    paragraph_spans.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if in_blockquote || in_item {
                    // handled by parent
                } else if in_paragraph {
                    in_paragraph = false;
                    blocks.push(DocBlock::Paragraph { spans: paragraph_spans.clone() });
                    paragraph_spans.clear();
                }
            }

            // Emphasis
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,

            // Lists
            Event::Start(Tag::List(ordered)) => {
                list_ordered = ordered.is_some();
                list_items.clear();
            }
            Event::End(TagEnd::List(_)) => {
                blocks.push(DocBlock::List { ordered: list_ordered, items: list_items.clone() });
                list_items.clear();
            }
            Event::Start(Tag::Item) => {
                in_item = true;
                current_item_spans.clear();
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                list_items.push(current_item_spans.clone());
            }

            // Code blocks
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    _ => String::new(),
                };
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                blocks.push(DocBlock::CodeBlock { _lang: code_lang.clone(), code: code_content.clone() });
            }

            // Tables
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_headers.clear();
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                blocks.push(DocBlock::Table { headers: table_headers.clone(), rows: table_rows.clone() });
            }
            Event::Start(Tag::TableHead) => { in_table_head = true; }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
                table_headers = table_row.clone();
                table_row.clear();
            }
            Event::Start(Tag::TableRow) => { table_row.clear(); }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head {
                    table_rows.push(table_row.clone());
                }
                table_row.clear();
            }
            Event::Start(Tag::TableCell) => { table_cell.clear(); }
            Event::End(TagEnd::TableCell) => { table_row.push(table_cell.clone()); }

            // Blockquotes
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
                blockquote_spans.clear();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
                blocks.push(DocBlock::Blockquote { spans: blockquote_spans.clone() });
            }

            // Rule
            Event::Rule => { blocks.push(DocBlock::Rule); }

            // Inline code
            Event::Code(text) => {
                let span = Span { text: text.to_string(), bold, italic, _code: true };
                if heading_level.is_some() {
                    heading_text.push_str(&text);
                } else if in_item {
                    current_item_spans.push(span);
                } else if in_blockquote {
                    blockquote_spans.push(span);
                } else if in_paragraph {
                    paragraph_spans.push(span);
                }
            }

            // Text
            Event::Text(text) => {
                if in_code_block {
                    code_content.push_str(&text);
                } else if heading_level.is_some() {
                    heading_text.push_str(&text);
                } else if in_table {
                    table_cell.push_str(&text);
                } else if in_item {
                    current_item_spans.push(Span { text: text.to_string(), bold, italic, _code: false });
                } else if in_blockquote {
                    blockquote_spans.push(Span { text: text.to_string(), bold, italic, _code: false });
                } else if in_paragraph {
                    paragraph_spans.push(Span { text: text.to_string(), bold, italic, _code: false });
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                let br = Span { text: "\n".to_string(), bold: false, italic: false, _code: false };
                if in_item { current_item_spans.push(br); }
                else if in_blockquote { blockquote_spans.push(br); }
                else if in_paragraph { paragraph_spans.push(br); }
            }

            Event::Html(html) | Event::InlineHtml(html) => {
                let h = html.trim();
                // Check for SVG placeholder comments
                if h.starts_with("<!--SVG_PLACEHOLDER_") {
                    if let Some(idx_str) = h.strip_prefix("<!--SVG_PLACEHOLDER_")
                        .and_then(|s| s.strip_suffix("-->"))
                    {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Some(svg) = svgs.get(idx) {
                                blocks.push(DocBlock::EmbeddedSvg(svg.clone()));
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    blocks
}

fn heading_font_size(level: u8) -> f64 {
    match level {
        1 => H1_FONT_SIZE,
        2 => H2_FONT_SIZE,
        3 => H3_FONT_SIZE,
        _ => H4_FONT_SIZE,
    }
}

fn render_spans_width(spans: &[Span], font_size: f64) -> f64 {
    spans.iter().map(|s| doc_text_width(&s.text, font_size)).sum()
}

fn compute_block_height(block: &DocBlock, content_area: f64) -> f64 {
    match block {
        DocBlock::Heading { level, .. } => {
            let fs = heading_font_size(*level);
            fs + 8.0 + if *level <= 2 { 8.0 } else { 0.0 }
        }
        DocBlock::Paragraph { spans } => {
            let total_text: String = spans.iter().map(|s| s.text.clone()).collect();
            let line_count = total_text.lines().count().max(1) +
                // estimate wrap lines
                (render_spans_width(spans, BODY_FONT_SIZE) / content_area).floor() as usize;
            line_count as f64 * DOC_LINE_HEIGHT + BLOCK_GAP
        }
        DocBlock::List { items, .. } => {
            items.len() as f64 * DOC_LINE_HEIGHT + BLOCK_GAP
        }
        DocBlock::CodeBlock { code, .. } => {
            let lines = code.lines().count().max(1);
            CODE_PAD * 2.0 + lines as f64 * CODE_LINE_HEIGHT + BLOCK_GAP
        }
        DocBlock::EmbeddedSvg(svg) => {
            let (_, sh) = svg_dimensions(svg);
            sh + BLOCK_GAP
        }
        DocBlock::Rule => RULE_GAP * 2.0 + 1.0,
        DocBlock::Table { headers, rows } => {
            let col_widths = compute_col_widths(headers, rows, content_area, TABLE_FONT_SIZE);
            let header_h = table_row_height(headers, &col_widths, TABLE_FONT_SIZE);
            let rows_h: f64 = rows.iter()
                .map(|row| table_row_height(row, &col_widths, TABLE_FONT_SIZE))
                .sum();
            header_h + rows_h + BLOCK_GAP
        }
        DocBlock::Blockquote { spans } => {
            let total_text: String = spans.iter().map(|s| s.text.clone()).collect();
            let lines = total_text.lines().count().max(1);
            lines as f64 * DOC_LINE_HEIGHT + BLOCK_GAP
        }
    }
}

fn render_table_row_cells(
    svg: &mut String, cells: &[String], y: f64, col_widths: &[f64],
    content_area: f64, font_size: f64, bold: bool, fill: &str,
) -> f64 {
    let row_h = table_row_height(cells, col_widths, font_size);
    let mut cell_x_start = PAGE_PAD;
    for (j, cell) in cells.iter().enumerate() {
        let cw = col_widths.get(j).copied().unwrap_or(MIN_COL_WIDTH);
        let cell_content_w = cw - TABLE_CELL_PAD;
        let cell_x = cell_x_start + TABLE_CELL_PAD;
        let wrapped = wrap_text_lines(cell, cell_content_w, font_size);
        let weight = if bold { " font-weight=\"bold\"" } else { "" };
        for (k, line) in wrapped.iter().enumerate() {
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\"{}>{}</text>",
                cell_x,
                y + font_size + 4.0 + k as f64 * TABLE_INNER_LINE_HEIGHT,
                font_size, fill, weight, escape_xml(line)
            ));
        }
        cell_x_start += cw;
    }
    // Row border
    svg.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#eee\" stroke-width=\"1\"/>",
        PAGE_PAD, y + row_h, PAGE_PAD + content_area, y + row_h
    ));
    row_h
}

fn render_doc_blocks(blocks: &[DocBlock], svg: &mut String, mut y: f64, content_area: f64, page_width: f64) -> f64 {
    for block in blocks {
        match block {
            DocBlock::Heading { level, text } => {
                let fs = heading_font_size(*level);
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"bold\" fill=\"#1a1a1a\">{}</text>",
                    PAGE_PAD, y + fs * 0.85, fs, escape_xml(text)
                ));
                y += fs + 4.0;
                if *level <= 2 {
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
                        PAGE_PAD, y, page_width - PAGE_PAD, y
                    ));
                    y += 8.0;
                } else {
                    y += 4.0;
                }
            }

            DocBlock::Paragraph { spans } => {
                y = render_spans_to_svg(svg, spans, PAGE_PAD, y, content_area, BODY_FONT_SIZE, "#333");
                y += BLOCK_GAP;
            }

            DocBlock::List { ordered, items } => {
                for (i, item_spans) in items.iter().enumerate() {
                    let x = PAGE_PAD + LIST_INDENT;
                    if *ordered {
                        svg.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#333\">{}.</text>",
                            PAGE_PAD, y + BODY_FONT_SIZE * 0.85, BODY_FONT_SIZE, i + 1
                        ));
                    } else {
                        svg.push_str(&format!(
                            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#555\"/>",
                            PAGE_PAD + LIST_INDENT / 2.0, y + BODY_FONT_SIZE * 0.4, BULLET_RADIUS
                        ));
                    }
                    render_spans_to_svg(svg, item_spans, x, y, content_area - LIST_INDENT, BODY_FONT_SIZE, "#333");
                    y += DOC_LINE_HEIGHT;
                }
                y += BLOCK_GAP;
            }

            DocBlock::CodeBlock { code, .. } => {
                let lines = code.lines().count().max(1);
                let box_h = CODE_PAD * 2.0 + lines as f64 * CODE_LINE_HEIGHT;
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"{}\"/>",
                    PAGE_PAD, y, content_area, box_h, CODE_BG
                ));
                let mut cy = y + CODE_PAD;
                for line in code.lines() {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-family=\"monospace\" fill=\"#333\">{}</text>",
                        PAGE_PAD + CODE_PAD, cy + CODE_FONT_SIZE * 0.85, CODE_FONT_SIZE, escape_xml(line)
                    ));
                    cy += CODE_LINE_HEIGHT;
                }
                y += box_h + BLOCK_GAP;
            }

            DocBlock::EmbeddedSvg(raw_svg) => {
                let (sw, sh) = svg_dimensions(raw_svg);
                let inner = svg_inner(raw_svg);
                let scale = if sw > content_area { content_area / sw } else { 1.0 };
                let scaled_w = sw * scale;
                let scaled_h = sh * scale;
                let offset_x = (page_width - scaled_w) / 2.0;

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

            DocBlock::Rule => {
                y += RULE_GAP;
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd\" stroke-width=\"1\"/>",
                    PAGE_PAD, y, page_width - PAGE_PAD, y
                ));
                y += RULE_GAP + 1.0;
            }

            DocBlock::Table { headers, rows } => {
                let col_widths = compute_col_widths(headers, rows, content_area, TABLE_FONT_SIZE);

                // Header row background
                let header_h = table_row_height(headers, &col_widths, TABLE_FONT_SIZE);
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    PAGE_PAD, y, content_area, header_h, TABLE_HEADER_BG
                ));
                let h = render_table_row_cells(svg, headers, y, &col_widths, content_area, TABLE_FONT_SIZE, true, "#333");
                // Override header border to use darker color
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ccc\" stroke-width=\"1\"/>",
                    PAGE_PAD, y + h, PAGE_PAD + content_area, y + h
                ));
                y += h;

                // Data rows
                for row in rows {
                    let row_h = render_table_row_cells(svg, row, y, &col_widths, content_area, TABLE_FONT_SIZE, false, "#333");
                    y += row_h;
                }
                y += BLOCK_GAP;
            }

            DocBlock::Blockquote { spans } => {
                let text_x = PAGE_PAD + BLOCKQUOTE_BAR_WIDTH + BLOCKQUOTE_INDENT;
                let start_y = y;
                y = render_spans_to_svg(svg, spans, text_x, y, content_area - BLOCKQUOTE_INDENT - BLOCKQUOTE_BAR_WIDTH, BODY_FONT_SIZE, "#666");
                // Draw left bar
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ddd\" rx=\"2\"/>",
                    PAGE_PAD, start_y, BLOCKQUOTE_BAR_WIDTH, y - start_y
                ));
                y += BLOCK_GAP;
            }
        }
    }
    y
}

fn build_document_svg_from_markdown(processed: &str, fixed_width: f64) -> String {
    let content_area = fixed_width - PAGE_PAD * 2.0;
    let blocks = parse_markdown_blocks(processed);

    // Compute total height
    let total_h: f64 = PAGE_PAD * 2.0 + blocks.iter().map(|b| compute_block_height(b, content_area)).sum::<f64>();

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        fixed_width, total_h, fixed_width, total_h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    svg.push_str("<style>text { font-family: -apple-system, BlinkMacSystemFont, sans-serif; }</style>");

    let y = PAGE_PAD;
    render_doc_blocks(&blocks, &mut svg, y, content_area, fixed_width);

    svg.push_str("</svg>");
    svg
}

fn render_spans_to_svg(svg: &mut String, spans: &[Span], x: f64, y: f64, _max_width: f64, font_size: f64, fill: &str) -> f64 {
    let mut cy = y;
    // Flatten spans into lines
    let mut lines: Vec<Vec<&Span>> = vec![vec![]];
    for span in spans {
        if span.text == "\n" {
            lines.push(vec![]);
        } else {
            // Split span text by newlines
            let parts: Vec<&str> = span.text.split('\n').collect();
            for (i, _part) in parts.iter().enumerate() {
                if i > 0 {
                    lines.push(vec![]);
                }
                lines.last_mut().unwrap().push(span);
            }
        }
    }

    // Deduplicate: render each span once per line
    // Simpler approach: concatenate span text per line with style
    let full_text: String = spans.iter().map(|s| s.text.clone()).collect();
    let any_bold = spans.iter().any(|s| s.bold);
    let any_italic = spans.iter().any(|s| s.italic);

    for line in full_text.lines() {
        if line.is_empty() {
            cy += DOC_LINE_HEIGHT;
            continue;
        }
        let weight = if any_bold { " font-weight=\"bold\"" } else { "" };
        let style = if any_italic { " font-style=\"italic\"" } else { "" };
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\"{}{}>{}</text>",
            x, cy + font_size * 0.85, font_size, fill, weight, style, escape_xml(line)
        ));
        cy += DOC_LINE_HEIGHT;
    }
    cy
}

fn build_single_page_pdf(svg_data: &str, scale: f64) -> Vec<u8> {
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};

    let (svg_w, svg_h) = svg_dimensions(svg_data);
    let (pixels, pw, ph) = render_svg_to_pixels(svg_data, scale)
        .expect("Failed to render SVG");

    let mut pdf = Pdf::new();
    let catalog_ref = Ref::new(1);
    let pages_ref = Ref::new(2);
    let page_ref = Ref::new(3);
    let content_ref = Ref::new(4);
    let image_ref = Ref::new(5);

    pdf.catalog(catalog_ref).pages(pages_ref);

    let mut pages_obj = pdf.pages(pages_ref);
    pages_obj.count(1);
    pages_obj.kids([page_ref]);
    pages_obj.finish();

    let pdf_w = svg_w as f32;
    let pdf_h = svg_h as f32;

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

    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&pixels, 6);
    let mut image = pdf.image_xobject(image_ref, &compressed);
    image.filter(pdf_writer::Filter::FlateDecode);
    image.width(pw as i32);
    image.height(ph as i32);
    image.color_space().device_rgb();
    image.bits_per_component(8);
    image.finish();

    pdf.finish()
}

pub fn build_preview_pdf(path: &Path) -> Vec<u8> {
    let input = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mdd: Failed to read {}: {}", path.display(), e);
        std::process::exit(1);
    });

    let processed = crate::process::process(&input, path).unwrap_or_else(|e| {
        eprintln!("mdd: {}", e);
        std::process::exit(1);
    });

    let svg = build_document_svg_from_markdown(&processed, FIXED_PAGE_W);
    build_single_page_pdf(&svg, 2.0)
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
