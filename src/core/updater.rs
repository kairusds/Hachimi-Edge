use std::{sync::{Arc, Mutex}};

use rust_i18n::t;
use serde::Deserialize;

use crate::core::{gui::SimpleYesNoDialog, hachimi::{REPO_PATH, CODEBERG_API, GITHUB_API}, http, Error, Gui, Hachimi};

#[derive(Default)]
pub struct Updater {
    update_check_mutex: Mutex<()>,
    #[cfg(target_os = "windows")]
    new_update: arc_swap::ArcSwap<Option<ReleaseAsset>>
}

impl Updater {
    pub fn check_for_updates(self: Arc<Self>, callback: fn(bool)) {
        std::thread::spawn(move || {
            match self.check_for_updates_internal() {
                Ok(v) => callback(v),
                Err(e) => error!("{}", e)
            }
        });
    }

    fn check_for_updates_internal(&self) -> Result<bool, Error> {
        // Prevent multiple update checks running at the same time
        let Ok(_guard) = self.update_check_mutex.try_lock() else {
            return Ok(false);
        };

        if let Some(mutex) = Gui::instance() {
            mutex.lock().unwrap().show_notification(&t!("notification.checking_for_updates"));
        }

        let latest = match http::get_json::<Release>(&format!("{}/{}/releases/latest", GITHUB_API, REPO_PATH)) {
            Ok(res) => res,
            Err(e) => {
                warn!("GitHub update check failed, trying Codeberg: {}", e);
                http::get_json::<Release>(&format!("{}/{}/releases/latest", CODEBERG_API, REPO_PATH))?
            }
        };

        if latest.is_different_version() {
            #[cfg(target_os = "windows")]
            {
                let installer_asset = latest.assets.iter().find(|asset| asset.name == "hachimi_installer.exe");
                let hash_asset = latest.assets.iter().find(|asset| asset.name == "blake3.json");
    
                if let (Some(installer), Some(h_json)) = (installer_asset, hash_asset) {
                    let hash_data = http::get_json::<Blake3Hashes>(&h_json.browser_download_url)?;
                    let mut asset = installer.clone();
                    asset.expected_hash = Some(hash_data.installer_exe);
                    self.new_update.store(Arc::new(Some(asset)));
    
                    if let Some(mutex) = Gui::instance() {
                        mutex.lock().unwrap().show_window(Box::new(SimpleYesNoDialog::new(
                            &t!("update_prompt_dialog.title"),
                            &t!("update_prompt_dialog.content", version = latest.tag_name),
                            |ok| {
                                if !ok { return; }
                                Hachimi::instance().updater.clone().run();
                            }
                        )));
                    }
                    return Ok(true);
                }
            }
            #[cfg(target_os = "android")]
            {
                if let Some(mutex) = Gui::instance() {
                    mutex.lock().unwrap().show_window(Box::new(SimpleYesNoDialog::new(
                        &t!("update_prompt_dialog.title"),
                        &t!("update_prompt_dialog.content", version = latest.tag_name),
                        |ok| {
                            if !ok { return; }
                            Hachimi::instance().updater.clone().run();
                        }
                    )));
                }
            }
        } else if let Some(mutex) = Gui::instance() {
            mutex.lock().unwrap().show_notification(&t!("notification.no_updates"));
        }

        Ok(false)
    }

