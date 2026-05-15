use std::env::args;
use std::fs;
use std::path::Path;

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).expect("Usage: pdf_writer <file_or_dir>");
    if !Path::new(&path).exists() {
        panic!("Missing file: {path:?}");
    }

    if Path::new(&path).is_dir() {
        for entry in WalkDir::new(&path).into_iter().flatten() {
            if entry.file_type().is_file() {
                check_file(&entry.path().to_string_lossy());
            }
        }
    } else {
        check_file(&path);
    }
}

fn check_file(path: &str) {
    let Ok(data) = fs::read(path) else { return };
    let tc = TestCase::from_bytes(&data);

    // Write reproducer BEFORE running the library — guarantees the file
    // exists even if the call below panics, aborts, or gets SIGKILL'd.
    let _ = fs::write(format!("{path}.reproducer.rs"), tc.to_rust_reproducer());

    run_pdf_writer(&tc);
}

// ──────────────────────────────────────────────────────────────────────────────
// TestCase: structured representation of a fuzzed PDF.
// Built from raw bytes (deterministic), executed as-is, and emitted as
// standalone Rust code for the crash reproducer.
// ──────────────────────────────────────────────────────────────────────────────

struct TestCase {
    pages: Vec<PageCase>,
    info: Option<DocInfo>,
}

struct PageCase {
    width: f32,
    height: f32,
    crop: Option<(f32, f32)>,
    rotation: i32,
    has_font: bool,
    ops: Vec<Op>,
}

enum Op {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    Rect(f32, f32, f32, f32),
    Stroke,
    FillNonzero,
    FillEvenOdd,
    ClosePath,
    SetLineWidth(f32),
    SetFillRgb(f32, f32, f32),
    SetStrokeRgb(f32, f32, f32),
    Text { font_size: f32, text: Vec<u8>, tx: f32, ty: f32 },
    CubicTo(f32, f32, f32, f32, f32, f32),
    SaveRestore,
}

struct DocInfo {
    title: Option<String>,
    author: Option<String>,
}

impl TestCase {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut cur = Cursor::new(bytes);
        let page_count = cur.u8_range(1, 20).unwrap_or(1);
        let mut pages = Vec::with_capacity(page_count as usize);
        for _ in 0..page_count {
            match PageCase::from_cursor(&mut cur) {
                Some(p) => pages.push(p),
                None => break,
            }
        }
        if pages.is_empty() {
            pages.push(PageCase::default());
        }
        let info = if cur.bool().unwrap_or(false) {
            Some(DocInfo::from_cursor(&mut cur))
        } else {
            None
        };
        Self { pages, info }
    }
}

impl PageCase {
    fn default() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            crop: None,
            rotation: 0,
            has_font: false,
            ops: Vec::new(),
        }
    }

    fn from_cursor(cur: &mut Cursor<'_>) -> Option<Self> {
        let width = cur.f32_range(1.0, 2000.0)?;
        let height = cur.f32_range(1.0, 2000.0)?;
        let crop = if cur.bool()? {
            let cx = cur.f32_range(0.0, width)?;
            let cy = cur.f32_range(0.0, height)?;
            Some((cx, cy))
        } else {
            None
        };
        let rotation = match cur.u8_range(0, 3)? {
            1 => 90,
            2 => 180,
            3 => 270,
            _ => 0,
        };
        let has_font = cur.bool()?;
        let op_count = cur.u8_range(0, 30)?;
        let mut ops = Vec::with_capacity(op_count as usize);
        for _ in 0..op_count {
            match Op::from_cursor(cur, width, height) {
                Some(op) => ops.push(op),
                None => break,
            }
        }
        Some(Self { width, height, crop, rotation, has_font, ops })
    }
}

