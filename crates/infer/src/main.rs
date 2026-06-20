use std::fs;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else {
        return;
    };

    if let Some(kind) = infer::get(&data) {
        let _ = kind.mime_type();
        let _ = kind.extension();
    }

    let info = infer::Infer::new();
    let _ = info.is_image(&data);
    let _ = info.is_audio(&data);
    let _ = info.is_video(&data);
    let _ = info.is_archive(&data);
    let _ = info.is_document(&data);
    let _ = info.is_font(&data);
    let _ = info.is_app(&data);
    let _ = info.is_book(&data);
}
