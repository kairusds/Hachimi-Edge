use android_logger::FilterBuilder;
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::File;

pub fn init(filter_level: log::LevelFilter, file_logging: bool) {
    if file_logging {
        let mut path = super::utils::get_game_dir();
        path.push("hachimi.log");

        if let Ok(file) = File::create(path) {
            let config = ConfigBuilder::new()
                .set_target_level(LevelFilter::Error)
                .add_filter_ignore_str("sqlparser")
                .set_time_format_rfc3339()
                .build();

            if WriteLogger::init(filter_level, config, file).is_ok() {
                return;
            }
        }
    }

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(filter_level)
            .with_filter(
                FilterBuilder::new()
                    .filter_level(filter_level)
                    .filter_module("sqlparser", log::LevelFilter::Off) // annoying
                    .build()
            )
            .with_tag("Hachimi")
    );
}