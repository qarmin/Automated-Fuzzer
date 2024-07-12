use std::process::{Child, Command, Stdio};

use crate::obj::ProgramConfig;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LANGS {
    PYTHON,
    JAVASCRIPT,
    LUA,
    GO,
    #[allow(dead_code)]
    RUST,
    GENERAL,
}

const PYTHON_ARGS: &[&str] = &[
    "noqa", "#", "'", "\"", "False", "await", "else", "import", "pass", "None", "break", "except", "in", "raise",
    "True", "class", "finally", "is", "return", "and", "continue", "for", "lambda", "float", "int", "bool", "try",
    "as", "def", "from", "nonlocal", "while", "assert", "del", "global", "not", "with", "async", "elif", "if", "or",
    "yield", "__init__", "pylint", ":", "?", "[", "\"", "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=",
    ">", ".", ",", "==", "!=", "{", "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()",
    "[]", "\n", "\t", "# fmt: skip", "# fmt: off", "# fmt: on", "# fmt: noqa", "# noqa", "# type:", "is not", "None",
    "False", "True", "is None", "is not None", "is False", "is True", "is not ", "is not True", "is not False",
    "is not None", "is False", "is True", "is not True",
];

const JAVASCRIPT_ARGS: &[&str] = &[
    ":", "?", "[", "\"", "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".", ",", "==", "!=", "{",
    "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()", "[]", "pylint", "\n", "\t",
    "#", "'", "\"", "//", "abstract", "arguments", "await", "boolean", "break", "byte", "case", "catch", "char",
    "class", "const", "continue", "debugger", "default", "delete", "do", "double", "else", "enum", "eval", "export",
    "extends", "false", "final", "finally", "float", "for", "function", "goto", "if", "implements", "import", "in",
    "instanceof", "int", "interface", "let", "long", "native", "new", "null", "package", "private", "protected",
    "public", "return", "short", "static", "super", "switch", "synchronized", "this", "throw", "throws", "transient",
    "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield", "                                "
];
const LUA_ARGS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local", "nil", "not", "or",
    "repeat", "return", "then", "true", "until", "while", "+", "-", "*", "/", "%", "^", "#", "==", "~=", "<=", ">=",
    "<", ">", "=", "(", ")", "{", "}", "[", "]", ";", ":", ",", ".", "..", "...", "\"", "\'", "\'\'", "\"\"",
];

// "|", "||",  "|=", "--", "-="  cause some problems
const GO_ARGS: &[&str] = &[
    "<", "<=", "[", "+", "&", "+=", "&=", "&&", "==", "!=", "(", ")", "-", "*", "]", "^", "*=", "^=", "<-", ">", ">=",
    "{", "}", "/", "<<", "/=", "<<=", "++", "=", ":=", ",", ";", "%", ">>", "%=", ">>=", "!", "...", ".", ":", "&^",
    "&^=", "~", "break", "default", "func", "interface", "select", "case", "defer", "go", "map", "struct", "chan",
    "else", "goto", "package", "switch", "const", "fallthrough", "if", "range", "type", "continue", "for", "import",
    "return", "var", "append", "cap", "complex", "delete", "len", "panic",
    "                                                                                        ", "https"
];

// "|", "||",  "|=", "--", "-=", "\0", "->"  cause some problems
const RUST_ARGS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
    "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box",
    "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "union", "'static",
    "dyn", "r#", "//", "///", "////", "//!", "//!!", "/*!", "/*!!", "/*", "/**", "/***", "*/", "**/", "***/", "\r",
    "\n", " ", "\t", "b", "\\", "/", "'", "\"", "0x", "0b", "0o", "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64",
    "u128", "i128", "usize", "isize", "f32", "f64", "{", "+", "-", "*", "/", "%", "^", "!", "&", "&&", "<<", ">>",
    "+=", "*=", "/=", "%=", "^=", "&=", "<<=", ">>=", "=", "==", "!=", ">", "<", ">=", "<=", "@", "_", ".", "..",
    "...", "..=", ",", ";", ":", "::", "=>", "$", "?", "~", "{", "}", "[", "]", "(", ")",
];

pub fn create_broken_files(obj: &dyn ProgramConfig, lang: LANGS) -> Child {
    let valid_input_files_dir = &obj.get_settings().valid_input_files_dir;
    let temp_possible_broken_files_dir = &obj.get_settings().temp_possible_broken_files_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    let mut command = Command::new("create_broken_files");
    let mut com = &mut command;
    if lang != LANGS::GENERAL {
        com = com.args(format!("-i {valid_input_files_dir} -o {temp_possible_broken_files_dir} -n {broken_files_for_each_file} -c true -s").split(' '));
    }
    match lang {
        LANGS::PYTHON => com = com.args(PYTHON_ARGS),
        LANGS::JAVASCRIPT => com = com.args(JAVASCRIPT_ARGS),
        LANGS::LUA => com = com.args(LUA_ARGS),
        LANGS::GO => com = com.args(GO_ARGS),
        LANGS::RUST => com = com.args(RUST_ARGS),
        LANGS::GENERAL => {
            com = com.args(
                format!(
                    "-i {valid_input_files_dir} -o {temp_possible_broken_files_dir} -n {broken_files_for_each_file} -c false"
                )
                    .split(' '),
            );
        }
    }
    com.stderr(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap()
}
