use crate::il2cpp::types::*;

pub mod DownloadProgressUIGame;
pub mod BackgroundDownloadProgressUI;

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DownloadManager);

    DownloadProgressUIGame::init(DownloadManager);
    BackgroundDownloadProgressUI::init(DownloadManager);
}