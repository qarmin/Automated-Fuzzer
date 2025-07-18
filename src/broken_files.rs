use std::process::{Child, Command, Stdio};

use crate::obj::ProgramConfig;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum LANGS {
    PYTHON,
    JAVASCRIPT,
    LUA,
    GO,
    RUST,
    BINARY,
    TEXT,
    SLINT,
    JSVUESVELTE,
    SVG,
}

const SLINT_ARGS: &[&str] = &[
    "Rectangle", "width", "height", ":", ";", "phx", "px", ":=", "=", "{", "}", "<", ">", "bool", "int", "float", "=>",
    "<=", "<=>", "=>", "(_)", "pure", "callback", "-", "^^", "^", "/", "*", "+", ".", "_", "==", "//", "@",
    "@image-url", "@tr", "(", ")", "\"", "import ", "from", "changed", ",", "return", "#", "transparent", "inherits",
    "Window", "false", "true", "clicked()", "%", "init", "debug", "accept", "visible", "property", "string", "in-out ",
    "in-out property", "[", "]", "out", "out property", "in", "in property", "export", "||", "!=", "[string]",
    "length", "x", "y", "z", "min-width", "max-width", "opacity", "animation-tick", "accessible-role", "component",
    "icon", "Palette", "Palette.background", "TextInputInterface", "Colors", "Key", "Math", "abs", "abs", "acos",
    "ceil", "clamp", "log", "max", "mod", "sqrt", "pow", "TableColumn", "StandardListViewItem", "PointerScrollEvent",
    "PointerEvent", "Point", "KeyboardModifiers", "KeyEvent", "if", "/*", "*/", "angle", "brush", "color", "duration",
    "easing", "image", "percent", "physical-length", "relative-font-size", "to-float", "[0]", "[100]", "[999999999]",
    "self", "self.", "parent", "parent.", "?", "!", "function", "root", "root.", "public", "for", "for r[idx] in",
    "in", "animate", "states", "global", "export global",
];

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
    "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield", "                                ",
];

const JS_VUE_SVELTE: &[&str] = &[
    ":", "?", "[", "\"", "\"\"\"", "\'", "]", "}", "%", "f\"", "f'", "<", "<=", ">=", ">", ".", ",", "==", "!=", "{",
    "=", "|", "\\", ";", "_", "-", "**", "*", "/", "!", "(", ")", "(True)", "{}", "()", "[]", "pylint", "\n", "\t",
    "#", "'", "\"", "//", "abstract", "arguments", "await", "boolean", "break", "byte", "case", "catch", "char",
    "class", "const", "continue", "debugger", "default", "delete", "do", "double", "else", "enum", "eval", "export",
    "extends", "false", "final", "finally", "float", "for", "function", "goto", "if", "implements", "import", "in",
    "instanceof", "int", "interface", "let", "long", "native", "new", "null", "package", "private", "protected",
    "public", "return", "short", "static", "super", "switch", "synchronized", "this", "throw", "throws", "transient",
    "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield", "                                ",
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
    "                                                                                        ", "https",
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

const SVG_ARGS: &[&str] = &[
    "<svg>", "</svg>", // podstawowe atrybuty SVG
    "width", "height", "viewBox", "preserveAspectRatio", "xmlns", "xmlns:xlink", // atrybuty globalne
    "id", "class", "style", "transform", "visibility", // współrzędne i rozmiary
    "x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "r", "rx", "ry", // kolory i style
    "fill", "fill-opacity", "stroke", "stroke-width", "stroke-linecap", "stroke-dasharray", "opacity",
    // tekst
    "font-family", "font-size", "font-style", "font-weight", "text-anchor", "letter-spacing", "word-spacing",
    // gradienci i filtry
    "gradientUnits", "gradientTransform", "filter", "flood-color", "flood-opacity", // animacja
    "from", "to", "dur", "repeatCount", "keyTimes", "keyPoints", // linki i odwołania
    "href", "xlink:href", // atrybuty dodatkowe
    "cursor", "clip-path", "clip-rule", "opacity", "viewTarget", "=", "=\"\"", "5", "0.2", "<", ">",
];

pub(crate) fn create_broken_files(obj: &dyn ProgramConfig, lang: LANGS) -> Child {
    let valid_input_files_dir = &obj.get_settings().valid_input_files_dir;
    let temp_possible_broken_files_dir = &obj.get_settings().temp_possible_broken_files_dir;
    let broken_files_for_each_file = &obj.get_settings().broken_files_for_each_file;
    let mut command = Command::new("create_broken_files");
    let mut com = &mut command;
    if ![LANGS::BINARY, LANGS::TEXT].contains(&lang) {
        com = com.args(
            format!(
                "-i {valid_input_files_dir} -o {temp_possible_broken_files_dir} -n {broken_files_for_each_file} -c -s"
            )
            .split(' '),
        );
    }
    match lang {
        LANGS::PYTHON => com = com.args(PYTHON_ARGS).arg("-m"),
        LANGS::JAVASCRIPT => com = com.args(JAVASCRIPT_ARGS),
        LANGS::LUA => com = com.args(LUA_ARGS),
        LANGS::GO => com = com.args(GO_ARGS),
        LANGS::RUST => com = com.args(RUST_ARGS),
        LANGS::SLINT => com = com.args(SLINT_ARGS),
        LANGS::JSVUESVELTE => com = com.args(JS_VUE_SVELTE),
        LANGS::SVG => com = com.args(SVG_ARGS),
        LANGS::BINARY => {
            com = com.args(
                format!(
                    "-i {valid_input_files_dir} -o {temp_possible_broken_files_dir} -n {broken_files_for_each_file}"
                )
                .split(' '),
            );
        }
        LANGS::TEXT => {
            com = com.args(
                format!(
                    "-i {valid_input_files_dir} -o {temp_possible_broken_files_dir} -n {broken_files_for_each_file} -c"
                )
                .split(' '),
            );
        }
    }

    // info!("Command: {:?}", com);

    com.stderr(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap()
}