    pub fn run(self: Arc<Self>) {
        #[cfg(target_os = "windows")]
        {
            std::thread::spawn(move || {
                let dialog_show = Arc::new(std::sync::atomic::AtomicBool::new(true));
                if let Some(mutex) = Gui::instance() {
                    mutex.lock().unwrap().show_window(Box::new(crate::core::gui::PersistentMessageWindow::new(
                        &t!("updating_dialog.title"),
                        &t!("updating_dialog.content"),
                        dialog_show.clone()
                    )));
                }
    
                if let Err(e) = self.clone().run_internal() {
                    error!("{}", e);
                    if let Some(mutex) = Gui::instance() {
                        mutex.lock().unwrap().show_notification(&t!("notification.update_failed", reason = e.to_string()));
                    }
                }
    
                dialog_show.store(false, std::sync::atomic::Ordering::Relaxed)
            });
        }
        #[cfg(target_os = "android")]
        {
            use crate::{android::utils, core::hachimi::{UMAPATCHER_INSTALL_URL, UMAPATCHER_PACKAGE_NAME}};
            utils::open_app_or_fallback(
                UMAPATCHER_PACKAGE_NAME,
                &format!("{}.MainActivity", UMAPATCHER_PACKAGE_NAME.replace(".edge", "")),
                UMAPATCHER_INSTALL_URL
            );
        }
    }

    #[cfg(target_os = "windows")]
    fn run_internal(self: Arc<Self>) -> Result<(), Error> {
        let Some(ref asset) = **self.new_update.load() else {
            return Ok(());
        };
        self.new_update.store(Arc::new(None));

        use crate::windows::{main::DLL_HMODULE, utils};
        use windows::{
            core::{HSTRING, PCWSTR},
            Win32::{
                Foundation::{MAX_PATH, WPARAM, LPARAM}, System::LibraryLoader::GetModuleFileNameW,
                UI::{Shell::ShellExecuteW, WindowsAndMessaging::{PostMessageW, SW_NORMAL, WM_CLOSE}}
            }
        };
        use std::{fs::File, io::Read};

        // Download the installer
        let installer_path = utils::get_tmp_installer_path();

        let res = ureq::get(&asset.browser_download_url).call()?;
        let mut reader = res.into_body().into_reader();
        std::io::copy(&mut reader, &mut File::create(&installer_path)?)?;

        // Verify the installer
        if let Some(expected_hash) = &asset.expected_hash {
            let mut file = File::open(&installer_path)?;
            let mut hasher = blake3::Hasher::new();
            let mut buffer = [0u8; 8192];

            while let Ok(n) = file.read(&mut buffer) {
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }

            if hasher.finalize().to_hex().as_str() != expected_hash {
                let _ = std::fs::remove_file(&installer_path);
                return Err(Error::FileHashMismatch(installer_path.to_string_lossy().into()));
            }
        }

        // Launch the installer
        let mut slice = [0u16; MAX_PATH as usize];
        let length = unsafe { GetModuleFileNameW(Some(DLL_HMODULE), &mut slice) } as usize;
        let hachimi_path_str = unsafe { widestring::Utf16Str::from_slice_unchecked(&slice[..length]) };
        let game_dir = utils::get_game_dir();
        unsafe {
            ShellExecuteW(
                None,
                None,
                &HSTRING::from(installer_path.into_os_string()),
                &HSTRING::from(format!(
                    "install --install-dir \"{}\" --target \"{}\" --sleep 1000 --prompt-for-game-exit --launch-game -- {}",
                    game_dir.display(), hachimi_path_str, std::env::args().skip(1).collect::<Vec<String>>().join(" ")
                )),
                PCWSTR::from_raw(slice.as_ptr()),
                SW_NORMAL
            );

            // Close the game
            _ = PostMessageW(None, WM_CLOSE, WPARAM(0), LPARAM(0));
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct Release {
    // STUB
    tag_name: String,
    #[cfg(target_os = "windows")]
    assets: Vec<ReleaseAsset>
}

impl Release {
    pub fn is_different_version(&self) -> bool {
        self.tag_name != format!("v{}", env!("CARGO_PKG_VERSION"))
    }
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Clone)]
pub struct ReleaseAsset {
    // STUB
    name: String,
    browser_download_url: String,
    #[serde(skip)]
    pub expected_hash: Option<String>
}

#[cfg(target_os = "windows")]
#[derive(Deserialize)]
struct Blake3Hashes {
    #[serde(rename = "hachimi_installer.exe")]
    installer_exe: String
}