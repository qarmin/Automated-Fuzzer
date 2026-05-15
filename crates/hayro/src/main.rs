use std::fs;
use std::sync::Arc;

use hayro::hayro_interpret::font::FontQuery;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::{render, RenderCache, RenderSettings};

fn main() {
    let save_path = std::env::args().nth(2);
    fuzz_utils::run(|path| check_file(path, save_path.as_deref()));
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

    // Exercise document-level APIs
    let _ = pdf.version();
    let _ = pdf.len();
    for _obj in pdf.objects() {}

    // Access XRef (cross-reference table)
    let xref = pdf.xref();
    let _ = xref.root_id();
    let _ = xref.has_optional_content_groups();

    // Interpreter settings with font resolver
    let interp_with_fonts = InterpreterSettings {
        font_resolver: Arc::new(|query| match query {
            FontQuery::Standard(s) => Some(s.get_font_data()),
            FontQuery::Fallback(f) => Some(f.pick_standard_font().get_font_data()),
        }),
        ..Default::default()
    };

    let interp_default = InterpreterSettings::default();

    // Render settings at different scales
    let render_settings_low = RenderSettings {
        x_scale: 0.5,
        y_scale: 0.5,
        ..Default::default()
    };
    let render_settings_default = RenderSettings::default();
    let render_settings_high = RenderSettings {
        x_scale: 2.0,
        y_scale: 2.0,
        ..Default::default()
    };

    let cache = RenderCache::default();

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

        // Render with default settings (no font resolver)
        let pixmap = render(page, &cache, &interp_default, &render_settings_default);
        let _ = pixmap.width();
        let _ = pixmap.height();
        let _ = pixmap.data();

        // Render with font resolver at low scale
        let pixmap_low = render(page, &cache, &interp_with_fonts, &render_settings_low);
        let _ = pixmap_low.width();
        let _ = pixmap_low.height();

        // Render at high scale
        let pixmap_high = render(page, &cache, &interp_with_fonts, &render_settings_high);

        // Exercise pixel access
        let w = pixmap_high.width();
        let h = pixmap_high.height();
        if w > 0 && h > 0 {
            let _ = pixmap_high.sample(0, 0);
            let _ = pixmap_high.sample(w - 1, h - 1);
        }
        let _ = pixmap_high.data_as_u8_slice();

        // Try PNG encoding (consumes pixmap)
        match pixmap_high.into_png() {
            Ok(png) => {
                if let Some(save_path) = save_path {
                    let output_path = format!("{save_path}_{}.png", page_num);
                    if let Err(e) = fs::write(&output_path, &png) {
                        eprintln!("Failed to write PNG {}: {}", output_path, e);
                    }
                }
            }
            Err(e) => eprintln!("Failed to encode PNG for page {}: {:?}", page_num, e),
        }

        // Take unpremultiplied pixels from the low-scale render
        let _ = pixmap_low.take_unpremultiplied();

        // Take premultiplied pixels from the default render
        let _ = pixmap.take();
    }

    println!("OK: {file_path}");
}
