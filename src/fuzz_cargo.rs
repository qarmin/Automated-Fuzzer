use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

use log::info;

use crate::SHOULD_STOP;

pub fn run_cargo_fuzz(target: &str, corpus: &str, timeout: u64, features: Option<&str>, jobs: u32) {
    info!("Starting cargo-fuzz mode for target: {target}");

    let mut cmd = Command::new("cargo");
    cmd.arg("fuzz").arg("run").arg(target);

    if !corpus.is_empty() {
        cmd.arg(corpus);
    }

    cmd.arg(format!("-j{jobs}")).arg("--release");

    if let Some(feat) = features {
        cmd.args(["--features", feat]);
    }

    cmd.arg("--")
        .arg("-max_len=99999")
        .arg(format!("-max_total_time={timeout}"))
        .arg("-rss_limit_mb=20000");

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    // Put cargo-fuzz in its own process group so that on Ctrl+C we can kill
    // the entire tree (cargo → cargo-fuzz → libfuzzer workers) with one signal.
    cmd.process_group(0);

    info!("Running: cargo fuzz run {target} ...");

    let mut child = cmd.spawn().expect("Failed to start cargo fuzz");

    // Wait for completion, checking for stop signal
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    info!("cargo-fuzz completed successfully (no crashes found or time expired)");
                } else {
                    info!("cargo-fuzz exited with status: {status} (crashes may have been found)");
                }
                break;
            }
            Ok(None) => {
                if SHOULD_STOP.load(Ordering::Relaxed) {
                    info!("Stop requested, sending SIGTERM to cargo-fuzz process group...");
                    signal_child_group(&child);
                    let _ = child.wait();
                    info!("cargo-fuzz stopped.");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(e) => {
                eprintln!("Error waiting for cargo-fuzz: {e}");
                break;
            }
        }
    }

    // Collect results from fuzz/artifacts/
    collect_cargo_fuzz_results(target);
}

fn collect_cargo_fuzz_results(target: &str) {
    let artifacts_dir = format!("fuzz/artifacts/{target}");
    let path = std::path::Path::new(&artifacts_dir);

    if !path.exists() {
        info!("No artifacts directory found at {artifacts_dir}");
        return;
    }

    let mut crash_count = 0;
    let mut slow_count = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("slow-") {
                slow_count += 1;
            } else {
                crash_count += 1;
            }
        }
    }

    info!("Found {crash_count} crash artifacts and {slow_count} slow-unit artifacts in {artifacts_dir}");

    if slow_count > 0 {
        info!("Removing {slow_count} slow-unit files...");
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("slow-") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// Send SIGTERM to the cargo-fuzz process group on Unix. Logs any failure so
/// CI debugging doesn't have to guess whether the signal was delivered.
fn signal_child_group(child: &std::process::Child) {
    let pid = child.id() as i32;
    // Use negative PID to signal the entire process group; relies on
    // `cmd.process_group(0)` having put the child in its own group.
    let rc = unsafe { libc::kill(-pid, libc::SIGTERM) };
    if rc < 0 {
        log::warn!(
            "kill(-{pid}, SIGTERM) failed: {}",
            std::io::Error::last_os_error()
        );
    }
}
