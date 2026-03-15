pub fn init(filter_level: log::LevelFilter, _file_logging: bool) {
    let log_path = crate::ios::utils::get_log_path();

    // Ensure the parent directory exists
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("failed to open iOS log file");

    simplelog::WriteLogger::init(
        filter_level,
        simplelog::Config::default(),
        log_file,
    )
    .expect("failed to initialize iOS logger");

    info!("Hachimi iOS logger initialized, log: {}", log_path.display());
}

