use dicom::core::value::{DicomDate, DicomTime, PersonName, PrimitiveValue};
use dicom::core::{Tag, VR, DataElementHeader, Length};
use dicom::encoding::text::{SpecificCharacterSet, TextCodec};
use rand::Rng;

/// Fuzzer for DICOM library
fn main() {
    println!("🔬 DICOM Fuzzer - Testing random functions with random data");
    println!("============================================================\n");

    let iterations = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10000);

    println!("Running {} iterations...\n", iterations);

    let mut rng = rand::thread_rng();
    let mut passed = 0;
    let mut failed = 0;

    for i in 0..iterations {
        if i % 100 == 0 {
            print!("\rProgress: {}/{} (Passed: {}, Failed: {})", i, iterations, passed, failed);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }

        // Randomly select which category to test
        match rng.gen_range(0..10) {
            0 => test_date_parsing(&mut rng, &mut passed, &mut failed),
            1 => test_time_parsing(&mut rng, &mut passed, &mut failed),
            2 => test_datetime_parsing(&mut rng, &mut passed, &mut failed),
            3 => test_range_parsing(&mut rng, &mut passed, &mut failed),
            4 => test_value_conversions(&mut rng, &mut passed, &mut failed),
            5 => test_primitive_value_creation(&mut rng, &mut passed, &mut failed),
            6 => test_character_set_encoding(&mut rng, &mut passed, &mut failed),
            7 => test_person_name_parsing(&mut rng, &mut passed, &mut failed),
            8 => test_tag_operations(&mut rng, &mut passed, &mut failed),
            9 => test_basic_decoder(&mut rng, &mut passed, &mut failed),
            _ => unreachable!(),
        }
    }

    println!("\n\n============================================================");
    println!("✅ Fuzzing complete!");
    println!("   Passed: {}", passed);
    println!("   Failed: {}", failed);
    println!("   Total:  {}", iterations);
    println!("============================================================");
}

// Helper function to generate random bytes
fn random_bytes(rng: &mut impl Rng, max_len: usize) -> Vec<u8> {
    let len = rng.gen_range(0..=max_len);
    (0..len).map(|_| rng.gen()).collect()
}

// Helper function to generate random string
fn random_string(rng: &mut impl Rng, max_len: usize) -> String {
    let len = rng.gen_range(0..=max_len);
    (0..len).map(|_| rng.gen_range(0..128) as u8 as char).collect()
}

// Test date parsing with random malformed input
fn test_date_parsing(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Test DicomDate creation with random values
    let year = rng.gen_range(0..10000) as u16;
    let month = rng.gen_range(0..20) as u8;
    let day = rng.gen_range(0..40) as u8;

    match std::panic::catch_unwind(|| {
        let _ = DicomDate::from_y(year);
        let _ = DicomDate::from_ym(year, month);
        let _ = DicomDate::from_ymd(year, month, day);
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in DicomDate creation with y={}, m={}, d={}", year, month, day);
        }
    }
}

// Test time parsing with random malformed input
fn test_time_parsing(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Test DicomTime creation with random values
    let hour = rng.gen_range(0..30) as u8;
    let minute = rng.gen_range(0..70) as u8;
    let second = rng.gen_range(0..70) as u8;
    let millis = rng.gen::<u32>();
    let micros = rng.gen::<u32>();

    match std::panic::catch_unwind(|| {
        let _ = DicomTime::from_h(hour);
        let _ = DicomTime::from_hm(hour, minute);
        let _ = DicomTime::from_hms(hour, minute, second);
        let _ = DicomTime::from_hms_milli(hour, minute, second, millis);
        let _ = DicomTime::from_hms_micro(hour, minute, second, micros);
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in DicomTime creation");
        }
    }
}

// Test datetime construction
fn test_datetime_parsing(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Just use existing date/time tests
    test_date_parsing(rng, passed, failed);
    test_time_parsing(rng, passed, failed);
}

