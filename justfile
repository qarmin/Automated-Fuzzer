upgrade:
    cargo +nightly -Z unstable-options update --breaking
    cargo update
    for dir in */; do (cd "$dir" && cargo +nightly -Z unstable-options update --breaking); done
    for dir in */; do (cd "$dir" && cargo update); done

fix:
    cargo +nightly fmt
    cargo clippy --fix --allow-dirty --allow-staged -- -Wclippy::pedantic -Aclippy::default_trait_access -Aclippy::cast_possible_truncation -Aclippy::must_use_candidate -Aclippy::missing_panics_doc -Aclippy::too_many_lines -Aclippy::cast_precision_loss -Aclippy::cast_sign_loss -Aclippy::module_name_repetitions -Aclippy::struct_excessive_bools -Aclippy::cast_possible_wrap -Aclippy::cast_lossless -Aclippy::if_not_else -Aclippy::wildcard_imports -Aclippy::return_self_not_must_use -Aclippy::missing_errors_doc -Aclippy::match_wildcard_for_single_variants -Aclippy::assigning_clones -Aclippy::unused_self -Aclippy::manual_is_variant_and -Aclippy::new_without_default
    cargo +nightly fmt
    cargo fmt

fixn:
    cargo +nightly fmt
    cargo +nightly clippy --fix --allow-dirty --allow-staged -- -Wclippy::pedantic -Aclippy::default_trait_access -Aclippy::cast_possible_truncation -Aclippy::must_use_candidate -Aclippy::missing_panics_doc -Aclippy::too_many_lines -Aclippy::cast_precision_loss -Aclippy::cast_sign_loss -Aclippy::module_name_repetitions -Aclippy::struct_excessive_bools -Aclippy::cast_possible_wrap -Aclippy::cast_lossless -Aclippy::if_not_else -Aclippy::wildcard_imports -Aclippy::return_self_not_must_use -Aclippy::missing_errors_doc -Aclippy::match_wildcard_for_single_variants -Aclippy::assigning_clones -Aclippy::unused_self -Aclippy::manual_is_variant_and -Aclippy::new_without_default
    cargo +nightly fmt
    cargo fmt