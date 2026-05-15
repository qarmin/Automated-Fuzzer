use std::fs;

use ttf_parser::GlyphId;

fn main() {
    fuzz_utils::run(check_file);
}
fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let face = match ttf_parser::Face::parse(&content, 0) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {}.", e);
            return;
        }
    };
    let gid = GlyphId(0);
    let _ = face.glyph_raster_image(gid, 0);
    let _ = face.glyph_raster_image(gid, 96);
    let _ = face.glyph_raster_image(gid, u16::MAX);
    let _ = face.glyph_name(gid);
}
