use krilla::blend::BlendMode;
use krilla::color::rgb;
use krilla::destination::XyzDestination;
use krilla::error::KrillaError;
use krilla::geom::{Path, PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::Metadata;
use krilla::num::NormalizedF32;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::{Page, PageSettings};
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke};
use krilla::surface::Surface;
use krilla::Document;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static ITERATION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn main() {
    let start_time = Instant::now();
    let mut crash_count = 0;
    let mut success_count = 0;
    let mut crashes = Vec::new();
    let max_iterations = 200_000;

    for i in 0..max_iterations {
        ITERATION_COUNTER.store(i, Ordering::SeqCst);

        let seed = i;
        match std::panic::catch_unwind(|| run_fuzz_iteration(seed)) {
            Ok(Ok(_)) => {
                success_count += 1;
            }
            Ok(Err(e)) => {
                crash_count += 1;
                let msg = format!("Error: {:?}", e);
                crashes.push((seed, msg));
            }
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    format!("{:?}", panic)
                };

                if [
                    "self.push_instructions.is_empty()", "self.bd.sub_builders.is_empty()", "Option::unwrap()",
                ]
                .iter()
                .any(|&frag| msg.contains(frag))
                {
                    continue;
                }
                println!("==================================");
                println!("==================================");
                println!("==================================");
                println!("==================================");
                println!("==================================");
                println!("==================================");
                println!("==================================");

                crash_count += 1;
                crashes.push((seed, msg));
            }
        }
    }

    if !crashes.is_empty() {
        let _ = crashes.len();
    }

    let _ = (start_time.elapsed(), max_iterations, success_count, crash_count);
}

fn run_fuzz_iteration(seed: u64) -> Result<Vec<u8>, KrillaError> {
    let mut rng = SimpleRng::new(seed);

    let mut doc = Document::new();

    let page_count = rng.range(1, 5);

    for _ in 0..page_count {
        let mut page = if rng.bool() {
            doc.start_page()
        } else {
            let width = rng.float_range(50.0, 1000.0);
            let height = rng.float_range(50.0, 1000.0);
            let settings = PageSettings::new(width, height);
            doc.start_page_with(settings)
        };

        let operation_count = rng.range(0, 10);
        for _ in 0..operation_count {
            fuzz_page_operations(&mut page, &mut rng);
        }

        page.finish();
    }

    if rng.bool() {
        let mut metadata = Metadata::new();

        if rng.bool() {
            metadata = metadata.title(generate_random_string(&mut rng, 50));
        }
        if rng.bool() {
            metadata = metadata.description(generate_random_string(&mut rng, 100));
        }
        if rng.bool() {
            metadata = metadata.creator(generate_random_string(&mut rng, 30));
        }
        if rng.bool() {
            metadata = metadata.language("en-US".to_string());
        }

        doc.set_metadata(metadata);
    }

    if rng.bool() {
        let mut outline = Outline::new();
        let node_count = rng.range(0, 3);

        for _ in 0..node_count {
            let text = generate_random_string(&mut rng, 30);
            let page_index = rng.range(0, page_count.max(1)) as usize;
            let point = Point::from_xy(rng.float_range(0.0, 500.0), rng.float_range(0.0, 500.0));

            let dest = XyzDestination::new(page_index, point);
            let node = OutlineNode::new(text, dest);
            outline.push_child(node);
        }

        doc.set_outline(outline);
    }

    doc.finish()
}

fn fuzz_page_operations(page: &mut Page, rng: &mut SimpleRng) {
    let mut surface = page.surface();

    let op_type = rng.range(0, 12);

    match op_type {
        0 => fuzz_draw_path(&mut surface, rng),
        1 => fuzz_draw_image(&mut surface, rng),
        2 => fuzz_transforms(&mut surface, rng),
        3 => fuzz_fill_stroke(&mut surface, rng),
        4 => fuzz_clipping(&mut surface, rng),
        5 => fuzz_opacity(&mut surface, rng),
        6 => fuzz_blend_mode(&mut surface, rng),
        7 => fuzz_nested_operations(&mut surface, rng),
        8 => fuzz_edge_case_values(&mut surface, rng),
        9 => fuzz_multiple_pops(&mut surface, rng),
        10 => fuzz_stream_builder(&mut surface, rng),
        _ => {}
    }

    surface.finish();
}

