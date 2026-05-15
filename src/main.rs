#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::borrowed_box)]
#![allow(clippy::too_many_arguments)]

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};
use log::info;

use crate::common::{calculate_number_of_files, check_files_number, set_timeout};
use crate::finding_different_output::find_broken_files_by_different_output;
use crate::finding_text_status::find_broken_files_by_text_status;
use crate::fuzz_cargo::run_cargo_fuzz;
use crate::ignore_list::{IgnoreEntry, IgnoreList};
use crate::settings::{StabilityMode, get_object, load_settings, load_settings_from_path};

mod apps;
mod broken_files;
mod ci;
mod common;
mod error_signature;
mod finding_different_output;
mod finding_text_status;
mod fuzz_cargo;
mod ignore_list;
mod minimize_cmd;
mod obj;
mod remove_non_crashing_files;
mod report;
mod settings;
mod validate;

pub static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[command(name = "auto_fuzzer", about = "Automated fuzzing tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run fuzzing (custom or cargo-fuzz mode)
    Fuzz {
        /// Fuzzing mode: "custom" or "cargo-fuzz"
        #[arg(long, default_value = "custom")]
        mode: String,

        /// Path to config file
        #[arg(long, default_value = "fuzz_settings.toml")]
        config: String,

        /// Total fuzzing timeout in seconds (0 = no limit)
        #[arg(long, default_value_t = 0)]
        timeout: u64,

        /// Cargo-fuzz target name (required for cargo-fuzz mode)
        #[arg(long)]
        target: Option<String>,

        /// Corpus directory (for cargo-fuzz mode)
        #[arg(long)]
        corpus: Option<String>,

        /// Extra features for cargo-fuzz
        #[arg(long)]
        features: Option<String>,

        /// Number of parallel jobs for cargo-fuzz
        #[arg(long, default_value_t = 4)]
        jobs: u32,
    },

    /// Minimize found crash files (without deleting originals)
    Minimize {
        /// Path to config file
        #[arg(long, default_value = "fuzz_settings.toml")]
        config: String,

        /// Directory with broken files (overrides config)
        #[arg(long)]
        dir: Option<String>,

        /// Command template with {} placeholder (overrides config)
        #[arg(long)]
        command: Option<String>,
    },

    /// Generate and manage crash reports
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    /// Manage ignore list for known bugs
    Ignore {
        #[command(subcommand)]
        action: IgnoreAction,
    },

    /// CI mode with state management between runs
    Ci {
        #[command(subcommand)]
        action: CiAction,
    },

    /// Legacy mode - run like old automated_fuzzer [TIMEOUT]
    Legacy {
        /// Timeout in seconds
        #[arg(default_value_t = 999_999_999_999)]
        timeout: u64,

        /// Run remove_non_crashing mode instead
        #[arg(long)]
        remove_non_crashing: bool,
    },
}

#[derive(Subcommand)]
enum ReportAction {
    /// List all unreported crashes
    List {
        /// Filter by project name
        #[arg(long)]
        project: Option<String>,

        /// Results directory
        #[arg(long, default_value = "results")]
        dir: String,
    },
    /// Generate issue report for a crash directory
    Create {
        /// Path to crash result directory
        #[arg(long)]
        dir: String,

        /// Target GitHub repo (e.g. "Serial-ATA/lofty-rs")
        #[arg(long)]
        repo: Option<String>,

        /// Project version or commit hash
        #[arg(long)]
        version: Option<String>,

        /// Report variant: "cli" or "library"
        #[arg(long, default_value = "cli")]
        variant: String,
    },
    /// Generate reports for all unreported crashes
    CreateAll {
        /// Filter by project name
        #[arg(long)]
        project: Option<String>,

        /// Results directory
        #[arg(long, default_value = "results")]
        dir: String,
    },
}

#[derive(Subcommand)]
enum IgnoreAction {
    /// Add an entry to the ignore list
    Add {
        /// Project name
        project: String,
        /// Pattern to match in crash output
        pattern: String,
        /// URL to the GitHub issue
        issue_url: String,
    },
    /// Remove an entry from the ignore list
    Remove {
        /// Project name
        project: String,
        /// Pattern to remove
        pattern: String,
    },
    /// List all ignored patterns (from ignore_list.toml and fuzz configs)
    List {
        /// Filter by project name
        #[arg(long)]
        project: Option<String>,
    },
    /// Check if ignored issues have been closed (read-only)
    Verify,
    /// Check and remove entries for closed issues
    Clean,
}

#[derive(Subcommand)]
enum CiAction {
    /// Run fuzzer in CI mode with state persistence
    Run {
        /// Path to config file
        #[arg(long, default_value = "fuzz_settings.toml")]
        config: String,

        /// Total fuzzing timeout in seconds
        #[arg(long)]
        timeout: u64,

        /// Directory for persistent state between CI runs
        #[arg(long)]
        state_dir: String,

        /// Directory for output results
        #[arg(long, default_value = "results")]
        output_dir: String,

        /// Fuzzing mode: "custom" or "cargo-fuzz"
        #[arg(long, default_value = "custom")]
        mode: String,

        /// Cargo-fuzz target name (required for cargo-fuzz mode)
        #[arg(long)]
        target: Option<String>,
    },
    /// Verify if previously found crashes are still reproducible
    VerifyRegressions {
        /// Path to config file
        #[arg(long, default_value = "fuzz_settings.toml")]
        config: String,

        /// Directory with persistent state
        #[arg(long)]
        state_dir: String,
    },
}

fn main() {
    handsome_logger::init().unwrap();

    // Setup CTRL+C handler
    ctrlc::set_handler(|| {
        if SHOULD_STOP.load(Ordering::Relaxed) {
            eprintln!("\nForce quit.");
            std::process::exit(1);
        }
        eprintln!("\nGraceful shutdown requested. Finishing current work...");
        eprintln!("Press Ctrl+C again to force quit.");
        SHOULD_STOP.store(true, Ordering::Relaxed);
    })
    .expect("Failed to set Ctrl+C handler");

    let cli = Cli::parse();

    match cli.command {
        Commands::Fuzz {
            mode,
            config,
            timeout,
            target,
            corpus,
            features,
            jobs,
        } => {
            let effective_timeout = if timeout == 0 { 999_999_999_999 } else { timeout };
            set_timeout(effective_timeout);

            match mode.as_str() {
                "custom" => {
                    let settings = load_settings_from(&config);
                    let mut obj = get_object(settings.clone());
                    obj.init();

                    let _ = fs::create_dir_all(&settings.temp_folder);
                    let _ = fs::create_dir_all(&settings.broken_files_dir);
                    let _ = fs::create_dir_all(&settings.custom_folder_path);

                    check_files_number("Valid input dir", &settings.valid_input_files_dir);
                    assert!(Path::new(&settings.valid_input_files_dir).exists());

                    info!(
                        "Found {} files in valid input dir",
                        calculate_number_of_files(&settings.valid_input_files_dir)
                    );

                    if settings.check_for_stability && obj.get_stability_mode() != StabilityMode::None {
                        find_broken_files_by_different_output(&settings, &obj);
                    } else {
                        find_broken_files_by_text_status(&settings, &obj);
                    }
                }
                "cargo-fuzz" => {
                    let target = target.expect("--target is required for cargo-fuzz mode");
                    let corpus = corpus.unwrap_or_default();
                    run_cargo_fuzz(&target, &corpus, effective_timeout, features.as_deref(), jobs);
                }
                other => {
                    eprintln!("Unknown fuzzing mode: {other}. Use 'custom' or 'cargo-fuzz'.");
                    std::process::exit(1);
                }
            }
        }

        Commands::Minimize { config, dir, command } => {
            set_timeout(999_999_999_999);
            minimize_cmd::run_minimize(&config, dir.as_deref(), command.as_deref());
        }

        Commands::Report { action } => match action {
            ReportAction::List { project, dir } => {
                report::list_reports(&dir, project.as_deref());
            }
            ReportAction::Create {
                dir,
                repo,
                version,
                variant,
            } => {
                report::create_report(&dir, repo.as_deref(), version.as_deref(), &variant);
            }
            ReportAction::CreateAll { project, dir } => {
                report::create_all_reports(&dir, project.as_deref());
            }
        },

        Commands::Ignore { action } => {
            let mut ignore = IgnoreList::load_or_default();
            match action {
                IgnoreAction::Add {
                    project,
                    pattern,
                    issue_url,
                } => {
                    ignore.add(IgnoreEntry {
                        project,
                        pattern,
                        issue_url,
                        added_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    });
                    ignore.save();
                    println!("Added to ignore list.");
                }
                IgnoreAction::Remove { project, pattern } => {
                    let removed = ignore.remove(&project, &pattern);
                    ignore.save();
                    if removed {
                        println!("Removed from ignore list.");
                    } else {
                        println!("Entry not found.");
                    }
                }
                IgnoreAction::List { project } => {
                    ignore.print_list(project.as_deref());
                    ignore_list::print_config_ignored_items(project.as_deref());
                }
                IgnoreAction::Verify => {
                    validate::validate_links(false);
                    validate::validate_config_ignored_items(false);
                }
                IgnoreAction::Clean => {
                    validate::validate_links(true);
                    validate::validate_config_ignored_items(true);
                }
            }
        }

        Commands::Ci { action } => match action {
            CiAction::Run {
                config,
                timeout,
                state_dir,
                output_dir,
                mode,
                target,
            } => {
                ci::run_ci(&config, timeout, &state_dir, &output_dir, &mode, target.as_deref());
            }
            CiAction::VerifyRegressions { config, state_dir } => {
                ci::verify_regressions(&config, &state_dir);
            }
        },

        Commands::Legacy {
            timeout,
            remove_non_crashing,
        } => {
            set_timeout(timeout);
            let settings = load_settings();
            let mut obj = get_object(settings.clone());
            obj.init();

            let _ = fs::create_dir_all(&settings.temp_folder);
            let _ = fs::create_dir_all(&settings.broken_files_dir);
            let _ = fs::create_dir_all(&settings.custom_folder_path);

            if remove_non_crashing || settings.remove_non_crashing_items_from_broken_files {
                info!("RUNNING REMOVE NON CRASHING FILES");
                remove_non_crashing_files::remove_non_crashing_files(&settings, &obj);
                return;
            }

            check_files_number("Valid input dir", &settings.valid_input_files_dir);
            check_files_number("Broken files dir", &settings.broken_files_dir);
            check_files_number("Temp possible broken files dir", &settings.temp_possible_broken_files_dir);

            assert!(Path::new(&settings.valid_input_files_dir).exists());
            assert!(Path::new(&settings.broken_files_dir).exists());

            info!(
                "Found {} files in valid input dir",
                calculate_number_of_files(&settings.valid_input_files_dir)
            );

            if settings.check_for_stability && obj.get_stability_mode() != StabilityMode::None {
                find_broken_files_by_different_output(&settings, &obj);
            } else {
                find_broken_files_by_text_status(&settings, &obj);
            }
        }
    }
}

fn load_settings_from(config_path: &str) -> settings::Setting {
    // Strip .toml extension if present — the config crate adds it automatically
    let path = config_path.strip_suffix(".toml").unwrap_or(config_path);
    load_settings_from_path(path)
}