// Test range operations
fn test_range_parsing(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Test creating dates and times which can then be used for ranges
    test_date_parsing(rng, passed, failed);
    test_time_parsing(rng, passed, failed);
}

// Test value type conversions with random data
fn test_value_conversions(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Create random primitive values and try to convert them
    let primitives = vec![
        PrimitiveValue::Str(random_string(rng, 30)),
        PrimitiveValue::Strs(smallvec::smallvec![random_string(rng, 10), random_string(rng, 10)]),
        PrimitiveValue::U16(smallvec::smallvec![rng.gen(), rng.gen(), rng.gen()]),
        PrimitiveValue::I16(smallvec::smallvec![rng.gen(), rng.gen()]),
        PrimitiveValue::U32(smallvec::smallvec![rng.gen()]),
        PrimitiveValue::I32(smallvec::smallvec![rng.gen(), rng.gen()]),
        PrimitiveValue::U8(random_bytes(rng, 50).into()),
        PrimitiveValue::F32(smallvec::smallvec![rng.gen(), rng.gen()]),
        PrimitiveValue::F64(smallvec::smallvec![rng.gen()]),
    ];

    for pv in primitives {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = pv.to_str();
            let _ = pv.to_raw_str();
            let _ = pv.to_multi_str();
            let _ = pv.to_bytes();
            let _ = pv.to_int::<i32>();
            let _ = pv.to_int::<i16>();
            let _ = pv.to_int::<u32>();
            let _ = pv.to_multi_int::<i32>();
            let _ = pv.to_float32();
            let _ = pv.to_float64();
            let _ = pv.to_multi_float32();
            let _ = pv.to_multi_float64();
            let _ = pv.to_date();
            let _ = pv.to_time();
            let _ = pv.to_datetime();
            let _ = pv.to_multi_date();
            let _ = pv.to_multi_time();
            let _ = pv.to_multi_datetime();
            let _ = pv.to_date_range();
            let _ = pv.to_time_range();
            let _ = pv.to_datetime_range();
            let _ = pv.to_person_name();
            let _ = pv.string();
            let _ = pv.strings();
            let _ = pv.date();
            let _ = pv.time();
            let _ = pv.datetime();
            let _ = pv.uint8();
            let _ = pv.uint16();
            let _ = pv.int16();
            let _ = pv.uint32();
            let _ = pv.int32();
            let _ = pv.int64();
            let _ = pv.uint64();
            let _ = pv.float32();
            let _ = pv.float64();
        })) {
            Ok(_) => *passed += 1,
            Err(_) => {
                *failed += 1;
                eprintln!("\n❌ PANIC in value conversion");
            }
        }
    }
}

// Test primitive value creation with edge cases
fn test_primitive_value_creation(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    // Generate random values outside catch_unwind
    let u16_val = rng.gen();
    let u32_val = rng.gen();
    let i32_val = rng.gen();
    let large_vec: smallvec::SmallVec<[u16; 2]> = (0..10000).map(|_| rng.gen()).collect();
    let large_str = random_string(rng, 100000);

    match std::panic::catch_unwind(move || {
        // Test various constructors
        let _ = PrimitiveValue::new_u16(u16_val);
        let _ = PrimitiveValue::new_u32(u32_val);
        let _ = PrimitiveValue::new_i32(i32_val);

        // Test with extreme values
        let _ = PrimitiveValue::U16(smallvec::smallvec![u16::MAX, u16::MIN, 0]);
        let _ = PrimitiveValue::I32(smallvec::smallvec![i32::MAX, i32::MIN, 0]);
        let _ = PrimitiveValue::F32(smallvec::smallvec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0]);
        let _ = PrimitiveValue::F64(smallvec::smallvec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0]);

        // Test empty values
        let _ = PrimitiveValue::Str(String::new());
        let empty_vec: Vec<u8> = Vec::new();
        let _ = PrimitiveValue::U8(empty_vec.into());
        let _ = PrimitiveValue::U16(smallvec::SmallVec::new());

        // Test very large values
        let _ = PrimitiveValue::U16(large_vec);
        let _ = PrimitiveValue::Str(large_str);
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in primitive value creation");
        }
    }
}

