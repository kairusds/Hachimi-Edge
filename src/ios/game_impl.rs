use std::path::PathBuf;

pub fn get_package_name() -> String {
    // UM:PD bundle identifier on iOS
    "jp.co.cygames.umamusume".to_string()
}

pub fn get_data_dir(_package_name: &str) -> PathBuf {
    get_game_documents_dir()
}

/// Returns the app's Documents directory, which is sandbox-accessible and user-visible.
/// Path: <AppSandbox>/Documents/hachimi/
fn get_game_documents_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/var/mobile".to_string());
    PathBuf::from(home).join("Documents").join("hachimi")
}