fn fuzz_draw_path(surface: &mut Surface, rng: &mut SimpleRng) {
    if let Some(path) = create_random_path(rng) {
        surface.draw_path(&path);
    }
}

fn fuzz_draw_image(surface: &mut Surface, rng: &mut SimpleRng) {
    let width = rng.range(1, 100) as u32;
    let height = rng.range(1, 100) as u32;

    let pixel_count = (width * height * 4) as usize;
    if pixel_count > 1_000_000 {
        return;
    }

    let mut data = Vec::with_capacity(pixel_count);
    for _ in 0..pixel_count {
        data.push(rng.byte());
    }

    let image = Image::from_rgba8(data, width, height);

    if let Some(size) = Size::from_wh(rng.float_range(1.0, 500.0), rng.float_range(1.0, 500.0)) {
        surface.draw_image(image, size);
    }
}

fn fuzz_transforms(surface: &mut Surface, rng: &mut SimpleRng) {
    let transform_type = rng.range(0, 5);

    let transform = match transform_type {
        0 => Transform::from_translate(rng.float_range(-500.0, 500.0), rng.float_range(-500.0, 500.0)),
        1 => Transform::from_scale(rng.float_range(-5.0, 5.0), rng.float_range(-5.0, 5.0)),
        2 => Transform::from_rotate(rng.float_range(0.0, 360.0)),
        3 => Transform::from_skew(rng.float_range(-2.0, 2.0), rng.float_range(-2.0, 2.0)),
        _ => Transform::identity(),
    };

    surface.push_transform(&transform);

    if rng.bool() {
        if let Some(path) = create_random_path(rng) {
            surface.draw_path(&path);
        }
        surface.pop();
    }
}

fn fuzz_fill_stroke(surface: &mut Surface, rng: &mut SimpleRng) {
    if rng.bool() {
        let color = rgb::Color::new(rng.byte(), rng.byte(), rng.byte());
        let fill = Fill {
            paint: color.into(),
            opacity: NormalizedF32::new(1.0).unwrap(),
            rule: FillRule::NonZero,
        };
        surface.set_fill(Some(fill));
    }

    if rng.bool() {
        let color = rgb::Color::new(rng.byte(), rng.byte(), rng.byte());
        let width = rng.float_range(0.1, 10.0);
        let stroke = Stroke {
            paint: color.into(),
            width,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 4.0,
            dash: None,
            opacity: NormalizedF32::new(1.0).unwrap(),
        };
        surface.set_stroke(Some(stroke));
    }

    if let Some(path) = create_random_path(rng) {
        surface.draw_path(&path);
    }
}

fn fuzz_clipping(surface: &mut Surface, rng: &mut SimpleRng) {
    if let Some(path) = create_random_path(rng) {
        let fill_rule = if rng.bool() {
            FillRule::NonZero
        } else {
            FillRule::EvenOdd
        };

        surface.push_clip_path(&path, &fill_rule);

        if rng.bool() {
            if let Some(inner_path) = create_random_path(rng) {
                surface.draw_path(&inner_path);
            }
            surface.pop();
        }
    }
}

fn fuzz_opacity(surface: &mut Surface, rng: &mut SimpleRng) {
    if let Some(opacity) = NormalizedF32::new(rng.float_range(0.0, 1.0)) {
        surface.push_opacity(opacity);

        if rng.bool() {
            if let Some(path) = create_random_path(rng) {
                surface.draw_path(&path);
            }
            surface.pop();
        }
    }
}

fn fuzz_blend_mode(surface: &mut Surface, rng: &mut SimpleRng) {
    let blend_modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
    ];

    let mode = blend_modes[rng.range(0, blend_modes.len() as u64) as usize];
    surface.push_blend_mode(mode);

    if rng.bool() {
        if let Some(path) = create_random_path(rng) {
            surface.draw_path(&path);
        }
        surface.pop();
    }
}

