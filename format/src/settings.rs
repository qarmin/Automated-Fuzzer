use config::Config;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Setting {
    pub mode: String,
    pub start_dir: String,
    pub test_dir: String,
    pub test_dir2: String,
    pub broken_files_dir: String,
    pub black_timeout: u64,
    pub depth: usize,
}
pub fn load_settings() -> Setting {
    let settings = Config::builder()
        .add_source(config::File::with_name("settings"))
        .build()
        .unwrap();
    let config = settings
        .try_deserialize::<HashMap<String, HashMap<String, String>>>()
        .unwrap();

    let general = config["general"].clone();

    Setting {
        mode: general["mode"].clone(),
        start_dir: general["start_dir"].clone(),
        test_dir: general["test_dir"].clone(),
        test_dir2: general["test_dir2"].clone(),
        broken_files_dir: general["broken_files_dir"].clone(),
        black_timeout: general["black_timeout"].parse::<u64>().unwrap(),
        depth: general["depth"].parse::<usize>().unwrap(),
    }
}
