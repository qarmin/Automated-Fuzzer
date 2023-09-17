use crate::settings::Setting;
use jwalk::WalkDir;
use log::info;
use std::fs;
use std::path::Path;
use std::process::{Output, Stdio};

pub fn check_differences(setting: &Setting) {
    // let mut files_to_check = collect_files();
    // println!("Found {} files to check", files_to_check.len());

    info!("Start dir {} have {} files", &setting.start_dir, collect_number_of_files(&setting.start_dir));

    copy_files_from_start_dir_to_test_dir(setting);

    run_ruff(&setting.test_dir);

    let output = run_black(&setting.test_dir);
    copy_broken_black_files(output, setting);

    copy_broken_files_to_test_dir(setting);

    run_ruff(&setting.test_dir);

    copy_broken_files_with_ruff(setting);

    run_diff_on_files(setting);
}

fn collect_number_of_files(dir: &str) -> usize {
    WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|i| {
            let path = i.path();
            if path.is_dir() {
                return false;
            }
            let file_name = path.to_str().unwrap();
            file_name.ends_with(".py")
        })
        .count()
}

fn run_diff_on_files(setting: &Setting) {
    let black_files: Vec<_> = WalkDir::new(&setting.broken_files_dir)
        .into_iter()
        .flatten()
        .filter(|i| {
            let path = i.path();
            if path.is_dir() {
                return false;
            }
            let file_name = path.to_str().unwrap();
            file_name.ends_with("_black.py")
        })
        .collect();

    info!("Starting to run diff on files");
    black_files.into_iter().for_each(|black_dir_entry| {
        let black_file = black_dir_entry.path().to_str().unwrap().to_string();
        let ruff_file = black_file.replace("_black", "_ruff");
        let result_file = black_file.replace("_black", "_diff");

        if !(Path::new(&ruff_file).is_file()) {
            let _ = fs::remove_file(&black_file);
            let _ = fs::remove_file(&ruff_file);
            let _ = fs::remove_file(&result_file);
            return;
        }

        let diff_output = std::process::Command::new("diff")
            .arg("-u")
            .arg(&black_file)
            .arg(&ruff_file)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .wait_with_output()
            .unwrap();
        let out = String::from_utf8_lossy(&diff_output.stdout);
        let err = String::from_utf8_lossy(&diff_output.stderr);
        let all = format!("{}\n{}", out, err);

        if all.trim().is_empty() {
            let _ = fs::remove_file(&black_file);
            let _ = fs::remove_file(&ruff_file);
            let _ = fs::remove_file(&result_file);
        } else {
            fs::write(&result_file, all.trim().as_bytes()).unwrap();
        }
    });

    info!("Finished running diff on files");
}

fn copy_broken_files_to_test_dir(setting: &Setting) {
    let _ = fs::remove_dir_all(&setting.test_dir);
    fs::create_dir_all(&setting.test_dir).unwrap();
    info!("Removed and created test_dir");

    info!("Starting to copy files with differences to test_dir, to check them again with ruff");
    for file in WalkDir::new(&setting.broken_files_dir)
        .into_iter()
        .flatten()
    {
        let path = file.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name.replace(&setting.broken_files_dir, &setting.test_dir);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied files with differences to test_dir");
}

fn copy_broken_black_files(black_output: Output, setting: &Setting) {
    let out = String::from_utf8_lossy(&black_output.stdout);
    let err = String::from_utf8_lossy(&black_output.stderr);
    let all = format!("{}\n{}", out, err);

    let _ = fs::remove_dir_all(&setting.broken_files_dir);
    fs::create_dir_all(&setting.broken_files_dir).unwrap();
    info!("Created broken_files_dir");

    info!("Starting to copy black files with differences to broken_files_dir");
    for (idx, line) in all.lines().enumerate() {
        let Some(file_name) = line.strip_prefix("reformatted ") else {
            continue;
        };
        let new_full_name = format!("{}{}_black.py", &setting.broken_files_dir, idx);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied black files with differences to broken_files_dir");
}

fn copy_broken_files_with_ruff(setting: &Setting) {
    info!("Starting to copy ruff files with differences to broken_files_dir");
    for i in WalkDir::new(&setting.test_dir).into_iter().flatten() {
        let path = i.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name
            .replace("_black", "_ruff")
            .replace(&setting.test_dir, &setting.broken_files_dir);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied ruff files with differences to broken_files_dir");
}

fn run_ruff(dir: &str) -> Output {
    info!("Running ruff on dir: {dir}", );
    let output = std::process::Command::new("ruff")
        .arg("format")
        .arg(dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    info!("Ruff formatted files");
    output
}

fn run_black(dir: &str) -> Output {
    info!("Running black");
    let black_output = std::process::Command::new("black")
        .arg(dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    info!("Black formatted files");
    black_output
}

fn copy_files_from_start_dir_to_test_dir(setting: &Setting) {
    info!("Starting to copy files to check");
    let _ = fs::remove_dir_all(&setting.test_dir);
    fs::create_dir_all(&setting.test_dir).unwrap();

    for file in WalkDir::new(&setting.start_dir).into_iter().flatten() {
        let path = file.path();
        if path.is_dir() {
            continue;
        }
        let file_name = path.to_str().unwrap();
        let new_full_name = file_name.replace(&setting.start_dir, &setting.test_dir);
        let parent = Path::new(&new_full_name).parent().unwrap();
        let _ = fs::create_dir_all(parent);
        fs::copy(file_name, new_full_name).unwrap();
    }
    info!("Copied files to {}", &setting.test_dir);
}
