use std::{fs, path::{Path, PathBuf}, process, sync::{atomic::{self, AtomicBool, AtomicI32}, Arc, Mutex}, time::{Duration, Instant}};
use arc_swap::ArcSwap;
use fnv::{FnvHashMap, FnvHashSet};
use once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use textwrap::wrap_algorithms::Penalties;

use crate::{core::{gui, plugin_api::Plugin, updater}, gui_impl, hachimi_impl, il2cpp::{self, hook::umamusume::{CySpringController::SpringUpdateMode, GameSystem}, sql::{CharacterData, SkillInfo}}};

use super::{game::{Game, Region}, ipc, plurals, template, template_filters, tl_repo, utils, Error, Interceptor};

pub const REPO_PATH: &str = "kairusds/Hachimi-Edge";
pub const GITHUB_API: &str = "https://api.github.com/repos";
pub const CODEBERG_API: &str = "https://codeberg.org/api/v1/repos";
pub const WEBSITE_URL: &str = "https://hachimi.noccu.art";
pub const UMAPATCHER_PACKAGE_NAME: &str = "com.leadrdrk.umapatcher.edge";
pub const UMAPATCHER_INSTALL_URL: &str = "https://github.com/kairusds/UmaPatcher-Edge/releases/latest";

static mut ORIG_SQLITE3_OPEN_V2: Option<extern "C" fn(*const i8, *mut *mut std::ffi::c_void, i32, *const i8) -> i32> = None;
static mut ORIG_SQLITE3_KEY: Option<extern "C" fn(*mut std::ffi::c_void, *const std::ffi::c_void, i32) -> i32> = None;

extern "C" fn sqlite3_open_v2_hook(filename: *const i8, pp_db: *mut *mut std::ffi::c_void, flags: i32, z_vfs: *const i8) -> i32 {
    let result = unsafe { ORIG_SQLITE3_OPEN_V2.unwrap()(filename, pp_db, flags, z_vfs) };

    if result == 0 && !pp_db.is_null() {
        if crate::il2cpp::sql::AUTO_UNLOCK_NEXT_DB.swap(false, std::sync::atomic::Ordering::Relaxed) {
            let raw_key = crate::il2cpp::sql::RETRIEVED_RAW_KEY.lock().unwrap();
            if !raw_key.is_empty() {
                let db_ptr = unsafe { *pp_db };
                unsafe { ORIG_SQLITE3_KEY.unwrap()(db_ptr, raw_key.as_ptr() as *const std::ffi::c_void, raw_key.len() as i32) };
            }
        }
    }
    result
}

extern "C" fn sqlite3_key_hook(db: *mut std::ffi::c_void, p_key: *const std::ffi::c_void, n_key: i32) -> i32 {
    if !p_key.is_null() {
        let mut raw_guard = crate::il2cpp::sql::RETRIEVED_RAW_KEY.lock().unwrap();
        if raw_guard.is_empty() {
            let key_bytes = unsafe { std::slice::from_raw_parts(p_key as *const u8, n_key as usize) };
            *raw_guard = key_bytes.to_vec();
        }
    }
    unsafe { ORIG_SQLITE3_KEY.unwrap()(db, p_key, n_key) }
}

pub struct Hachimi {
    // Hooking stuff
    pub interceptor: Interceptor,
    pub hooking_finished: AtomicBool,
    pub plugins: Mutex<Vec<Plugin>>,
    pub plugin_init_callbacks: Mutex<Vec<(usize, usize)>>,
    #[cfg(target_os = "windows")]
    pub present_callbacks: Mutex<Vec<(usize, usize)>>,

    // Translation repo manager
    pub tl_repo_manager: Mutex<tl_repo::RepoList>,

    // Localized data
    pub localized_data: ArcSwap<LocalizedData>,
    pub tl_updater: Arc<tl_repo::Updater>,
    pub tl_update_cmd: Mutex<Option<crossbeam_channel::Sender<()>>>,

    // Character data
    pub chara_data: ArcSwap<CharacterData>,
    // Untranslated skill info
    pub skill_info: ArcSwap<SkillInfo>,

    // Shared properties
    pub game: Game,
    pub config: ArcSwap<Config>,
    pub template_parser: template::Parser,

    /// -1 = default
    pub target_fps: AtomicI32,

    #[cfg(target_os = "windows")]
    pub vsync_count: AtomicI32,

    #[cfg(target_os = "windows")]
    pub window_always_on_top: AtomicBool,

    #[cfg(target_os = "windows")]
    pub discord_rpc: AtomicBool,

    pub updater: Arc<updater::Updater>
}

static INSTANCE: OnceCell<Arc<Hachimi>> = OnceCell::new();

impl Hachimi {
    pub fn init() -> bool {
        if INSTANCE.get().is_some() {
            warn!("Hachimi should be initialized only once");
            return true;
        }

        let instance = match Self::new() {
            Ok(v) => v,
            Err(e) => {
                super::log::init(false, false); // early init to log error
                error!("Init failed: {}", e);
                return false;
            }
        };

        let config = instance.config.load();
        if config.disable_gui_once {
            let mut config = config.as_ref().clone();
            config.disable_gui_once = false;
            _ = instance.save_config(&config);

            config.disable_gui = true;
            instance.config.store(Arc::new(config));
        }

        super::log::init(config.debug_mode, config.enable_file_logging);

        info!("Hachimi {}", env!("HACHIMI_DISPLAY_VERSION"));
        info!("Game region: {}", instance.game.region);

        if let Err(e) = instance.repair_tl_repo_state() {
            error!("TL repo repair failed: {}", e);
        }

        instance.load_localized_data();

        INSTANCE.set(Arc::new(instance)).is_ok()
    }