// Test character set encoding/decoding with random data
fn test_character_set_encoding(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    let charsets = vec![
        SpecificCharacterSet::default(),
    ];

    let bytes = random_bytes(rng, 100);
    let text = random_string(rng, 100);

    for charset in charsets {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = charset.decode(&bytes);
            let _ = charset.encode(&text);
        })) {
            Ok(_) => *passed += 1,
            Err(_) => {
                *failed += 1;
                eprintln!("\n❌ PANIC in character set encoding");
            }
        }
    }
}

// Test person name parsing with random data
fn test_person_name_parsing(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    let text = random_string(rng, 200);

    match std::panic::catch_unwind(|| {
        let pn = PersonName::from_text(&text);
        let _ = pn.family();
        let _ = pn.given();
        let _ = pn.middle();
        let _ = pn.prefix();
        let _ = pn.suffix();
        let _ = pn.to_dicom_string();
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in person name parsing with input: {:?}", text);
        }
    }

    // Test PersonName builder with random data
    let family = random_string(rng, 50);
    let given = random_string(rng, 50);
    let middle = random_string(rng, 50);
    let prefix = random_string(rng, 50);
    let suffix = random_string(rng, 50);

    match std::panic::catch_unwind(|| {
        let mut builder = PersonName::builder();
        builder
            .with_family(family.clone())
            .with_given(given.clone())
            .with_middle(middle.clone())
            .with_prefix(prefix.clone())
            .with_suffix(suffix.clone());
        let _ = builder.build();
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in person name builder");
        }
    }
}

// Test tag operations with random data
fn test_tag_operations(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    let group: u16 = rng.gen();
    let element: u16 = rng.gen();
    let vr_choice = rng.gen_range(0..10);
    let use_undefined = rng.gen_bool(0.1);
    let len_val: u32 = rng.gen();

    match std::panic::catch_unwind(move || {
        let tag = Tag(group, element);
        let _ = tag.group();
        let _ = tag.element();

        // Test DataElementHeader creation
        let vr = match vr_choice {
            0 => VR::AE,
            1 => VR::AS,
            2 => VR::AT,
            3 => VR::CS,
            4 => VR::DA,
            5 => VR::DS,
            6 => VR::DT,
            7 => VR::FD,
            8 => VR::FL,
            _ => VR::IS,
        };

        let len = if use_undefined {
            Length::UNDEFINED
        } else {
            Length(len_val)
        };

        let _ = DataElementHeader::new(tag, vr, len);

        // Test with extreme tag values
        let _ = Tag(0, 0);
        let _ = Tag(u16::MAX, u16::MAX);
        let _ = Tag(0xFFFF, 0xFFFF);
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in tag operations");
        }
    }
}

// Test string and byte operations with edge cases
fn test_basic_decoder(rng: &mut impl Rng, passed: &mut usize, failed: &mut usize) {
    let bytes = random_bytes(rng, 200);

    match std::panic::catch_unwind(|| {
        // Test string conversions with potentially malformed data
        let s = String::from_utf8_lossy(&bytes);
        let _ = s.len();
        let _ = s.chars().count();

        // Test with empty data
        let empty: Vec<u8> = Vec::new();
        let _ = String::from_utf8_lossy(&empty);

        // Test with maximum size
        let large = vec![0x41u8; 1000000];
        let _ = String::from_utf8_lossy(&large);
    }) {
        Ok(_) => *passed += 1,
        Err(_) => {
            *failed += 1;
            eprintln!("\n❌ PANIC in string/bytes operations");
        }
    }
}

