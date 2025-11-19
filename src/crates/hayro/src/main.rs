use std::env::args;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use hayro::{render, InterpreterSettings, Pdf, RenderSettings};
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    let save_path = args().nth(2);
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_string_lossy().to_string();
            check_file(&path, save_path.as_deref());
        }
    } else {
        check_file(&path, save_path.as_deref());
    }
}

fn check_file(file_path: &str, save_path: Option<&str>) {
    let Ok(content) = fs::read(file_path) else {
        eprintln!("Failed to read file: {file_path}");
        return;
    };

    println!("Checking file: {file_path}");

    let data = Arc::new(content);
    let pdf = match Pdf::new(data) {
        Ok(pdf) => pdf,
        Err(e) => {
            eprintln!("Failed to parse PDF {file_path}: {:?}", e);
            return;
        }
    };

    // Iterate through all PDF objects (forces parsing)
    let _ = pdf.version();
    let _ = pdf.len();
    for _obj in pdf.objects() {}

    // Access XRef (cross-reference table)
    let xref = pdf.xref();
    let _ = xref.root_id();
    let _ = xref.has_optional_content_groups();

    // Process each page
    for (idx, page) in pdf.pages().iter().enumerate() {
        let page_num = idx + 1;

        // Get page information (forces parsing)
        let _ = page.base_dimensions();
        let _ = page.render_dimensions();
        let _ = page.rotation();
        let _ = page.media_box();
        let _ = page.intersected_crop_box();

        // Access page stream (forces stream parsing)
        let _ = page.page_stream();

        // Access raw page dictionary
        let _ = page.raw();

        // Access resources
        let resources = page.resources();
        let _ = resources.parent();

        // Parse all operations (computational)
        for _op in page.operations() {}

        // Parse typed operations (more computational)
        for _typed_op in page.typed_operations() {}

        // Render the page
        let pixmap = render(page, &InterpreterSettings::default(), &RenderSettings::default());

        if let Some(save_path) = save_path {
            match pixmap.into_png() {
                Ok(png) => {
                    let output_path = format!("{save_path}_{}.png", page_num);
                    if let Err(e) = fs::write(&output_path, png) {
                        eprintln!("Failed to write PNG {}: {}", output_path, e);
                    }
                }
                Err(e) => eprintln!("Failed to encode PNG for page {}: {:?}", page_num, e),
            }
        }
    }

    println!("OK: {file_path}");
}