    pub fn instance() -> Arc<Hachimi> {
        INSTANCE.get().unwrap_or_else(|| {
            error!("FATAL: Attempted to get Hachimi instance before initialization");
            process::exit(1);
        }).clone()
    }

    pub fn is_initialized() -> bool {
        INSTANCE.get().is_some()
    }

    fn new() -> Result<Hachimi, Error> {
        let game = Game::init();
        let config = Self::load_config(&game.data_dir, &game.region)?;

        config.language.set_locale();

        Ok(Hachimi {
            interceptor: Interceptor::default(),
            hooking_finished: AtomicBool::new(false),
            plugins: Mutex::default(),
            plugin_init_callbacks: Mutex::default(),
            #[cfg(target_os = "windows")]
            present_callbacks: Mutex::default(),

            tl_repo_manager: Mutex::new(tl_repo::RepoList::default()),

            // Don't load localized data initially since it might fail, logging the error is not possible here
            localized_data: ArcSwap::default(),
            tl_updater: Arc::default(),
            tl_update_cmd: Mutex::new(None),

            // Same with these
            chara_data: ArcSwap::default(),
            skill_info: ArcSwap::default(),

            game,
            template_parser: template::Parser::new(&template_filters::LIST),

            target_fps: AtomicI32::new(config.target_fps.unwrap_or(-1)),

            #[cfg(target_os = "windows")]
            vsync_count: AtomicI32::new(config.windows.vsync_count),

            #[cfg(target_os = "windows")]
            window_always_on_top: AtomicBool::new(config.windows.window_always_on_top),

            #[cfg(target_os = "windows")]
            discord_rpc: AtomicBool::new(config.windows.discord_rpc),

            updater: Arc::default(),

            config: ArcSwap::new(Arc::new(config))
        })
    }

    // region param is unused?
    fn load_config(data_dir: &Path, _region: &Region) -> Result<Config, Error> {
        let config_path = data_dir.join("config.json");
        if fs::metadata(&config_path).is_ok() {
            let json = fs::read_to_string(&config_path)?;
            match serde_json::from_str::<Config>(&json) {
                Ok(config) => Ok(config),
                Err(e) => {
                    eprintln!("Failed to parse config: {}", e);
                    gui::request_notification(gui::NotificationRequest::ConfigLoadError);
                    Ok(Config::default())
                }
            }
        }else {
            Ok(Config::default())
        }
    }

