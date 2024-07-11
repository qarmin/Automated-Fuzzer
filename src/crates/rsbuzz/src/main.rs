use std::env::args;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
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
    let data = std::fs::read(path).unwrap();
    let _ = process_face(&data);
}
fn process_face(data: &[u8]) -> Option<()> {
    let face =  rustybuzz::Face::from_slice(data, 0)?;
    let buffer = rustybuzz::UnicodeBuffer::new();
    rustybuzz::shape(&face, &[], buffer);
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str("fi");
    rustybuzz::shape(&face, &[], buffer);
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFfASFAMFIQAWNFWOIQBFOBFOABFOBAWOFBQWOFBOABFOASBOFBASOFBOASBFOIAWBOFBQWOFBOAWBFOIAWBFOAWBFOAWBODBAWODNQWOFBQOWBFOABFOAWBODBAOWWAOFBNOQWTGPOQWNGFPOWQNBFDOQWNFONQWDFNASODNBAWOBDFWQOFBNQWODOQWFNBOQWBNFOWAF");
    rustybuzz::shape(&face, &[], buffer);
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str("ĄĆŹŻĆŒĆŁΩŒ™ΩŒ™ΩŒ® ̵ŁŁ®Ω¡¿®¡˝¿∧¡×¿£∧×¡¿¼¡—®ÞŁ¡¿¡¿™GŒÐΩŒÐÞΩŒÆŊ ̵ΩŒŊ°ÞΩ¡Ff");
    rustybuzz::shape(&face, &[], buffer);
    Some(())
}