use std::env::args;
use std::fs;
use std::path::Path;

use i_slint_compiler::diagnostics::{BuildDiagnostics, SourceFile};
use i_slint_compiler::lexer;
use i_slint_compiler::parser::parse_tokens;
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file");
    }

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
fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };
    let Ok(s) = String::from_utf8(content) else {
        return;
    };
    let tokens = lexer::lex(&s);
    if tokens.is_empty() {
        return;
    }

    let source_file = SourceFile::default();
    let mut diags = BuildDiagnostics::default();

    let doc_node = parse_tokens(tokens, source_file, &mut diags);

    let (_, _, _) = spin_on::spin_on(i_slint_compiler::compile_syntax_node(
        doc_node,
        diags,
        i_slint_compiler::CompilerConfiguration::new(i_slint_compiler::generator::OutputFormat::Llr),
    ));
}
