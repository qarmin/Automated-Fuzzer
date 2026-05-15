
use fast_image_resize::images::{Image, TypedCroppedImage};
use fast_image_resize::{FilterType, IntoImageView, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::*;

fn main() {
    fuzz_utils::run(check_file);
}

const FILTER_TYPES: &[FilterType] = &[
    FilterType::Box,
    FilterType::Bilinear,
    FilterType::Hamming,
    FilterType::CatmullRom,
    FilterType::Mitchell,
    FilterType::Gaussian,
    FilterType::Lanczos3,
];

const PIXEL_TYPES: &[PixelType] = &[
    PixelType::U8,
    PixelType::U8x2,
    PixelType::U8x3,
    PixelType::U8x4,
    PixelType::U16,
    PixelType::U16x2,
    PixelType::U16x3,
    PixelType::U16x4,
    PixelType::I32,
    PixelType::F32,
    PixelType::F32x2,
    PixelType::F32x3,
    PixelType::F32x4,
];

fn check_file(file_path: &str) {
    let Ok(mut src_image) = ImageReader::open(file_path) else {
        eprintln!("Error while reading image: {:?}", file_path);
        return;
    };
    let Ok((decoded, _metadata)) = src_image.decode() else {
        eprintln!("Error while decoding image: {:?} (probably not image)", file_path);
        return;
    };

    let mut all = vec![];
    all.push(ResizeAlg::Nearest);
    for filter_type in FILTER_TYPES {
        all.push(ResizeAlg::Interpolation(*filter_type));
        all.push(ResizeAlg::Convolution(*filter_type));
        for num in [2, 15] {
            all.push(ResizeAlg::SuperSampling(*filter_type, num));
        }
    }

    for pixel_type in PIXEL_TYPES {
        let w = decoded.width() % 300;
        let h = decoded.height() % 300;

        for (width, height) in [
            (0, 0),
            (1, 1),
            (300, 1),
            (1, 300),
            (500, 50),
            (50, 500),
            (w, h),
            (w * 2, h * 2),
            (w / 2, h / 2),
        ] {
            for resize_alg in &all {
                println!(
                    "Checking file: {:?}, width: {:?}, height: {:?}, pixel_type: {:?}, resize_alg: {:?}",
                    file_path, width, height, pixel_type, resize_alg
                );
                let mut dst_image = Image::new(width, height, *pixel_type);

                let resize_options = ResizeOptions::new().resize_alg(*resize_alg);
                if let Err(e) = Resizer::new().resize(&decoded, &mut dst_image, Some(&resize_options)) {
                    eprintln!("Error while resizing image: {:?}", e);
                };

                let resize_options = ResizeOptions::new()
                    .crop(-1.0, -1.0, 1.0, 1.0)
                    .fit_into_destination(Some((-15.0, 25.0)))
                    .use_alpha(true);
                if let Err(e) = Resizer::new().resize(&decoded, &mut dst_image, Some(&resize_options)) {
                    eprintln!("Error while resizing image(2): {:?}", e);
                };

                if let Some(image_view) = dst_image.image_view::<fast_image_resize::pixels::Pixel<[u8; 3], u8, 3>>() {
                    if let Err(e) = TypedCroppedImage::new(image_view, 0, 0, 1, 1) {
                        eprintln!("Error while creating TypedCroppedImage: {:?}", e);
                    };
                };
            }
        }
    }
}