impl Op {
    fn from_cursor(cur: &mut Cursor<'_>, page_w: f32, page_h: f32) -> Option<Self> {
        let kind = cur.u8_range(0, 12)?;
        match kind {
            0 => Some(Op::MoveTo(cur.f32_range(-1000.0, 3000.0)?, cur.f32_range(-1000.0, 3000.0)?)),
            1 => Some(Op::LineTo(cur.f32_range(-1000.0, 3000.0)?, cur.f32_range(-1000.0, 3000.0)?)),
            2 => Some(Op::Rect(
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(0.0, 1000.0)?,
                cur.f32_range(0.0, 1000.0)?,
            )),
            3 => Some(Op::Stroke),
            4 => Some(Op::FillNonzero),
            5 => Some(Op::FillEvenOdd),
            6 => Some(Op::ClosePath),
            7 => Some(Op::SetLineWidth(cur.f32_range(0.0, 100.0)?)),
            8 => Some(Op::SetFillRgb(
                cur.f32_range(0.0, 1.0)?,
                cur.f32_range(0.0, 1.0)?,
                cur.f32_range(0.0, 1.0)?,
            )),
            9 => Some(Op::SetStrokeRgb(
                cur.f32_range(0.0, 1.0)?,
                cur.f32_range(0.0, 1.0)?,
                cur.f32_range(0.0, 1.0)?,
            )),
            10 => {
                let font_size = cur.f32_range(1.0, 200.0)?;
                let text = cur.bytes(100)?;
                let tx = cur.f32_range(0.0, page_w)?;
                let ty = cur.f32_range(0.0, page_h)?;
                Some(Op::Text { font_size, text, tx, ty })
            }
            11 => Some(Op::CubicTo(
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
                cur.f32_range(-500.0, 2000.0)?,
            )),
            _ => Some(Op::SaveRestore),
        }
    }
}

