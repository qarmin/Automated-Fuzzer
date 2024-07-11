use crate::broken_files::{create_broken_files, LANGS};
use crate::common::CheckGroupFileMode;
use crate::obj::ProgramConfig;
use crate::settings::Setting;
use std::process::Child;

pub struct SymphoniaStruct {
    pub settings: Setting,
}

impl ProgramConfig for SymphoniaStruct {
    fn is_broken(&self, content: &str) -> bool {
        let contains_rust_backtrace = content.contains("RUST_BACKTRACE");
        let contains_memory_allocation_failure = content.contains("memory allocation of"); // https://github.com/pdeljanov/Symphonia/issues/297
        let contains_mkv_problem = content.contains("symphonia_format_mkv::ebml::read_vint"); // https://github.com/pdeljanov/Symphonia/issues/298
        let contains_time_problem = content.contains("symphonia_core::units::TimeBase::new"); // https://github.com/pdeljanov/Symphonia/issues/299
        let contains_iso_mp4_problem = content.contains("symphonia_format_isomp4::atoms::AtomIterator<B>::read_atom"); // https://github.com/pdeljanov/Symphonia/issues/300
        
        contains_rust_backtrace && !contains_memory_allocation_failure && !contains_mkv_problem && !contains_time_problem && !contains_iso_mp4_problem
    }
    fn broken_file_creator(&self) -> Child {
        create_broken_files(self, LANGS::GENERAL)
    }
    fn get_settings(&self) -> &Setting {
        &self.settings
    }
    fn get_files_group_mode(&self) -> CheckGroupFileMode {
        CheckGroupFileMode::ByFolder
    }
}
