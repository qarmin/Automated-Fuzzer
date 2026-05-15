use std::env::args;
use std::fs;
use std::path::Path;
use std::time::Duration;
use fuzz_utils::ByteInput;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path);
        }
    } else {
        check_file(&path);
    }
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else { return };
    let mut input = ByteInput::new(data);

    let _ = run_pdf_writer(&mut input);

    // Save params (human-readable key=value)
    let _ = input.save_params(&format!("{path}.params"));

    // Save standalone reproducer (Rust code with hardcoded values)
    let source_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs");
    if let Some(reproducer) = input.generate_source_reproducer(source_path) {
        let _ = fs::write(format!("{path}.reproducer.rs"), reproducer);
    }
}

fn run_pdf_writer(input: &mut ByteInput) -> Option<()> {
    let mut pdf = Pdf::new();

    let page_count = input.u8_range("page_count", 1, 20)? as i32;
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);

    // Build page refs starting from 3
    let page_ids: Vec<Ref> = (0..page_count).map(|i| Ref::new(3 + i)).collect();

    // Catalog
    pdf.catalog(catalog_id).pages(page_tree_id);

    // Page tree
    let mut pages = pdf.pages(page_tree_id);
    pages.count(page_count);
    pages.kids(page_ids.iter().copied());
    pages.finish();

    // Create each page
    for i in 0..page_count {
        std::thread::sleep(Duration::from_secs(20));
        panic!();
        if input.is_empty() {
            break;
        }

        let page_ref = Ref::new(3 + i);
        let content_ref = Ref::new(3 + page_count + i);

        let width = input.f32_range(&format!("page_{i}_width"), 1.0, 2000.0)?;
        let height = input.f32_range(&format!("page_{i}_height"), 1.0, 2000.0)?;

        let mut page = pdf.page(page_ref);
        page.parent(page_tree_id);
        page.media_box(Rect::new(0.0, 0.0, width, height));

        // Optional crop box
        if input.bool(&format!("page_{i}_has_crop"))? {
            let cx = input.f32_range(&format!("page_{i}_crop_x"), 0.0, width)?;
            let cy = input.f32_range(&format!("page_{i}_crop_y"), 0.0, height)?;
            page.crop_box(Rect::new(cx, cy, width, height));
        }

        // Optional rotation
        let rotations = [0i32, 90, 180, 270];
        let rot_idx = input.index(&format!("page_{i}_rotation"), rotations.len())?;
        if rotations[rot_idx] != 0 {
            page.rotate(rotations[rot_idx]);
        }

        page.contents(content_ref);

        // Resources
        let mut resources = page.resources();
        if input.bool(&format!("page_{i}_has_font"))? {
            resources
                .fonts()
                .pair(Name(b"F1"), Ref::new(100 + i));
        }
        resources.finish();
        page.finish();

        // Content stream
        let mut content = Content::new();

        let op_count = input.u8_range(&format!("page_{i}_ops"), 0, 30)?;
        for j in 0..op_count {
            if input.is_empty() {
                break;
            }

            let op_type = input.u8_range(&format!("page_{i}_op_{j}"), 0, 12)?;
            match op_type {
                0 => {
                    // Move to
                    let x = input.f32_range(&format!("p{i}_op{j}_x"), -1000.0, 3000.0)?;
                    let y = input.f32_range(&format!("p{i}_op{j}_y"), -1000.0, 3000.0)?;
                    content.move_to(x, y);
                }
                1 => {
                    // Line to
                    let x = input.f32_range(&format!("p{i}_op{j}_x"), -1000.0, 3000.0)?;
                    let y = input.f32_range(&format!("p{i}_op{j}_y"), -1000.0, 3000.0)?;
                    content.line_to(x, y);
                }
                2 => {
                    // Rectangle
                    let x = input.f32_range(&format!("p{i}_op{j}_x"), -500.0, 2000.0)?;
                    let y = input.f32_range(&format!("p{i}_op{j}_y"), -500.0, 2000.0)?;
                    let w = input.f32_range(&format!("p{i}_op{j}_w"), 0.0, 1000.0)?;
                    let h = input.f32_range(&format!("p{i}_op{j}_h"), 0.0, 1000.0)?;
                    content.rect(x, y, w, h);
                }
                3 => { content.stroke(); }
                4 => { content.fill_nonzero(); }
                5 => { content.fill_even_odd(); }
                6 => { content.close_path(); }
                7 => {
                    // Set line width
                    let w = input.f32_range(&format!("p{i}_op{j}_lw"), 0.0, 100.0)?;
                    content.set_line_width(w);
                }
                8 => {
                    // Set fill color (RGB)
                    let r = input.f32_range(&format!("p{i}_op{j}_r"), 0.0, 1.0)?;
                    let g = input.f32_range(&format!("p{i}_op{j}_g"), 0.0, 1.0)?;
                    let b = input.f32_range(&format!("p{i}_op{j}_b"), 0.0, 1.0)?;
                    content.set_fill_rgb(r, g, b);
                }
                9 => {
                    // Set stroke color
                    let r = input.f32_range(&format!("p{i}_op{j}_r"), 0.0, 1.0)?;
                    let g = input.f32_range(&format!("p{i}_op{j}_g"), 0.0, 1.0)?;
                    let b = input.f32_range(&format!("p{i}_op{j}_b"), 0.0, 1.0)?;
                    content.set_stroke_rgb(r, g, b);
                }
                10 => {
                    // Text
                    let font_size = input.f32_range(&format!("p{i}_op{j}_fs"), 1.0, 200.0)?;
                    let text_bytes = input.bytes(&format!("p{i}_op{j}_text"), 100)?;
                    content.begin_text();
                    content.set_font(Name(b"F1"), font_size);
                    let tx = input.f32_range(&format!("p{i}_op{j}_tx"), 0.0, width)?;
                    let ty = input.f32_range(&format!("p{i}_op{j}_ty"), 0.0, height)?;
                    content.next_line(tx, ty);
                    content.show(Str(&text_bytes));
                    content.end_text();
                }
                11 => {
                    // Cubic bezier
                    let x1 = input.f32_range(&format!("p{i}_op{j}_x1"), -500.0, 2000.0)?;
                    let y1 = input.f32_range(&format!("p{i}_op{j}_y1"), -500.0, 2000.0)?;
                    let x2 = input.f32_range(&format!("p{i}_op{j}_x2"), -500.0, 2000.0)?;
                    let y2 = input.f32_range(&format!("p{i}_op{j}_y2"), -500.0, 2000.0)?;
                    let x3 = input.f32_range(&format!("p{i}_op{j}_x3"), -500.0, 2000.0)?;
                    let y3 = input.f32_range(&format!("p{i}_op{j}_y3"), -500.0, 2000.0)?;
                    content.cubic_to(x1, y1, x2, y2, x3, y3);
                }
                12 => {
                    // Save/restore graphics state
                    content.save_state();
                    content.restore_state();
                }
                _ => {}
            }
        }

        pdf.stream(content_ref, &content.finish());
    }

    // Document info
    if !input.is_empty() {
        if input.bool("has_info")? {
            let info_ref = Ref::new(200);
            let mut info = pdf.document_info(info_ref);
            if let Some(title) = input.string("title", 50) {
                info.title(TextStr(&title));
            }
            if let Some(author) = input.string("author", 50) {
                info.author(TextStr(&author));
            }
            info.finish();
        }
    }

    // Finish and verify the output is valid bytes
    let output = pdf.finish();
    assert!(!output.is_empty(), "PDF output should not be empty");

    // Optionally write to temp file (verifies the full pipeline)
    let _ = fs::write("/tmp/fuzz_pdf_output.pdf", &output);

    Some(())
}
