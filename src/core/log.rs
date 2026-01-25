use crate::log_impl;

pub fn init(debug_mode: bool, file_logging: bool) {
    let filter_level = if debug_mode {
        log::LevelFilter::Debug
    }
    else {
        log::LevelFilter::Info
    };

    log_impl::init(filter_level, file_logging);
}