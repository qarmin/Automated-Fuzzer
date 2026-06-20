use std::fs;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::XmlVersion;

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else {
        return;
    };

    let mut reader = Reader::from_reader(data.as_slice());
    let config = reader.config_mut();
    config.expand_empty_elements = true;
    config.trim_text(true);

    let mut buf = Vec::new();
    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event,
            Err(_) => break,
        };
        match &event {
            Event::Start(e) | Event::Empty(e) => {
                let _ = e.name();
                for a in e.attributes() {
                    let Ok(a) = a else { break };
                    let _ = a.decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder());
                }
            }
            Event::Text(e) | Event::Comment(e) | Event::DocType(e) => {
                let _ = e.decode();
            }
            Event::CData(e) => {
                let _ = e.clone().escape();
            }
            Event::End(e) => {
                let _ = e.name();
            }
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::GeneralRef(_) => {}
        }
        buf.clear();
    }

    if let Ok(s) = std::str::from_utf8(&data) {
        let mut reader = Reader::from_str(s);
        loop {
            match reader.read_event() {
                Ok(Event::Eof) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }
}
