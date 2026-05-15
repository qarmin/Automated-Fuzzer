#![no_main]

use std::panic;
use libfuzzer_sys::{fuzz_target, Corpus};

fuzz_target!(|data: &[u8]| -> Corpus {
    let res = panic::catch_unwind(||{
    match image_crates::load_from_memory(&data) {
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
                None => return Corpus::Reject,
            }
        },
        Err(_e) => {
            return Corpus::Reject;
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

    Corpus::Keep
});
