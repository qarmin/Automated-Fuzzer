#![no_main]

use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    if process_face(data).is_some() {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
});

fn process_face(data: &[u8]) -> Option<()> {
    let face = rustybuzz::Face::from_slice(data, 0)?;
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