impl DocInfo {
    fn from_cursor(cur: &mut Cursor<'_>) -> Self {
        let title = cur.string(50);
        let author = cur.string(50);
        Self { title, author }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Runner: takes a TestCase and drives pdf-writer with it.
// ──────────────────────────────────────────────────────────────────────────────

fn run_pdf_writer(tc: &TestCase) {
    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_count = tc.pages.len() as i32;
    let page_ids: Vec<Ref> = (0..page_count).map(|i| Ref::new(3 + i)).collect();

    pdf.catalog(catalog_id).pages(page_tree_id);

    let mut pages = pdf.pages(page_tree_id);
    pages.count(page_count);
    pages.kids(page_ids.iter().copied());
    pages.finish();

    for (i, page) in tc.pages.iter().enumerate() {
        let i = i as i32;
        let page_ref = Ref::new(3 + i);
        let content_ref = Ref::new(3 + page_count + i);

        let mut p = pdf.page(page_ref);
        p.parent(page_tree_id);
        p.media_box(Rect::new(0.0, 0.0, page.width, page.height));
        if let Some((cx, cy)) = page.crop {
            p.crop_box(Rect::new(cx, cy, page.width, page.height));
        }
        if page.rotation != 0 {
            p.rotate(page.rotation);
        }
        p.contents(content_ref);

        let mut res = p.resources();
        if page.has_font {
            res.fonts().pair(Name(b"F1"), Ref::new(100 + i));
        }
        res.finish();
        p.finish();

        let mut content = Content::new();
        for op in &page.ops {
            match op {
                Op::MoveTo(x, y) => { content.move_to(*x, *y); }
                Op::LineTo(x, y) => { content.line_to(*x, *y); }
                Op::Rect(x, y, w, h) => { content.rect(*x, *y, *w, *h); }
                Op::Stroke => { content.stroke(); }
                Op::FillNonzero => { content.fill_nonzero(); }
                Op::FillEvenOdd => { content.fill_even_odd(); }
                Op::ClosePath => { content.close_path(); }
                Op::SetLineWidth(w) => { content.set_line_width(*w); }
                Op::SetFillRgb(r, g, b) => { content.set_fill_rgb(*r, *g, *b); }
                Op::SetStrokeRgb(r, g, b) => { content.set_stroke_rgb(*r, *g, *b); }
                Op::Text { font_size, text, tx, ty } => {
                    content.begin_text();
                    content.set_font(Name(b"F1"), *font_size);
                    content.next_line(*tx, *ty);
                    content.show(Str(text));
                    content.end_text();
                }
                Op::CubicTo(x1, y1, x2, y2, x3, y3) => {
                    content.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
                }
                Op::SaveRestore => {
                    content.save_state();
                    content.restore_state();
                }
            }
        }
        pdf.stream(content_ref, &content.finish());
    }

    if let Some(info) = &tc.info {
        let info_ref = Ref::new(200);
        let mut di = pdf.document_info(info_ref);
        if let Some(t) = &info.title { di.title(TextStr(t)); }
        if let Some(a) = &info.author { di.author(TextStr(a)); }
        di.finish();
    }

    let output = pdf.finish();
    assert!(!output.is_empty(), "PDF output should not be empty");
}

// ──────────────────────────────────────────────────────────────────────────────
// Reproducer emission: standalone Rust code with all values hardcoded.
// The output has zero dependencies beyond `pdf-writer` itself.
// ──────────────────────────────────────────────────────────────────────────────

impl TestCase {
    fn to_rust_reproducer(&self) -> String {
        let mut out = String::new();
        out.push_str("#![allow(unused_imports)]\n");
        out.push_str("use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str, TextStr};\n\n");
        out.push_str("fn main() {\n");
        out.push_str("    let mut pdf = Pdf::new();\n");
        out.push_str("    let catalog_id = Ref::new(1);\n");
        out.push_str("    let page_tree_id = Ref::new(2);\n");
        let pc = self.pages.len() as i32;
        out.push_str(&format!("    let page_count: i32 = {pc};\n"));
        out.push_str("\n");
        out.push_str("    pdf.catalog(catalog_id).pages(page_tree_id);\n\n");
        out.push_str("    let page_ids: Vec<Ref> = (0..page_count).map(|i| Ref::new(3 + i)).collect();\n");
        out.push_str("    let mut pages = pdf.pages(page_tree_id);\n");
        out.push_str("    pages.count(page_count);\n");
        out.push_str("    pages.kids(page_ids.iter().copied());\n");
        out.push_str("    pages.finish();\n\n");

        for (i, page) in self.pages.iter().enumerate() {
            page.emit(&mut out, i as i32, pc);
        }

        if let Some(info) = &self.info {
            info.emit(&mut out);
        }

        out.push_str("    let output = pdf.finish();\n");
        out.push_str("    assert!(!output.is_empty());\n");
        out.push_str("}\n");
        out
    }
}

impl PageCase {
    fn emit(&self, out: &mut String, i: i32, total: i32) {
        out.push_str(&format!("    // ── Page {i} ──\n"));
        out.push_str("    {\n");
        out.push_str(&format!("        let page_ref = Ref::new({});\n", 3 + i));
        out.push_str(&format!("        let content_ref = Ref::new({});\n", 3 + total + i));
        out.push_str("        let mut page = pdf.page(page_ref);\n");
        out.push_str("        page.parent(page_tree_id);\n");
        out.push_str(&format!(
            "        page.media_box(Rect::new(0.0, 0.0, {}, {}));\n",
            fmt_f32(self.width),
            fmt_f32(self.height),
        ));
        if let Some((cx, cy)) = self.crop {
            out.push_str(&format!(
                "        page.crop_box(Rect::new({}, {}, {}, {}));\n",
                fmt_f32(cx),
                fmt_f32(cy),
                fmt_f32(self.width),
                fmt_f32(self.height),
            ));
        }
        if self.rotation != 0 {
            out.push_str(&format!("        page.rotate({});\n", self.rotation));
        }
        out.push_str("        page.contents(content_ref);\n");
        let res_mut = if self.has_font { "mut " } else { "" };
        out.push_str(&format!("        let {res_mut}res = page.resources();\n"));
        if self.has_font {
            out.push_str(&format!(
                "        res.fonts().pair(Name(b\"F1\"), Ref::new({}));\n",
                100 + i,
            ));
        }
        out.push_str("        res.finish();\n");
        out.push_str("        page.finish();\n\n");

        let content_mut = if self.ops.is_empty() { "" } else { "mut " };
        out.push_str(&format!("        let {content_mut}content = Content::new();\n"));
        for op in &self.ops {
            op.emit(out);
        }
        out.push_str("        pdf.stream(content_ref, &content.finish());\n");
        out.push_str("    }\n\n");
    }
}

impl Op {
    fn emit(&self, out: &mut String) {
        match self {
            Op::MoveTo(x, y) => {
                out.push_str(&format!("        content.move_to({}, {});\n", fmt_f32(*x), fmt_f32(*y)));
            }
            Op::LineTo(x, y) => {
                out.push_str(&format!("        content.line_to({}, {});\n", fmt_f32(*x), fmt_f32(*y)));
            }
            Op::Rect(x, y, w, h) => {
                out.push_str(&format!(
                    "        content.rect({}, {}, {}, {});\n",
                    fmt_f32(*x), fmt_f32(*y), fmt_f32(*w), fmt_f32(*h),
                ));
            }
            Op::Stroke => out.push_str("        content.stroke();\n"),
            Op::FillNonzero => out.push_str("        content.fill_nonzero();\n"),
            Op::FillEvenOdd => out.push_str("        content.fill_even_odd();\n"),
            Op::ClosePath => out.push_str("        content.close_path();\n"),
            Op::SetLineWidth(w) => {
                out.push_str(&format!("        content.set_line_width({});\n", fmt_f32(*w)));
            }
            Op::SetFillRgb(r, g, b) => {
                out.push_str(&format!(
                    "        content.set_fill_rgb({}, {}, {});\n",
                    fmt_f32(*r), fmt_f32(*g), fmt_f32(*b),
                ));
            }
            Op::SetStrokeRgb(r, g, b) => {
                out.push_str(&format!(
                    "        content.set_stroke_rgb({}, {}, {});\n",
                    fmt_f32(*r), fmt_f32(*g), fmt_f32(*b),
                ));
            }
            Op::Text { font_size, text, tx, ty } => {
                out.push_str("        content.begin_text();\n");
                out.push_str(&format!(
                    "        content.set_font(Name(b\"F1\"), {});\n",
                    fmt_f32(*font_size),
                ));
                out.push_str(&format!("        content.next_line({}, {});\n", fmt_f32(*tx), fmt_f32(*ty)));
                out.push_str(&format!("        content.show(Str(&{}));\n", fmt_bytes(text)));
                out.push_str("        content.end_text();\n");
            }
            Op::CubicTo(x1, y1, x2, y2, x3, y3) => {
                out.push_str(&format!(
                    "        content.cubic_to({}, {}, {}, {}, {}, {});\n",
                    fmt_f32(*x1), fmt_f32(*y1), fmt_f32(*x2), fmt_f32(*y2), fmt_f32(*x3), fmt_f32(*y3),
                ));
            }
            Op::SaveRestore => {
                out.push_str("        content.save_state();\n");
                out.push_str("        content.restore_state();\n");
            }
        }
    }
}

impl DocInfo {
    fn emit(&self, out: &mut String) {
        out.push_str("    let info_ref = Ref::new(200);\n");
        out.push_str("    let mut di = pdf.document_info(info_ref);\n");
        if let Some(t) = &self.title {
            out.push_str(&format!("    di.title(TextStr({:?}));\n", t));
        }
        if let Some(a) = &self.author {
            out.push_str(&format!("    di.author(TextStr({:?}));\n", a));
        }
        out.push_str("    di.finish();\n\n");
    }
}

fn fmt_f32(v: f32) -> String {
    if v.is_nan() {
        return "f32::NAN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "f32::INFINITY".into() } else { "f32::NEG_INFINITY".into() };
    }
    format!("{v:?}_f32")
}

fn fmt_bytes(b: &[u8]) -> String {
    let mut s = String::from("[");
    for (i, byte) in b.iter().enumerate() {
        if i > 0 { s.push_str(", "); }
        s.push_str(&format!("{byte}"));
    }
    s.push(']');
    s
}

// ──────────────────────────────────────────────────────────────────────────────
// Cursor: minimal byte reader used only for fuzz-input → TestCase.
// Not exposed; the reproducer code never references it.
// ──────────────────────────────────────────────────────────────────────────────

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take_byte(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn take_n(&mut self, n: usize) -> Option<&[u8]> {
        if self.pos + n <= self.data.len() {
            let s = &self.data[self.pos..self.pos + n];
            self.pos += n;
            Some(s)
        } else {
            None
        }
    }

    fn u8_range(&mut self, min: u8, max: u8) -> Option<u8> {
        let raw = self.take_byte()?;
        let span = (max - min) as u16 + 1;
        Some(min + (raw as u16 % span) as u8)
    }

    fn bool(&mut self) -> Option<bool> {
        Some(self.take_byte()? & 1 == 1)
    }

    fn f32_range(&mut self, min: f32, max: f32) -> Option<f32> {
        let bytes = self.take_n(4)?;
        let raw = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let t = raw as f64 / u32::MAX as f64;
        Some((min as f64 + t * (max as f64 - min as f64)) as f32)
    }

    fn bytes(&mut self, max_len: usize) -> Option<Vec<u8>> {
        let len = self.take_byte()? as usize;
        let len = len.min(max_len).min(self.remaining());
        if len == 0 {
            return Some(Vec::new());
        }
        Some(self.take_n(len)?.to_vec())
    }

    fn string(&mut self, max_len: usize) -> Option<String> {
        let raw = self.bytes(max_len)?;
        Some(String::from_utf8_lossy(&raw).into_owned())
    }
}