    pub fn reload_config(&self) {
        let new_config = match Self::load_config(&self.game.data_dir, &self.game.region) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to reload config: {}", e);
                return;
            }
        };

        new_config.language.set_locale();
        self.config.store(Arc::new(new_config));

        if Hachimi::is_initialized() && self.hooking_finished.load(atomic::Ordering::Relaxed) {
            Hachimi::instance().start_translation_updater_thread();
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<(), Error> {
        fs::create_dir_all(&self.game.data_dir)?;
        let config_path = self.get_data_path("config.json");
        utils::write_json_file(config, &config_path)?;

        Ok(())
    }

    pub fn save_and_reload_config(&self, config: Config) -> Result<(), Error> {
        let old_id = self.config.load().selected_tl_repo_id;
        self.save_config(&config)?;

        config.language.set_locale();
        self.config.store(Arc::new(config));

        let new_config = self.config.load();
        if new_config.selected_tl_repo_id != old_id {
            self.load_localized_data();
            gui::request_notification(gui::NotificationRequest::TLRepoChanged);
        }

        if Hachimi::is_initialized() && self.hooking_finished.load(atomic::Ordering::Relaxed) {
            Hachimi::instance().start_translation_updater_thread();
        }

        Ok(())
    }

    pub fn get_active_tl_dir(&self) -> Option<PathBuf> {
        let id = self.config.load().selected_tl_repo_id?;
        Some(self.get_repo_dir(id))
    }

    pub fn load_localized_data(&self) {
        if self.tl_updater.progress().is_some() {
            warn!("Update in progress, not loading localized data");
            return;
        }

        let config = self.config.load();
        let ld_path = self.get_active_tl_dir().or_else(|| {
            config.localized_data_dir.as_ref().map(|p| self.game.data_dir.join(p))
        });

        let new_data = match LocalizedData::new(&self.config.load(), ld_path) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to load localized data: {}", e);
                return;
            }
        };
        self.localized_data.store(Arc::new(new_data));
    }

    pub fn init_character_data(&self) {
        if self.chara_data.load().chara_ids.is_empty() {
            let data = CharacterData::load_from_db();
            self.chara_data.store(Arc::new(data));
            info!("Character database loaded successfully.");
        }
    }

    pub fn init_skill_info(&self) {
        if self.skill_info.load().skill_names.is_empty() {
            let data = SkillInfo::load_from_db();
            self.skill_info.store(Arc::new(data));
            info!("Skill info loaded successfully.");
        }
    }

    pub fn on_dlopen(&self, filename: &str, handle: usize) -> bool {
        let filename_lower = filename.to_lowercase();

        #[cfg(target_os = "windows")]
        if filename_lower.contains("libnative.dll") {
            unsafe {
                use windows::Win32::System::LibraryLoader::GetProcAddress;
                use windows::core::PCSTR;

                let h_module = windows::Win32::Foundation::HMODULE(handle as _);
                let open_addr = GetProcAddress(h_module, PCSTR("sqlite3_open_v2\0".as_ptr()));
                let key_addr = GetProcAddress(h_module, PCSTR("sqlite3_key\0".as_ptr()));

                if let Some(addr) = open_addr {
                    if let Ok(orig) = self.interceptor.hook(addr as usize, sqlite3_open_v2_hook as *const () as usize) {
                        ORIG_SQLITE3_OPEN_V2 = Some(std::mem::transmute(orig));
                    }
                }
                if let Some(addr) = key_addr {
                    if let Ok(orig) = self.interceptor.hook(addr as usize, sqlite3_key_hook as *const () as usize) {
                        ORIG_SQLITE3_KEY = Some(std::mem::transmute(orig));
                    }
                }
            }
        }

        #[cfg(target_os = "android")]
        if filename_lower.contains("libnative.so") {
            unsafe {
                let handle_ptr = handle as *mut libc::c_void;

                let open_sym = b"sqlite3_open_v2\0".as_ptr() as *const libc::c_char;
                let key_sym = b"sqlite3_key\0".as_ptr() as *const libc::c_char;

                let open_addr = libc::dlsym(handle_ptr, open_sym);
                let key_addr = libc::dlsym(handle_ptr, key_sym);

                if !open_addr.is_null() {
                    if let Ok(orig) = self.interceptor.hook(open_addr as usize, sqlite3_open_v2_hook  as *const () as usize) {
                        ORIG_SQLITE3_OPEN_V2 = Some(std::mem::transmute(orig));
                        info!("Successfully hooked native sqlite3_open_v2 (Android)");
                    }
                }
                if !key_addr.is_null() {
                    if let Ok(orig) = self.interceptor.hook(key_addr as usize, sqlite3_key_hook as *const () as usize) {
                        ORIG_SQLITE3_KEY = Some(std::mem::transmute(orig));
                        info!("Successfully hooked native sqlite3_key (Android)");
                    }
                }
            }
        }

        if hachimi_impl::is_criware_lib(filename) {
            crate::core::criware::init(handle);
            if !self.hooking_finished.load(atomic::Ordering::Relaxed) {
                self.on_hooking_finished();
            }
            return true;
        }

        // Prevent double initialization
        if self.hooking_finished.load(atomic::Ordering::Relaxed) { return false; }

        if hachimi_impl::is_il2cpp_lib(filename) {
            info!("Got il2cpp handle");
            il2cpp::symbols::set_handle(handle);
            false
        }
        else {
            false
        }
    }

    pub fn on_hooking_finished(&self) {
        self.hooking_finished.store(true, atomic::Ordering::Relaxed);

        info!("GameAssembly finished loading");
        il2cpp::symbols::init();
        il2cpp::hook::init();

        // By the time it finished hooking the game will have already finished initializing
        GameSystem::on_game_initialized();

        let config = self.config.load();
        if !config.disable_gui {
            gui_impl::init();
        }

        if config.enable_ipc {
            ipc::start_http(config.ipc_listen_all);
        }

        hachimi_impl::on_hooking_finished(self);

        Hachimi::instance().start_translation_updater_thread();

        for plugin in self.plugins.lock().unwrap().iter() {
            info!("Initializing plugin: {}", plugin.name);
            let res = plugin.init();
            if !res.is_ok() {
                info!("Plugin init failed");
            }
        }
    }

    pub fn get_data_path<P: AsRef<Path>>(&self, rel_path: P) -> PathBuf {
        self.game.data_dir.join(rel_path)
    }

    pub fn get_repo_dir(&self, id: u32) -> PathBuf {
        if id == 1 {
            let legacy = self.game.data_dir.join("localized_data");
            if legacy.is_dir() {
                return legacy;
            }
        }
        self.game.data_dir.join(format!("localized_data_{id}"))
    }

    fn repair_tl_repo_state(&self) -> Result<(), Error> {
        let repos_path = self.get_data_path(".tl_repos");
        let old_data_dir = self.game.data_dir.join("localized_data");
        let mut manager = self.tl_repo_manager.lock().unwrap();

        if !repos_path.exists() && old_data_dir.is_dir() {
            info!("Found legacy 'localized_data' folder and no .tl_repos; migrating…");

            let config = self.config.load();
            if let Some(index) = &config.translation_repo_index {
                let id = manager.add(index.clone());
                manager.save(&repos_path)?;

                let mut new_config = (**config).clone();
                new_config.selected_tl_repo_id = Some(id);
                self.save_and_reload_config(new_config)?;
            } else {
                manager.save(&repos_path)?;
            }
        }

        *manager = if repos_path.exists() {
            tl_repo::RepoList::load(&repos_path).unwrap_or_else(|e| {
                warn!("Failed to load .tl_repos ({e}); starting fresh");
                tl_repo::RepoList::default()
            })
        } else {
            tl_repo::RepoList::default()
        };

        let config = self.config.load();
        let index = config.translation_repo_index.clone();
        let current_id = config.selected_tl_repo_id;

        let mut manager_dirty = false;

        match current_id {
            Some(id) => {
                if manager.find_by_id(id) != index.as_deref() {
                    warn!("TL repo ID {id} does not match index {index:?}; re-resolving");

                    let mut cleared = (**config).clone();
                    cleared.selected_tl_repo_id = None;
                    self.save_config(&cleared)?;
                    self.config.store(Arc::new(cleared));

                    if let Some(ref idx) = index {
                        let new_id = match manager.find_by_index(idx) {
                            Some(existing) => existing,
                            None => {
                                let nid = manager.add(idx.clone());
                                manager_dirty = true;
                                nid
                            }
                        };

                        let mut new_config = self.config.load().as_ref().clone();
                        new_config.selected_tl_repo_id = Some(new_id);
                        self.save_and_reload_config(new_config)?;
                    } else {
                        let data_dir = self.get_repo_dir(id);
                        if !data_dir.is_dir() {
                            warn!("TL repo data folder '{}' is missing, clearing localised data until next update...", data_dir.display());
                            self.localized_data.store(Arc::new(LocalizedData::default()));
                            gui::request_notification(gui::NotificationRequest::TLFolderMissing);
                        }
                    }
                }
            }

            None => {
                if let Some(ref idx) = index {
                    let id = match manager.find_by_index(idx) {
                        Some(existing) => existing,
                        None => {
                            let nid = manager.add(idx.clone());
                            manager_dirty = true;
                            nid
                        }
                    };
                    let mut new_config = (**config).clone();
                    new_config.selected_tl_repo_id = Some(id);
                    self.save_and_reload_config(new_config)?;
                }
            }
        }

        if manager_dirty {
            manager.save(&repos_path)?;
        }

        if let Some(id) = self.config.load().selected_tl_repo_id {
            let old_cache = self.get_data_path(".tl_repo_cache");
            if old_cache.exists() {
                let new_cache = self.get_data_path(format!(".tl_repo_cache_{}", id));
                info!("Migrating standalone legacy tl repo cache file to {}", new_cache.display());
                if let Err(e) = fs::rename(&old_cache, &new_cache) {
                    warn!("Failed to rename legacy tp repo cache file: {e}");
                }
            }
        }

        Ok(())
    }

    pub fn run_auto_update_check(&self) {
        if !self.config.load().disable_auto_update_check {
            // Check for hachimi updates first, then translations
            // Don't auto check for tl updates if it's not up to date
            self.updater.clone().check_for_updates(|new_update| {
                let hachimi = Hachimi::instance();
                if !new_update && !hachimi.config.load().translator_mode {
                    hachimi.tl_updater.clone().check_for_updates(false, false);
                }
            });
        }
    }

    pub fn start_translation_updater_thread(self: Arc<Self>) {
        let mut cmd_lock = self.tl_update_cmd.lock().unwrap();

        // drop the old sender to signal the existing thread to exit.
        // Its recv_timeout will return Disconnected within 1 second.
        *cmd_lock = None;

        let config = self.config.load();
        if config.tl_auto_updater_mode == TLAutoUpdaterMode::Disabled
            || config.tl_auto_updater_interval_sec == 0
            || config.translator_mode
        {
            return;
        }

        let (tx, rx) = crossbeam_channel::bounded::<()>(1);
        *cmd_lock = Some(tx);
        drop(cmd_lock);

        let interval = Duration::from_secs(config.tl_auto_updater_interval_sec);

        std::thread::Builder::new()
            .name("translation_updater_thread".into())
            .spawn(move || {
                let mut next_check = Instant::now() + interval;
                let mut last_interval = interval;

                loop {
                    let config = self.config.load();
                    if config.tl_auto_updater_mode == TLAutoUpdaterMode::Disabled
                        || config.tl_auto_updater_interval_sec == 0
                        || config.translator_mode
                    {
                        break;
                    }

                    let interval = Duration::from_secs(config.tl_auto_updater_interval_sec);

                    // realign timer if interval changed
                    if interval != last_interval {
                        next_check = Instant::now() + interval;
                        last_interval = interval;
                    }

                    // don't re-check while user hasn't acted on the current update
                    if self.tl_updater.has_pending_update() {
                        next_check = Instant::now() + interval;
                        continue;
                    }

                    if Instant::now() >= next_check {
                        let silent = config.tl_auto_updater_mode == TLAutoUpdaterMode::Silent;
                        info!("Running translation updater check (Silent: {})...", silent);
                        self.tl_updater.clone().check_for_updates(false, silent);
                        next_check = Instant::now() + interval;
                    }

                    // interruptible sleep. wakes at least once/sec,
                    // exits immediately when sender is dropped (restart/stop).
                    let remaining = next_check.saturating_duration_since(Instant::now());
                    let sleep = remaining.min(Duration::from_secs(1));

                    match rx.recv_timeout(sleep) {
                        Ok(()) | Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .expect("Failed to spawn translation updater thread");
    }
}

fn default_serde_instance<'a, T: Deserialize<'a>>() -> Option<T> {
    let empty_data = std::iter::empty::<((), ())>();
    let empty_deserializer = serde::de::value::MapDeserializer::<_, serde::de::value::Error>::new(empty_data);
    T::deserialize(empty_deserializer).ok()
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum TLAutoUpdaterMode {
    Disabled,
    Periodic,
    Silent
}

impl Default for TLAutoUpdaterMode {
    fn default() -> Self { Self::Disabled }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CaptionConfig {
    #[serde(default)]
    pub caption_enable: bool,
    #[serde(default = "CaptionConfig::default_lines_char_count")]
    pub caption_lines_char_count: i32,
    #[serde(default = "CaptionConfig::default_font_size")]
    pub caption_font_size: i32,
    #[serde(default = "CaptionConfig::default_color")]
    pub caption_color: String,
    #[serde(default = "CaptionConfig::default_outline_size")]
    pub caption_outline_size: String,
    #[serde(default = "CaptionConfig::default_outline_color")]
    pub caption_outline_color: String,
    #[serde(default = "CaptionConfig::default_bg_alpha")]
    pub caption_bg_alpha: f32,
    #[serde(default = "CaptionConfig::default_pos_x")]
    pub caption_pos_x: f32,
    #[serde(default = "CaptionConfig::default_pos_y")]
    pub caption_pos_y: f32,
}

impl Default for CaptionConfig {
    fn default() -> Self {
        Self {
            caption_enable: false,
            caption_lines_char_count: 26,
            caption_font_size: 50,
            caption_color: "White".to_owned(),
            caption_outline_size: "L".to_owned(),
            caption_outline_color: "Brown".to_owned(),
            caption_bg_alpha: 0.0,
            caption_pos_x: 0.0,
            caption_pos_y: -3.0,
        }
    }
}

impl CaptionConfig {
    fn default_lines_char_count() -> i32 { 26 }
    fn default_font_size() -> i32 { 50 }
    fn default_color() -> String { "White".to_owned() }
    fn default_outline_size() -> String { "L".to_owned() }
    fn default_outline_color() -> String { "Brown".to_owned() }
    fn default_bg_alpha() -> f32 { 0.0 }
    fn default_pos_x() -> f32 { 0.0 }
    fn default_pos_y() -> f32 { -3.0 }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub enable_file_logging: bool,
    #[serde(default)]
    pub apply_atlas_workaround: bool,
    #[serde(default)]
    pub translator_mode: bool,
    #[serde(default)]
    pub disable_gui: bool,
    #[serde(default)]
    pub disable_gui_once: bool,
    // legacy fallback path. populated by old versions, new code uses selected_tl_repo_id + get_active_tl_dir() exclusively
    // do NOT write this in new code
    pub localized_data_dir: Option<String>,
    pub target_fps: Option<i32>,
    #[serde(default = "Config::default_open_browser_url")]
    pub open_browser_url: String,
    #[serde(default = "Config::default_virtual_res_mult")]
    pub virtual_res_mult: f32,
    #[serde(default)]
    pub selected_tl_repo_id: Option<u32>,
    pub translation_repo_index: Option<String>,
    #[serde(default)]
    pub skip_first_time_setup: bool,
    #[serde(default)]
    pub lazy_translation_updates: bool,
    #[serde(default)]
    pub etag_translation_updates: bool,
    #[serde(default)]
    pub disable_auto_update_check: bool,

    #[serde(default)]
    pub tl_auto_updater_mode: TLAutoUpdaterMode,
    #[serde(default = "Config::default_tl_auto_updater_interval_sec")]
    pub tl_auto_updater_interval_sec: u64,

    #[serde(default)]
    pub disable_translations: bool,
    #[serde(default = "Config::default_gui_scale")]
    pub gui_scale: f32,
    #[serde(default = "Config::default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default = "Config::default_render_scale")]
    pub render_scale: f32,
    #[serde(default)]
    pub msaa: crate::il2cpp::hook::umamusume::GraphicSettings::MsaaQuality,
    #[serde(default)]
    pub aniso_level: crate::il2cpp::hook::UnityEngine_CoreModule::Texture::AnisoLevel,
    #[serde(default)]
    pub shadow_resolution: crate::il2cpp::hook::umamusume::CameraData::ShadowResolution,
    #[serde(default)]
    pub graphics_quality: crate::il2cpp::hook::umamusume::GraphicSettings::GraphicsQuality,
    #[serde(default = "Config::default_story_choice_auto_select_delay")]
    pub story_choice_auto_select_delay: f32,
    #[serde(default = "Config::default_story_tcps_multiplier")]
    pub story_tcps_multiplier: f32,
    #[serde(default)]
    pub enable_ipc: bool,
    #[serde(default)]
    pub ipc_listen_all: bool,
    #[serde(default)]
    pub force_allow_dynamic_camera: bool,
    #[serde(default)]
    pub live_theater_allow_same_chara: bool,
    #[serde(default = "Config::default_live_vocals_swap")]
    pub live_vocals_swap: [i32; 6],
    #[serde(default)]
    pub skill_info_dialog: bool,
    #[serde(default)]
    pub homescreen_bgseason: crate::il2cpp::hook::umamusume::TimeUtil::BgSeason,
    pub sugoi_url: Option<String>,
    #[serde(default)]
    pub auto_translate_stories: bool,
    #[serde(default)]
    pub auto_translate_localize: bool,
    #[serde(default)]
    pub disable_skill_name_translation: bool,
    #[serde(default)]
    pub hide_ingame_ui_hotkey: bool,
    #[serde(flatten)]
    pub caption: CaptionConfig,
    #[serde(default)]
    pub language: Language,
    #[serde(default = "Config::default_meta_index_url")]
    pub meta_index_url: String,
    #[serde(default)]
    pub ipv4_only: bool,
    pub physics_update_mode: Option<SpringUpdateMode>,
    #[serde(default)]
    pub cyspring_mono_uncap_frame_scale: bool,
    #[serde(default = "Config::default_ui_animation_scale")]
    pub ui_animation_scale: f32,
    #[serde(default)]
    pub live_slider_always_show: bool,
    #[serde(default)]
    pub live_playback_loop: bool,
    #[serde(default)]
    pub champions_live_show_text: bool,
    #[serde(default = "Config::default_champions_live_resource_id")]
    pub champions_live_resource_id: i32,
    #[serde(default = "Config::default_champions_live_year")]
    pub champions_live_year: i32,
    #[serde(default)]
    pub hide_now_loading: bool,
    #[serde(default)]
    pub replace_to_builtin_font: bool,
    #[serde(default)]
    pub disabled_hooks: FnvHashSet<String>,

    // theme settings
    #[serde(default = "Config::default_ui_accent")]
    pub ui_accent_color: egui::Color32,
    #[serde(default = "Config::default_window_fill")]
    pub ui_window_fill: egui::Color32,
    #[serde(default = "Config::default_panel_fill")]
    pub ui_panel_fill: egui::Color32,
    #[serde(default = "Config::default_extreme_bg")]
    pub ui_extreme_bg_color: egui::Color32,
    #[serde(default = "Config::default_text_color")]
    pub ui_text_color: egui::Color32,
    #[serde(default = "Config::default_window_rounding")]
    pub ui_window_rounding: f32,

    #[cfg(target_os = "windows")]
    #[serde(flatten)]
    pub windows: hachimi_impl::Config,

    #[cfg(target_os = "android")]
    #[serde(flatten)]
    pub android: hachimi_impl::Config
}

impl Config {
    fn default_open_browser_url() -> String { "https://www.google.com/".to_owned() }
    fn default_virtual_res_mult() -> f32 { 1.0 }
    fn default_ui_scale() -> f32 { 1.0 }
    fn default_render_scale() -> f32 { 1.0 }
    fn default_gui_scale() -> f32 { 1.0 }
    fn default_story_choice_auto_select_delay() -> f32 { 1.2 }
    fn default_story_tcps_multiplier() -> f32 { 3.0 }
    fn default_meta_index_url() -> String { "https://gitlab.com/umatl/hachimi-meta/-/raw/main/meta.json".to_owned() }
    fn default_ui_animation_scale() -> f32 { 1.0 }
    fn default_live_vocals_swap() -> [i32; 6] { [0; 6] }
    fn default_champions_live_resource_id() -> i32 { 15 }
    fn default_champions_live_year() -> i32 { 2025 }
    pub fn default_ui_accent() -> egui::Color32 { egui::Color32::from_rgb(100, 150, 240) }
    pub fn default_window_fill() -> egui::Color32 { egui::Color32::from_rgba_premultiplied(27, 27, 27, 220) }
    pub fn default_panel_fill() -> egui::Color32 { egui::Color32::from_rgba_premultiplied(27, 27, 27, 220) }
    pub fn default_extreme_bg() -> egui::Color32 { egui::Color32::from_rgb(15, 15, 15) }
    pub fn default_text_color() -> egui::Color32 { egui::Color32::from_gray(170) }
    pub fn default_window_rounding() -> f32 { 10.0 }
    fn default_tl_auto_updater_interval_sec() -> u64 { 3600 }
}

impl Default for Config {
    fn default() -> Self {
        default_serde_instance().expect("default instance")
    }
}

#[derive(Deserialize, Default, Clone)]
pub struct OsOption<T> {
    #[cfg(target_os = "android")]
    android: Option<T>,

    #[cfg(target_os = "windows")]
    windows: Option<T>
}

impl<T> OsOption<T> {
    pub fn as_ref(&self) -> Option<&T> {
        #[cfg(target_os = "android")]
        return self.android.as_ref();

        #[cfg(target_os = "windows")]
        return self.windows.as_ref();
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum Language {
    #[serde(rename = "en")]
    English,

    #[serde(rename = "zh-tw")]
    TChinese,

    #[serde(rename = "zh-cn")]
    SChinese,

    #[serde(rename = "vi")]
    Vietnamese,

    #[serde(rename = "id")]
    Indonesian,

    #[serde(rename = "es")]
    Spanish,

    #[serde(rename = "pt-br")]
    BPortuguese,

    #[serde(rename = "fil")]
    Filipino
}

impl Default for Language {
    fn default() -> Self {
        let locale = sys_locale::get_locale().as_deref().unwrap_or("en").to_lowercase();
        if locale.contains("zh-hk") || locale.contains("zh-tw") || locale.contains("zh-hant") {
            Self::TChinese
        } else if locale.contains("zh") {
            Self::SChinese
        } else if locale.starts_with("vi") {
            Self::Vietnamese
        } else if locale.starts_with("id") {
            Self::Indonesian
        } else if locale.starts_with("es") {
            Self::Spanish
        } else if locale.starts_with("pt-br") {
            Self::BPortuguese
        } else if locale.starts_with("fil") {
            Self::Filipino
        } else {
            Self::English
        }
    }
}

impl Language {
    pub const CHOICES: &[(Self, &'static str)] = &[
        Self::English.choice(),
        Self::TChinese.choice(),
        Self::SChinese.choice(),
        Self::Vietnamese.choice(),
        Self::Indonesian.choice(),
        Self::Spanish.choice(),
        Self::BPortuguese.choice(),
        Self::Filipino.choice()
    ];

    pub fn set_locale(&self) {
        rust_i18n::set_locale(self.locale_str());
    }

    pub const fn locale_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::TChinese => "zh-tw",
            Language::SChinese => "zh-cn",
            Language::Vietnamese => "vi",
            Language::Indonesian => "id",
            Language::Spanish => "es",
            Language::BPortuguese => "pt-br",
            Language::Filipino => "fil"
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::TChinese => "繁體中文",
            Language::SChinese => "简体中文",
            Language::Vietnamese => "Tiếng Việt",
            Language::Indonesian => "Bahasa Indonesia",
            Language::Spanish => "Español (ES)",
            Language::BPortuguese => "Português (Brasil)",
            Language::Filipino => "Filipino"
        }
    }

    pub const fn choice(self) -> (Self, &'static str) {
        (self, self.name())
    }
}

#[derive(Default)]
pub struct LocalizedData {
    pub config: LocalizedDataConfig,
    path: Option<PathBuf>,

    pub localize_dict: FnvHashMap<String, String>,
    pub hashed_dict: FnvHashMap<u64, String>,
    pub text_data_dict: FnvHashMap<i32, FnvHashMap<i32, String>>, // {"category": {"index": "text"}}
    pub character_system_text_dict: FnvHashMap<i32, FnvHashMap<i32, String>>, // {"character_id": {"voice_id": "text"}}
    pub race_jikkyo_comment_dict: FnvHashMap<i32, String>, // {"id": "text"}
    pub race_jikkyo_message_dict: FnvHashMap<i32, String>, // {"id": "text"}
    assets_path: Option<PathBuf>,

    pub plural_form: plurals::Resolver,
    pub ordinal_form: plurals::Resolver,

    pub wrapper_penalties: Penalties
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CustomRubyBlock {
    pub block_index: i32,
    pub rubies: Vec<CustomRubyDef>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CustomRubyDef {
    pub char_x: f32,
    pub char_y: f32,
    pub ruby_text: String,
}

impl LocalizedData {
    fn new(config: &Config, ld_path: Option<PathBuf>) -> Result<LocalizedData, Error> {
        if config.disable_translations {
            return Ok(LocalizedData::default());
        }

        let path = ld_path;
        let config: LocalizedDataConfig = if let Some(ref p) = path {
            // Create .nomedia
            #[cfg(target_os = "android")]
            { _ = fs::OpenOptions::new().create_new(true).write(true).open(p.join(".nomedia")); }

            let ld_config_path = p.join("config.json");
            if fs::metadata(&ld_config_path).is_ok() {
                let json = fs::read_to_string(&ld_config_path)?;
                serde_json::from_str(&json)?
            }
            else {
                warn!("Localized data config not found");
                LocalizedDataConfig::default()
            }
        }
        else {
            LocalizedDataConfig::default()
        };

        let plural_form = Self::parse_plural_form_or_default(&config.plural_form)?;
        let ordinal_form = Self::parse_plural_form_or_default(&config.ordinal_form)?;

        let wrapper_penalties = Self::parse_wrap_penalties_or_default(&config.wrapper_penalties);

        Ok(LocalizedData {
            localize_dict: Self::load_dict_static(&path, config.localize_dict.as_ref()).unwrap_or_default(),
            hashed_dict: Self::load_dict_static(&path, config.hashed_dict.as_ref()).unwrap_or_default(),
            text_data_dict: Self::load_dict_static(&path, config.text_data_dict.as_ref()).unwrap_or_default(),
            character_system_text_dict: Self::load_dict_static(&path, config.character_system_text_dict.as_ref()).unwrap_or_default(),
            race_jikkyo_comment_dict: Self::load_dict_static(&path, config.race_jikkyo_comment_dict.as_ref()).unwrap_or_default(),
            race_jikkyo_message_dict: Self::load_dict_static(&path, config.race_jikkyo_message_dict.as_ref()).unwrap_or_default(),
            assets_path: path.as_ref()
                .map(|p| config.assets_dir.as_ref()
                    .map(|dir| p.join(dir))
                )
                .unwrap_or_default(),

            plural_form,
            ordinal_form,

            wrapper_penalties,

            config,
            path
        })
    }

    fn load_dict_static_ex<T: DeserializeOwned, P: AsRef<Path>>(ld_path_opt: &Option<PathBuf>, rel_path_opt: Option<P>, silent_fs_error: bool) -> Option<T> {
        let Some(ld_path) = ld_path_opt else {
            return None;
        };
        let Some(rel_path) = rel_path_opt else {
            return None;
        };

        let path = ld_path.join(rel_path);
        let json = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                if !silent_fs_error {
                    error!("Failed to read '{}': {}", path.display(), e);
                }
                return None;
            }
        };

        let dict = match serde_json::from_str::<T>(&json) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse '{}': {}", path.display(), e);
                return None;
            }
        };

        Some(dict)
    }

    fn load_dict_static<T: DeserializeOwned, P: AsRef<Path>>(ld_path_opt: &Option<PathBuf>, rel_path_opt: Option<P>) -> Option<T> {
        Self::load_dict_static_ex(ld_path_opt, rel_path_opt, false)
    }

    pub fn load_dict<T: DeserializeOwned, P: AsRef<Path>>(&self, rel_path_opt: Option<P>) -> Option<T> {
        Self::load_dict_static(&self.path, rel_path_opt)
    }

    pub fn load_assets_dict<T: DeserializeOwned, P: AsRef<Path>>(&self, rel_path_opt: Option<P>) -> Option<T> {
        Self::load_dict_static_ex(&self.assets_path, rel_path_opt, true)
    }

    fn parse_plural_form_or_default(opt: &Option<String>) -> Result<plurals::Resolver, Error> {
        if let Some(plural_form) = opt {
            Ok(plurals::Resolver::Expr(plurals::Ast::parse(plural_form)?))
        }
        else {
            Ok(plurals::Resolver::Function(|_| 0))
        }
    }

    fn parse_wrap_penalties_or_default(opt: &Option<PenaltiesConfig>) -> Penalties {
        let Some(cfg) = opt else {
            return Penalties::new()
        };
        Penalties {
            nline_penalty: cfg.nline_penalty,
            overflow_penalty: cfg.overflow_penalty,
            short_last_line_fraction: cfg.short_last_line_fraction,
            short_last_line_penalty: cfg.short_last_line_penalty,
            hyphen_penalty: cfg.hyphen_penalty
        }
    }

    pub fn get_assets_path<P: AsRef<Path>>(&self, rel_path: P) -> Option<PathBuf> {
        self.assets_path.as_ref().map(|p| p.join(rel_path))
    }

    pub fn get_data_path<P: AsRef<Path>>(&self, rel_path: P) -> Option<PathBuf> {
        self.path.as_ref().map(|p| p.join(rel_path))
    }

    pub fn load_asset_metadata<P: AsRef<Path>>(&self, rel_path: P) -> AssetMetadata {
        let mut path = rel_path.as_ref().to_owned();
        path.set_extension("json");
        self.load_assets_dict(Some(path)).unwrap_or_else(|| AssetInfo::<()>::default()).metadata()
    }

    pub fn load_asset_info<P: AsRef<Path>, T: DeserializeOwned>(&self, rel_path: P) -> AssetInfo<T> {
        let mut path = rel_path.as_ref().to_owned();
        path.set_extension("json");
        self.load_assets_dict(Some(path)).unwrap_or_else(|| AssetInfo::default())
    }

    pub fn load_custom_story_ruby(&self, ast_ruby_name: &str) -> Option<Vec<CustomRubyBlock>> {
        let filename = ast_ruby_name.split('/').last().unwrap_or(ast_ruby_name);

        let filename_no_ext = filename.strip_suffix(".asset").unwrap_or(filename);

        let id_str = filename_no_ext.strip_prefix("ast_ruby_")?;

        if id_str.len() < 6 { return None; }

        let category_id = &id_str[0..2];
        let story_id = &id_str[2..6];

        let path = format!("story/data/{}/{}/{}.json", category_id, story_id, filename_no_ext);

        self.load_assets_dict(Some(path))
    }
}

#[derive(Deserialize, Clone)]
pub struct LocalizedDataConfig {
    pub localize_dict: Option<String>,
    pub hashed_dict: Option<String>,
    pub text_data_dict: Option<String>,
    pub character_system_text_dict: Option<String>,
    pub race_jikkyo_comment_dict: Option<String>,
    pub race_jikkyo_message_dict: Option<String>,
    pub assets_dir: Option<String>,
    #[serde(default)]
    pub extra_asset_bundle: OsOption<String>,
    pub replacement_font_name: Option<String>,

    pub plural_form: Option<String>,
    pub ordinal_form: Option<String>,
    #[serde(default)]
    pub ordinal_types: Vec<String>,
    #[serde(default)]
    pub months: Vec<String>,
    pub month_text_format: Option<String>,

    #[serde(default)]
    pub use_text_wrapper: bool,
    // Predefined line widths are counts of cjk characters.
    // 1 cjk char = 2 columns, so setting this value to 2 replicates the default behaviour.
    pub line_width_multiplier: Option<f32>,
    #[serde(default)]
    pub systext_cue_lines: FnvHashMap<String, i32>,
    pub wrapper_penalties: Option<PenaltiesConfig>,

    #[serde(default)]
    pub auto_adjust_story_clip_length: bool,
    pub story_line_count_offset: Option<i32>,
    pub text_frame_line_spacing_multiplier: Option<f32>,
    pub text_frame_font_size_multiplier: Option<f32>,
    pub choice_btn_line_spacing_multiplier: Option<f32>,
    #[serde(default)]
    pub skill_formatting: SkillFormatting,
    #[serde(default)]
    pub text_common_allow_overflow: bool,
    #[serde(default)]
    pub text_common_best_fit: bool,
    #[serde(default)]
    pub now_loading_comic_title_ellipsis: bool,

    #[serde(default)]
    pub remove_ruby: bool,
    pub character_note_top_gallery_button: Option<UITextConfig>,
    pub character_note_top_talk_gallery_button: Option<UITextConfig>,

    pub news_url: Option<String>,

    // RESERVED
    #[serde(default)]
    pub _debug: i32
}

#[derive(Deserialize, Clone)]
pub struct UITextConfig {
    pub text: Option<String>,
    pub font_size: Option<i32>,
    pub line_spacing: Option<f32>
}

impl Default for LocalizedDataConfig {
    fn default() -> Self {
        default_serde_instance().expect("default instance")
    }
}

#[derive(Deserialize)]
pub struct AssetInfo<T> {
    #[cfg(target_os = "android")]
    #[serde(default)]
    android: AssetMetadata,

    #[cfg(target_os = "windows")]
    #[serde(default)]
    windows: AssetMetadata,

    pub data: Option<T>
}

// Can't derive(Default), see rust-lang/rust#26925
impl<T> Default for AssetInfo<T> {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "android")]
            android: Default::default(),

            #[cfg(target_os = "windows")]
            windows: Default::default(),

            data: None
        }
    }
}

