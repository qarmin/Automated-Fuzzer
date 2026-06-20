use std::fs;

use swash::scale::ScaleContext;
use swash::FontRef;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else {
        return;
    };

    for index in 0..4u32 {
        let Some(font) = FontRef::from_index(&data, index as usize) else {
            continue;
        };

        let _ = font.attributes();
        let _ = font.metrics(&[]);
        let _ = font.glyph_metrics(&[]);
        let _ = font.writing_systems().count();
        let _ = font.instances().count();
        let _ = font.variations().count();
        let _ = font.features().count();

        let charmap = font.charmap();
        for codepoint in 0u32..256 {
            let gid = charmap.map(codepoint);
            if gid != 0 {
                let mut context = ScaleContext::new();
                let mut scaler = context.builder(font).size(16.0).build();
                let _ = swash::scale::Render::new(&[swash::scale::Source::Outline]).render(&mut scaler, gid);
            }
        }
    }
}
