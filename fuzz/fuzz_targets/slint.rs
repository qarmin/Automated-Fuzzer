#![no_main]

use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::parser::parse;
use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut b = BuildDiagnostics::default();
        parse(s.to_string(), None, &mut b);
        // if lexer::lex(s).is_empty() {
        //     Corpus::Reject
        // } else {
        //     Corpus::Keep
        // }
        Corpus::Keep
    } else {
        Corpus::Reject
    }

});