impl<T> AssetInfo<T> {
    pub fn metadata(self) -> AssetMetadata {
        #[cfg(target_os = "android")]
        return self.android;

        #[cfg(target_os = "windows")]
        return self.windows;
    }

    pub fn metadata_ref(&self) -> &AssetMetadata {
        #[cfg(target_os = "android")]
        return &self.android;

        #[cfg(target_os = "windows")]
        return &self.windows;
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct AssetMetadata {
    pub bundle_name: Option<String>
}

#[derive(Deserialize, Clone)]
pub struct PenaltiesConfig {
    nline_penalty: usize,
    overflow_penalty: usize,
    short_last_line_fraction: usize,
    short_last_line_penalty: usize,
    hyphen_penalty: usize
}

#[derive(Deserialize, Clone)]
pub struct SkillFormatting {
    #[serde(default = "SkillFormatting::default_length")]
    pub name_length: i32,
    #[serde(default = "SkillFormatting::default_length")]
    pub desc_length: i32,
    #[serde(default = "SkillFormatting::default_lines")]
    pub name_short_lines: i32,

    #[serde(default = "SkillFormatting::default_mult")]
    pub name_short_mult: f32,
    #[serde(default = "SkillFormatting::default_mult")]
    pub name_sp_mult: f32,
}
impl SkillFormatting {
    fn default_length() -> i32 { 18 }
    fn default_lines() -> i32 { 1 }
    fn default_mult() -> f32 { 1.0 }
}

impl Default for SkillFormatting {
    fn default() -> Self {
        SkillFormatting {
            name_length: 13,
            desc_length: 18,
            name_short_lines: 1,
            name_short_mult: 1.0,
            name_sp_mult: 1.0 }
    }
}
