#![no_main]

use i_slint_compiler::diagnostics::{BuildDiagnostics, SourceFile};
use i_slint_compiler::lexer;
use i_slint_compiler::parser::parse_tokens;
use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    if let Ok(s) = std::str::from_utf8(data) {
        let tokens = lexer::lex(s);
        if tokens.is_empty() {
            return Corpus::Reject;
        }

        let source_file = SourceFile::default();
        let mut diags = BuildDiagnostics::default();

        let doc_node = parse_tokens(tokens, source_file, &mut diags);

        let (_, _, _) = spin_on::spin_on(i_slint_compiler::compile_syntax_node(
            doc_node,
            diags,
            i_slint_compiler::CompilerConfiguration::new(
                i_slint_compiler::generator::OutputFormat::Llr,
            ),
        ));

        Corpus::Keep
    } else {
        Corpus::Reject
    }

});