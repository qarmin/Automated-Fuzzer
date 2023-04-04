use std::process::Child;
use crate::Setting;

pub trait ProgramConfig: Sync {
    fn is_broken(&self, content: &str) -> bool;
    fn validate_output(&self, full_name: String, output: String) -> Option<String>;
    fn get_run_command(&self, full_name: &str) -> Child;
    fn broken_file_creator(&self)  -> Child;
    fn get_settings(&self) -> &Setting;
}