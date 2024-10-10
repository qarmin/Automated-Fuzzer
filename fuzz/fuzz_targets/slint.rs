#![no_main]

use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::parser::parse;
use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut open_brackets = 0;
        for b in s.bytes() {
            if b == b'{' {
                open_brackets += 1;
                if open_brackets > 200 {
                    return Corpus::Reject;
                }
            }
        }

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