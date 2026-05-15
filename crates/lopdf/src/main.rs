use std::fs;
use std::io::Cursor;

use lopdf::Document;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };

    println!("Checking file: {file_path}");

    let cursor = Cursor::new(content);
    let mut document = match Document::load_from(cursor) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("Error reading PDF: {err}");
            return;
        }
    };

    // Read document version
    let _ = document.version.clone();

    // Read trailer dictionary
    let _ = document.trailer.clone();

    // Read reference table
    let _ = document.reference_table.clone();

    // Get page count and page list
    let pages = document.get_pages();
    let page_count = pages.len();
    println!("Pages: {page_count}");

    // Iterate all objects in the document
    for (obj_id, object) in document.objects.iter() {
        let _ = obj_id;
        let _ = object.type_name();
    }

    // Extract text from each page
    for i in 1..=(page_count as u32) {
        match document.extract_text(&[i]) {
            Ok(text) => {
                let _ = text.len();
            }
            Err(e) => {
                eprintln!("extract_text page {i}: {e}");
            }
        }
    }

    // Try to get page content streams, fonts, and annotations for each page
    for (_page_num, page_id) in &pages {
        // Get the page object
        if let Ok(page_obj) = document.get_object(*page_id) {
            let _ = page_obj.type_name();
        }

        // Try to get page content
        match document.get_page_content(*page_id) {
            Ok(content) => {
                // Try to decode the content stream
                match lopdf::content::Content::decode(&content) {
                    Ok(content_parsed) => {
                        for operation in &content_parsed.operations {
                            let _ = operation.operator.clone();
                            let _ = operation.operands.len();
                        }
                    }
                    Err(e) => {
                        eprintln!("decode content: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("get_page_content: {e}");
            }
        }

        // Try to get page fonts
        if let Ok(fonts) = document.get_page_fonts(*page_id) {
            for (name, font_ref) in &fonts {
                let _ = name;
                let _ = font_ref;
            }
        }

        // Try to get page annotations
        if let Ok(annotations) = document.get_page_annotations(*page_id) {
            for annotation in &annotations {
                let _ = annotation.len();
            }
        }
    }

    // Decompress and re-compress
    let mut doc_clone = document.clone();
    doc_clone.decompress();

    // Try compressing
    doc_clone.compress();

    // Try saving the decompressed document
    if let Err(e) = doc_clone.save_to(&mut Cursor::new(Vec::new())) {
        eprintln!("save decompressed: {e}");
    }

    // Try saving the original document
    if let Err(e) = document.save_to(&mut Cursor::new(Vec::new())) {
        eprintln!("save original: {e}");
    }

    // Try to prune objects
    let mut doc_prune = document.clone();
    doc_prune.prune_objects();

    // Try to delete zero-length streams
    let mut doc_del = document.clone();
    doc_del.delete_zero_length_streams();

    // Try to renumber objects
    let mut doc_renum = document.clone();
    doc_renum.renumber_objects();
}
