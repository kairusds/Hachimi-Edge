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

    if let Some(level) = filter_level.to_level() {
        windebug_logger::init_with_level(level).ok();
    }
}