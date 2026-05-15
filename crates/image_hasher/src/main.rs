use std::{fs, panic};

fn main() {
    fuzz_utils::run(check_file);
}

fn check_file(file_path: &str) {
    let Ok(content) = fs::read(file_path) else {
        return;
    };

    // Not interested in panic in image crate
    let res = panic::catch_unwind(|| image::load_from_memory(&content).ok());
    let dynamic_image = match res {
        Ok(res) => match res {
            Some(res) => res,
            None => return,
        },
        Err(_e) => {
            return;
        }
    };

    let hash_size = [(8, 8), (16, 16), (32, 32), (64, 64)];
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
            let hasher = image_hasher::HasherConfig::new()
                .hash_size(size.0, size.1)
                .hash_alg(*alg)
                .to_hasher();
            let _ = hasher.hash_image(&dynamic_image);
        }
    }
}
