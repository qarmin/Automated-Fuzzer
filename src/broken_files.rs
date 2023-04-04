use crate::obj::ProgramConfig;
use std::process::{Child, Command, Stdio};

const PYTHON_ARGS: &[&str] = &[
    "noqa", "#", "'", "\"", "False", "await", "else", "import", "pass", "None", "break", "except",
    "in", "raise", "True", "class", "finally", "is", "return", "and", "continue", "for", "lambda",
    "float", "int", "bool", "try", "as", "def", "from", "nonlocal", "while", "assert", "del",
    "global", "not", "with", "async", "elif", "if", "or", "yield", "__init__", ":", "?", "[", "\"",
    "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".", ",", "==", "!=", "{",
    "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()", "[]",
    "pylint", "\n", "\t",
];

const JAVASCRIPT_ARGS: &[&str] = &[
    ":", "?", "[", "\"", "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".",
    ",", "==", "!=", "{", "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)",
    "{}", "()", "[]", "pylint", "\n", "\t", "#", "'", "\"", "//", "abstract", "arguments", "await",
    "boolean", "break", "byte", "case", "catch", "char", "class", "const", "continue", "debugger",
    "default", "delete", "do", "double", "else", "enum", "eval", "export", "extends", "false",
    "final", "finally", "float", "for", "function", "goto", "if", "implements", "import", "in",
    "instanceof", "int", "interface", "let", "long", "native", "new", "null", "package", "private",
    "protected", "public", "return", "short", "static", "super", "switch", "synchronized", "this",
    "throw", "throws", "transient", "true", "try", "typeof", "var", "void", "volatile", "while",
    "with", "yield",
];

pub fn create_broken_python_files(obj: &dyn ProgramConfig) -> Child {
    let base_of_valid_files = &obj.get_settings().base_of_valid_files;
    let input_dir = &obj.get_settings().input_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    Command::new("create_broken_files")
        .args(format!("-i {base_of_valid_files} -o {input_dir} -n {broken_files_for_each_file} -c true -s").split(' '))
        .args(PYTHON_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_broken_javascript_files(obj: &dyn ProgramConfig) -> Child {
    let base_of_valid_files = &obj.get_settings().base_of_valid_files;
    let input_dir = &obj.get_settings().input_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    Command::new("create_broken_files")
        .args(format!("-i {base_of_valid_files} -o {input_dir} -n {broken_files_for_each_file} -c true -s").split(' '))
        .args(JAVASCRIPT_ARGS)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

pub fn create_broken_general_files(obj: &dyn ProgramConfig) -> Child {
    let base_of_valid_files = &obj.get_settings().base_of_valid_files;
    let input_dir = &obj.get_settings().input_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    Command::new("create_broken_files")
        .args(
            format!(
                "-i {base_of_valid_files} -o {input_dir} -n {broken_files_for_each_file} -c false"
            )
            .split(' '),
        )
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}