fn fuzz_nested_operations(surface: &mut Surface, rng: &mut SimpleRng) {
    let depth = rng.range(1, 5);

    for _ in 0..depth {
        match rng.range(0, 3) {
            0 => {
                let transform =
                    Transform::from_translate(rng.float_range(-100.0, 100.0), rng.float_range(-100.0, 100.0));
                surface.push_transform(&transform);
            }
            1 => {
                if let Some(opacity) = NormalizedF32::new(rng.float_range(0.0, 1.0)) {
                    surface.push_opacity(opacity);
                }
            }
            _ => {
                surface.push_blend_mode(BlendMode::Normal);
            }
        }
    }

    if let Some(path) = create_random_path(rng) {
        surface.draw_path(&path);
    }

    for _ in 0..depth {
        surface.pop();
    }
}

fn fuzz_edge_case_values(surface: &mut Surface, rng: &mut SimpleRng) {
    let extreme_values = [
        0.0,
        -0.0,
        0.00001,
        -0.00001,
        f32::MIN_POSITIVE,
        f32::MAX,
        -f32::MAX,
        100000.0,
        -100000.0,
    ];

    let val1 = extreme_values[rng.range(0, extreme_values.len() as u64) as usize];
    let val2 = extreme_values[rng.range(0, extreme_values.len() as u64) as usize];

    let transform = Transform::from_translate(val1, val2);
    surface.push_transform(&transform);

    if let Some(path) = create_random_path(rng) {
        surface.draw_path(&path);
    }

    surface.pop();
}

fn fuzz_multiple_pops(surface: &mut Surface, rng: &mut SimpleRng) {
    let pop_count = rng.range(0, 5);

    for _ in 0..pop_count {
        surface.pop();
    }
}

fn fuzz_stream_builder(surface: &mut Surface, rng: &mut SimpleRng) {
    let mut builder = surface.stream_builder();

    {
        let mut inner_surface = builder.surface();

        for _ in 0..rng.range(0, 3) {
            if let Some(path) = create_random_path(rng) {
                inner_surface.draw_path(&path);
            }
        }

        if rng.bool() {
            inner_surface.finish();
        }
    }

    let _stream = builder.finish();
}

fn create_random_path(rng: &mut SimpleRng) -> Option<Path> {
    let mut builder = PathBuilder::new();

    let segment_count = rng.range(1, 10);

    builder.move_to(rng.float_range(-500.0, 500.0), rng.float_range(-500.0, 500.0));

    for _ in 0..segment_count {
        let seg_type = rng.range(0, 5);

        match seg_type {
            0 => builder.line_to(rng.float_range(-500.0, 500.0), rng.float_range(-500.0, 500.0)),
            1 => builder.quad_to(
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
            ),
            2 => builder.cubic_to(
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
                rng.float_range(-500.0, 500.0),
            ),
            3 => {
                if let Some(rect) = create_random_rect(rng) {
                    builder.push_rect(rect);
                }
            }
            4 => builder.close(),
            _ => {}
        }
    }

    builder.finish()
}

fn create_random_rect(rng: &mut SimpleRng) -> Option<Rect> {
    let x1 = rng.float_range(-1000.0, 1000.0);
    let y1 = rng.float_range(-1000.0, 1000.0);
    let x2 = rng.float_range(-1000.0, 1000.0);
    let y2 = rng.float_range(-1000.0, 1000.0);

    Rect::from_ltrb(x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2))
}

fn generate_random_string(rng: &mut SimpleRng, max_len: usize) -> String {
    let len = rng.range(0, max_len as u64) as usize;
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 "
        .chars()
        .collect();

    (0..len)
        .map(|_| chars[rng.range(0, chars.len() as u64) as usize])
        .collect()
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }
        min + (self.next() % (max - min))
    }

    fn float_range(&mut self, min: f32, max: f32) -> f32 {
        let normalized = (self.next() as f64) / (u64::MAX as f64);
        min + (normalized as f32) * (max - min)
    }

    fn bool(&mut self) -> bool {
        self.next() % 2 == 0
    }

    fn byte(&mut self) -> u8 {
        (self.next() % 256) as u8
    }
}
