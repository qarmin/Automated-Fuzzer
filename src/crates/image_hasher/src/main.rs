use std::env::args;
use std::{fs, panic};
use std::path::Path;

use walkdir::WalkDir;

fn main() {
    let path = args().nth(1).unwrap().clone();
    if !Path::new(&path).exists() {
        panic!("Missing file, {path:?}");
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

    // Not interested in panic in image crate
    let res = panic::catch_unwind(||{
        match image::load_from_memory(&content) {
            Ok(res) => Some(res),
            Err(_e) => {
                None
            }
        }
    });
    let dynamic_image = match res {
        Ok(res) => {
            match res {
                Some(res) => res,
                None => return,
            }
        },
        Err(_e) => {
            return;
        }
    };

    let hash_size = [(8,8), (16, 16), (32, 32), (64, 64)];
    let hash_alg = [
        image_hasher::HashAlg::Mean,
        image_hasher::HashAlg::Median,
        image_hasher::HashAlg::Gradient,
        image_hasher::HashAlg::DoubleGradient,
        image_hasher::HashAlg::VertGradient,
        image_hasher::HashAlg::Blockhash,
    ];
    for size in hash_size.iter() {
        for alg in hash_alg.iter() {
            let hasher = image_hasher::HasherConfig::new().hash_size(size.0, size.1).hash_alg(*alg).to_hasher();
            let _ = hasher.hash_image(&dynamic_image);
        }
    }
}
