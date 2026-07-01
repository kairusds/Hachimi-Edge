use std::{
    borrow::Cow,
    collections::HashMap,
    ops::RangeInclusive,
    os::raw::c_void,
    panic::{self, AssertUnwindSafe},
    sync::{atomic::{self, AtomicBool}, Arc, Mutex},
    thread,
    time::Instant
};

use egui_scale::EguiScale;
use fnv::FnvHashSet;
use once_cell::sync::{Lazy, OnceCell};
use rust_i18n::t;
use chrono::{Utc, Datelike};

use crate::il2cpp::{
    ext::StringExt,
    hook::{
        umamusume::{CameraData::ShadowResolution, CySpringController::SpringUpdateMode, Director, GameSystem, GraphicSettings::{GraphicsQuality, MsaaQuality}, Localize, TimeUtil::BgSeason, SceneManager as UmaSceneManager},
        UnityEngine_CoreModule::{Application, Texture::AnisoLevel}
    },
    symbols::Thread
};

#[cfg(target_os = "android")]
use crate::il2cpp::{
    ext::Il2CppStringExt,
    hook::{umamusume::WebViewManager, UnityEngine_CoreModule::{TouchScreenKeyboard, TouchScreenKeyboardType}},
    symbols::GCHandle,
    types::{Il2CppObject, Il2CppString, RangeInt}
};

#[cfg(target_os = "windows")]
use crate::il2cpp::hook::UnityEngine_CoreModule::QualitySettings;

use super::{
    free_camera::{self, FreeCameraMode},
    game::Region,
    hachimi::{self, Language, REPO_PATH, WEBSITE_URL},
    http::{ureq_config, AsyncRequest},
    live_utils,
    tl_repo::{self, RepoInfo, LocalRepoInfo},
    utils::{self, get_localized_string, SendPtr},
    Hachimi
};

macro_rules! add_font {
    ($fonts:expr, $family_fonts:expr, $filename:literal) => {
        $fonts.font_data.insert(
            $filename.to_owned(),
            egui::FontData::from_static(include_bytes!(concat!("../../assets/fonts/", $filename))).into()
        );
        $family_fonts.push($filename.to_owned());
    };
}

static PENDING_THEME: Mutex<Option<hachimi::Config>> = Mutex::new(None);

pub fn enqueue_theme_preview(config: hachimi::Config) {
    if let Ok(mut lock) = PENDING_THEME.lock() {
        *lock = Some(config);
    }
}

#[derive(Debug)]
pub enum NotificationRequest {
    ConfigLoadError,
    TLRepoChanged,
    TLFolderMissing,
    Custom(String),
}

static NOTIFICATION_REQUESTS: Lazy<Mutex<Vec<NotificationRequest>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn request_notification(request: NotificationRequest) {
    if let Ok(mut queue) = NOTIFICATION_REQUESTS.lock() {
        queue.push(request);
    }
}

static PREV_MENU_WIDTH: Mutex<f32> = Mutex::new(200.0);
static REQUESTED_WIDTH: Mutex<Option<f32>> = Mutex::new(None);

pub fn get_menu_width() -> f32 {
    *PREV_MENU_WIDTH.lock().unwrap()
}

pub fn set_menu_width(width: f32) {
    if let Ok(mut lock) = REQUESTED_WIDTH.lock() {
        *lock = Some(width);
    }
}

#[cfg(target_os = "windows")]
static PENDING_TITLE: Mutex<Option<Option<String>>> = Mutex::new(None);

static REMOVING_TLREPO: atomic::AtomicBool = atomic::AtomicBool::new(false);
static REMOVED_TLREPO_ID: atomic::AtomicU32 = atomic::AtomicU32::new(u32::MAX);

type BoxedWindow = Box<dyn Window + Send + Sync>;
pub struct Gui {
    pub context: egui::Context,
    config: hachimi::Config,
    pub input: egui::RawInput,
    default_style: egui::Style,
    pub gui_scale: f32,

    pub finalized_scale: f32,
    pub start_time: Instant,
    pub prev_main_axis_size: i32,
    last_fps_update: Instant,
    tmp_frame_count: u32,
    fps_text: String,
    last_focused: Option<egui::Id>,
    #[cfg(target_os = "android")]
    ime_cooldown: Option<Instant>,

    show_menu: bool,

    splash_visible: bool,
    splash_tween: TweenInOutWithDelay,
    splash_sub_str: String,

    menu_visible: bool,
    menu_anim_time: Option<Instant>,
    menu_fps_value: i32,

    #[cfg(target_os = "windows")]
    menu_vsync_value: i32,

    pub update_progress_visible: bool,

    notifications: Vec<Notification>,
    next_notification_id: u32,
    windows: Vec<BoxedWindow>,
}

const PIXELS_PER_POINT_RATIO: f32 = 3.0/1080.0;

static INSTANCE: OnceCell<Mutex<Gui>> = OnceCell::new();
pub static IS_CONSUMING_INPUT: AtomicBool = AtomicBool::new(false);
pub static GUI_INPUT_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static WANTS_INPUT: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SCENE: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static DISABLED_GAME_UIS: Lazy<Mutex<FnvHashSet<SendPtr>>> =
    Lazy::new(|| Mutex::new(FnvHashSet::default()));
static PLUGIN_MENU_ITEMS: Lazy<Mutex<Vec<PluginMenuItem>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_MENU_SECTIONS: Lazy<Mutex<Vec<PluginMenuSection>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_MENU_ICONS: Lazy<Mutex<HashMap<String, PluginMenuIcon>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static PLUGIN_NOTIFICATIONS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_WINDOWS_TO_SHOW: Lazy<Mutex<Vec<PluginWindow>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_WINDOWS_TO_CLOSE: Lazy<Mutex<Vec<i32>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub type PluginMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type PluginMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type PluginWindowCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

#[derive(Clone)]
struct PluginMenuItem {
    label: String,
    callback: Option<PluginMenuCallback>,
    userdata: usize
}

#[derive(Clone)]
struct PluginMenuIcon {
    uri: String,
    bytes: Arc<[u8]>,
}

#[derive(Clone)]
struct PluginMenuSection {
    title: Option<String>,
    icon: Option<PluginMenuIcon>,
    callback: PluginMenuSectionCallback,
    userdata: usize
}

#[derive(Clone)]
struct PluginWindow {
    id: i32,
    title: String,
    contents_callback: Option<PluginWindowCallback>,
    bottom_callback: Option<PluginWindowCallback>,
    userdata: usize,
}

unsafe impl Send for PluginWindow {}
unsafe impl Sync for PluginWindow {}

pub fn register_plugin_menu_item(label: String, callback: Option<PluginMenuCallback>, userdata: *mut c_void) {
    PLUGIN_MENU_ITEMS.lock().unwrap().push(PluginMenuItem {
        label,
        callback,
        userdata: userdata as usize
    });
}

pub fn register_plugin_menu_section(callback: PluginMenuSectionCallback, userdata: *mut c_void) {
    PLUGIN_MENU_SECTIONS.lock().unwrap().push(PluginMenuSection {
        title: None,
        icon: None,
        callback,
        userdata: userdata as usize
    });
}

pub fn register_plugin_menu_section_with_icon(
    title: String,
    uri: String,
    bytes: Vec<u8>,
    callback: PluginMenuSectionCallback,
    userdata: *mut c_void
) -> bool {
    if title.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_SECTIONS.lock().unwrap().push(PluginMenuSection {
        title: Some(title),
        icon: Some(PluginMenuIcon { uri, bytes: bytes.into() }),
        callback,
        userdata: userdata as usize
    });
    true
}

pub fn register_plugin_menu_icon(label: String, uri: String, bytes: Vec<u8>) -> bool {
    if label.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_ICONS.lock().unwrap().insert(label, PluginMenuIcon {
        uri,
        bytes: bytes.into(),
    });
    true
}

pub fn enqueue_plugin_notification(message: String) {
    PLUGIN_NOTIFICATIONS.lock().unwrap().push(message);
}

fn get_plugin_menu_items() -> Vec<PluginMenuItem> {
    PLUGIN_MENU_ITEMS.lock().unwrap().clone()
}

fn get_plugin_menu_sections() -> Vec<PluginMenuSection> {
    PLUGIN_MENU_SECTIONS.lock().unwrap().clone()
}

fn get_plugin_menu_icon(label: &str) -> Option<PluginMenuIcon> {
    PLUGIN_MENU_ICONS.lock().unwrap().get(label).cloned()
}

fn drain_plugin_notifications() -> Vec<String> {
    let mut notifications = PLUGIN_NOTIFICATIONS.lock().unwrap();
    std::mem::take(&mut *notifications)
}

impl Window for PluginWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let id = egui::Id::new("plugin_window").with(self.id);

        new_window(ctx, id, &self.title)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                simple_window_layout(ui, id,
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if let Some(callback) = self.contents_callback {
                                let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                    callback(ui as *mut _ as *mut c_void, self.userdata as *mut c_void);
                                })).inspect_err(|_| error!("plugin window contents callback panicked"));
                            }
                        });
                    },
                    |ui| {
                        if let Some(callback) = self.bottom_callback {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                callback(ui as *mut _ as *mut c_void, self.userdata as *mut c_void);
                            })).inspect_err(|_| error!("plugin window bottom callback panicked"));
                        }
                    }
                );
            });

        open
    }

    fn plugin_window_id(&self) -> Option<i32> { Some(self.id) }
}

pub fn show_plugin_window(
    id: i32,
    title: String,
    contents_callback: Option<PluginWindowCallback>,
    bottom_callback: Option<PluginWindowCallback>,
    userdata: usize,
) {
    let window = PluginWindow {
        id,
        title,
        contents_callback,
        bottom_callback,
        userdata,
    };
    
    PLUGIN_WINDOWS_TO_SHOW.lock().unwrap().push(window);
}

pub fn close_plugin_window(id: i32) {
    PLUGIN_WINDOWS_TO_CLOSE.lock().unwrap().push(id);
}

fn drain_plugin_windows_to_show() -> Vec<PluginWindow> {
    let mut windows = PLUGIN_WINDOWS_TO_SHOW.lock().unwrap();
    std::mem::take(&mut *windows)
}

fn take_plugin_windows_to_close() -> Vec<i32> {
    let mut ids = PLUGIN_WINDOWS_TO_CLOSE.lock().unwrap();
    std::mem::take(&mut *ids)
}

#[cfg(target_os = "windows")]
pub type RawKeybind = u16;
#[cfg(target_os = "android")]
pub type RawKeybind = i32;

static KEYBIND_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static KEYBIND_CAPTURED: Lazy<Mutex<Option<(RawKeybind, String)>>> =
    Lazy::new(|| Mutex::new(None));

pub fn start_keybind_capture() {
    *KEYBIND_CAPTURED.lock().unwrap() = None;
    KEYBIND_CAPTURE_ACTIVE.store(true, atomic::Ordering::Relaxed);
}

pub fn is_keybind_capture_active() -> bool {
    KEYBIND_CAPTURE_ACTIVE.load(atomic::Ordering::Relaxed)
}

pub fn report_keybind_capture(raw: RawKeybind, display: String) {
    KEYBIND_CAPTURE_ACTIVE.store(false, atomic::Ordering::Relaxed);
    *KEYBIND_CAPTURED.lock().unwrap() = Some((raw, display));
}

fn take_keybind_capture() -> Option<(RawKeybind, String)> {
    KEYBIND_CAPTURED.lock().unwrap().take()
}

#[cfg(target_os = "android")]
static PENDING_KB_TYPE: atomic::AtomicI32 = atomic::AtomicI32::new(0);
#[cfg(target_os = "android")]
static PENDING_KEYBOARD_TEXT: atomic::AtomicPtr<Il2CppString> = atomic::AtomicPtr::new(std::ptr::null_mut());
#[cfg(target_os = "android")]
static ACTIVE_KEYBOARD: atomic::AtomicPtr<Il2CppObject> = atomic::AtomicPtr::new(std::ptr::null_mut());
#[cfg(target_os = "android")]
pub static KEYBOARD_GC_HANDLE: Lazy<Mutex<Option<GCHandle>>> = Lazy::new(|| Mutex::default());
#[cfg(target_os = "android")]
static KEYBOARD_SELECTION: Lazy<Mutex<RangeInt>> = Lazy::new(|| {
    Mutex::new(RangeInt::new(0, 1))
});
#[cfg(target_os = "android")]
pub static KEYBOARD_OWNER: Lazy<Mutex<Option<KeyboardOwner>>> =
    Lazy::new(|| Mutex::new(None));
#[cfg(target_os = "android")]
#[derive(PartialEq)]
pub enum KeyboardOwner {
    JNI(egui::Id),
    Unity(egui::Id)
}

fn get_scale_salt(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale_salt"))).unwrap_or(1.0)
}

pub fn get_scale(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale"))).unwrap_or(1.0)
}

#[cfg(target_os = "android")]
fn is_ime_visible() -> bool {
    let kb_ptr = ACTIVE_KEYBOARD.load(atomic::Ordering::Acquire);
    let unity_visible = if !kb_ptr.is_null() {
        TouchScreenKeyboard::get_status(kb_ptr) == TouchScreenKeyboard::Status::Visible
    } else {
        false
    };
    let jni_visible = crate::android::utils::IS_IME_VISIBLE.load(atomic::Ordering::Acquire);

    unity_visible || jni_visible
}

#[cfg(target_os = "android")]
fn ime_scroll_padding(ctx: &egui::Context) -> f32 {
    if !is_ime_visible() {
        return 0.0;
    }
    ctx.input(|i| i.viewport_rect().height() * 0.35)
}

#[cfg(target_os = "android")]
pub fn handle_android_keyboard<T: 'static>(res: &egui::Response, val: &mut T) {
    {
        let Ok(mut owner_lock) = KEYBOARD_OWNER.try_lock() else { return; };
        if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
            return;
        }

        if res.lost_focus() {
            if let Some(KeyboardOwner::Unity(id)) = *owner_lock {
                if id == res.id {
                    let kb_ptr = ACTIVE_KEYBOARD.load(atomic::Ordering::Acquire);
                    if !kb_ptr.is_null() {
                        TouchScreenKeyboard::set_active(kb_ptr, false);
                        ACTIVE_KEYBOARD.store(std::ptr::null_mut(), atomic::Ordering::Release);
                        *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
                    }
                    *owner_lock = None;
                }
            }
            return;
        }
    }

    if !res.has_focus() {
        return;
    }

    use utils::{char_to_utf16_index, utf16_to_char_index};
    use egui::{text::{CCursor, CCursorRange}, widgets::text_edit::TextEditState};

    let val_any = val as &dyn std::any::Any;
    PENDING_KB_TYPE.store(TouchScreenKeyboardType::KeyboardType::Default as i32, atomic::Ordering::Release);

    let text = if let Some(s) = val_any.downcast_ref::<String>() {
        s.clone()
    } else if let Some(f) = val_any.downcast_ref::<f32>() {
        PENDING_KB_TYPE.store(TouchScreenKeyboardType::KeyboardType::DecimalPad as i32, atomic::Ordering::Release);
        if f.fract() == 0.0 { format!("{:.1}", f) } else { f.to_string() }
    } else if let Some(i) = val_any.downcast_ref::<i32>() {
        PENDING_KB_TYPE.store(TouchScreenKeyboardType::KeyboardType::NumberPad as i32, atomic::Ordering::Release);
        i.to_string()
    } else {
        String::new()
    };

    if res.gained_focus() {
        {
            let mut owner_lock = KEYBOARD_OWNER.lock().unwrap();
            *owner_lock = Some(KeyboardOwner::Unity(res.id));
        }

        res.scroll_to_me(Some(egui::Align::Center));

        let ptr = text.to_il2cpp_string();
        PENDING_KEYBOARD_TEXT.store(ptr, atomic::Ordering::Release);

        let initial_selection = res.ctx.data(|data| {
            data.get_temp::<TextEditState>(res.id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| {
                let start_char = range.primary.index.min(range.secondary.index);
                let end_char = range.primary.index.max(range.secondary.index);

                let start_u16 = char_to_utf16_index(&text, start_char);
                let end_u16 = char_to_utf16_index(&text, end_char);

                RangeInt::new(start_u16, end_u16 - start_u16)
            })
            .unwrap_or(RangeInt::new(char_to_utf16_index(&text, text.chars().count()), 0))
        });
        *KEYBOARD_SELECTION.lock().unwrap() = initial_selection;

        Thread::main_thread().schedule(|| {
            let ptr = PENDING_KEYBOARD_TEXT.swap(std::ptr::null_mut(), atomic::Ordering::AcqRel);
            let typ: TouchScreenKeyboardType::KeyboardType = unsafe { *(&PENDING_KB_TYPE.load(atomic::Ordering::Acquire) as *const i32 as *const TouchScreenKeyboardType::KeyboardType) };

            if !ptr.is_null() {
                let keyboard = TouchScreenKeyboard::Open(ptr, typ, false, false, false);
                TouchScreenKeyboard::set_selection(keyboard, *KEYBOARD_SELECTION.lock().unwrap());
                let handle = GCHandle::new(keyboard, false);
                *KEYBOARD_GC_HANDLE.lock().unwrap() = Some(handle);
                ACTIVE_KEYBOARD.store(keyboard, atomic::Ordering::Release);
            }
        });
    }

    let kb_ptr = ACTIVE_KEYBOARD.load(atomic::Ordering::Acquire);
    if !kb_ptr.is_null() {
        let status = TouchScreenKeyboard::get_status(kb_ptr);

        if status == TouchScreenKeyboard::Status::Visible {
            let unity_range = TouchScreenKeyboard::get_selection(kb_ptr);

            let kb_txt_ptr = TouchScreenKeyboard::get_text(kb_ptr);
            if let Some(kb_ref) = unsafe { kb_txt_ptr.as_ref() } {
                let kb_txt_str = kb_ref.as_utf16str().to_string();

                let val_any_mut = val as &mut dyn std::any::Any;

                if let Some(s) = val_any_mut.downcast_mut::<String>() {
                    if *s != kb_txt_str { *s = kb_txt_str.clone(); }
                } else if let Some(f) = val_any_mut.downcast_mut::<f32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<f32>() {
                        let changed = !egui::emath::almost_equal(*f, parsed, 1e-6);
                        let drafting = kb_txt_str.ends_with('.') || (kb_txt_str.contains('.') && kb_txt_str.ends_with('0'));

                        if changed && !drafting {
                            *f = parsed;
                        }
                    }
                } else if let Some(i) = val_any_mut.downcast_mut::<i32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<i32>() {
                        if *i != parsed { *i = parsed; }
                    }
                }

                let kb_txt_clone = kb_txt_str.clone();
                res.ctx.data_mut(|data| {
                    if let Some(mut state) = data.get_temp::<TextEditState>(res.id) {
                        let start_char = utf16_to_char_index(&kb_txt_clone, unity_range.start as usize);
                        let end_char = utf16_to_char_index(&kb_txt_clone, (unity_range.start + unity_range.length) as usize);

                        let new_range = CCursorRange::two(CCursor::new(start_char), CCursor::new(end_char));

                        if state.cursor.char_range() != Some(new_range) {
                            state.cursor.set_char_range(Some(new_range));
                            data.insert_temp(res.id, state);
                        }
                    }
                });
            }
            res.ctx.request_repaint();
        }

        if status != TouchScreenKeyboard::Status::Visible {
            res.surrender_focus();
            res.ctx.memory_mut(|mem| mem.stop_text_input());
            res.ctx.data_mut(|data| {
                data.remove::<egui::widgets::text_edit::TextEditState>(res.id);
            });

            ACTIVE_KEYBOARD.store(std::ptr::null_mut(), atomic::Ordering::Release);
            *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
            res.ctx.request_repaint();
        }
    }
}

impl Gui {
    pub fn apply_theme(ctx: &egui::Context, style: &mut egui::Style, config: &hachimi::Config) {
        let mut visuals = egui::Visuals::dark(); // Base theme

        visuals.window_fill = config.ui_window_fill;
        visuals.panel_fill = config.ui_panel_fill;
        visuals.extreme_bg_color = config.ui_extreme_bg_color;
        visuals.window_corner_radius = config.ui_window_rounding.into();

        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, config.ui_text_color);

        visuals.widgets.active.bg_fill = config.ui_accent_color;
        visuals.widgets.hovered.bg_fill = config.ui_accent_color.linear_multiply(0.8);
        visuals.selection.bg_fill = config.ui_accent_color.linear_multiply(0.5);

        visuals.override_text_color = Some(config.ui_text_color);

        style.visuals = visuals.clone();
        ctx.set_visuals(visuals);
    }

    // Call this from the render thread!
    pub fn instance_or_init(
        #[cfg_attr(target_os = "windows", allow(unused))] open_key_id: &str
    ) -> &Mutex<Gui> {
        if let Some(instance) = INSTANCE.get() {
            return instance;
        }

        let hachimi = Hachimi::instance();
        let config = (**Hachimi::instance().config.load()).clone();

        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context);

        context.set_fonts(Self::get_font_definitions());

        let mut style = egui::Style::default();
        style.spacing.button_padding = egui::Vec2::new(8.0, 5.0);
        style.interaction.selectable_labels = false;

        Self::apply_theme(&context, &mut style, &config);

        context.set_style(style.clone());

        let default_style = style.clone();

        let mut fps_value = hachimi.target_fps.load(atomic::Ordering::Relaxed);
        if fps_value == -1 {
            fps_value = 30;
        }

        let mut windows: Vec<BoxedWindow> = Vec::new();
        if !config.skip_first_time_setup {
            windows.push(Box::new(FirstTimeSetupWindow::new()));
        }

        let now = Instant::now();
        let instance = Gui {
            context,
            config,
            input: egui::RawInput::default(),
            gui_scale: 1.0,
            finalized_scale: 1.0,
            default_style,
            start_time: now,
            prev_main_axis_size: 1,
            last_fps_update: now,
            tmp_frame_count: 0,
            fps_text: "FPS: 0".to_string(),
            last_focused: None,
            #[cfg(target_os = "android")]
            ime_cooldown: None,

            show_menu: false,

            splash_visible: true,
            splash_tween: TweenInOutWithDelay::new(0.8, 3.0, Easing::OutQuad),
            splash_sub_str: {
                #[cfg(target_os = "windows")]
                {
                    let key_label = crate::windows::utils::vk_to_display_label(hachimi.config.load().windows.menu_open_key);
                    t!("splash_sub", open_key_str = key_label).into_owned()
                }
                #[cfg(target_os = "android")]
                {
                    let key_label = crate::android::gui_impl::keymap::keycode_display_label(hachimi.config.load().android.menu_open_key);
                    let open_key_m = format!("{} / {}", t!(open_key_id), key_label);
                    t!("splash_sub", open_key_str = &*open_key_m).into_owned()
                }
            },

            menu_visible: false,
            menu_anim_time: None,
            menu_fps_value: fps_value,

            #[cfg(target_os = "windows")]
            menu_vsync_value: hachimi.vsync_count.load(atomic::Ordering::Relaxed),

            update_progress_visible: false,

            notifications: Vec::new(),
            next_notification_id: 0,
            windows,
        };

        unsafe {
            INSTANCE.set(Mutex::new(instance)).unwrap_unchecked();

            // Doing auto update check here to ensure that the updater can access the gui
            hachimi.run_auto_update_check();

            INSTANCE.get().unwrap_unchecked()
        }
    }

    pub fn instance() -> Option<&'static Mutex<Gui>> {
        INSTANCE.get()
    }

    fn get_font_definitions() -> egui::FontDefinitions {
        let mut fonts = egui::FontDefinitions::default();
        let proportional_fonts = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();

        add_font!(fonts, proportional_fonts, "Inter_24pt-Regular.ttf");
        add_font!(fonts, proportional_fonts, "AlibabaPuHuiTi-3-45-Light.otf");
        add_font!(fonts, proportional_fonts, "FontAwesome.otf");

        fonts
    }

    pub fn set_screen_size(&mut self, width: i32, height: i32) {
        let is_landscape = width > height;
        let main_axis_size = if is_landscape { height } else { width.min(height) };

        let orientation_scale = {
            #[cfg(target_os = "windows")]
            {
                let config = Hachimi::instance().config.load();
                let orientation_ratio = if is_landscape { height as f32 / width as f32 } else { 1.0 };
                if is_landscape && config.windows.enable_gui_landscape_ratio { orientation_ratio * config.windows.gui_landscape_ratio } else { 1.0 }
            }

            #[cfg(target_os = "android")]
            { 1.0 }
        };

        let pixels_per_point = main_axis_size as f32 * PIXELS_PER_POINT_RATIO * orientation_scale;
        self.context.set_pixels_per_point(pixels_per_point);

        self.input.screen_rect = Some(egui::Rect {
            min: egui::Pos2::default(),
            max: egui::Pos2::new(
                width as f32 / self.context.pixels_per_point(),
                height as f32 / self.context.pixels_per_point()
            )
        });

        self.prev_main_axis_size = main_axis_size;
    }

    fn take_input(&mut self) -> egui::RawInput {
        self.input.time = Some(self.start_time.elapsed().as_secs_f64());
        self.input.take()
    }

    fn update_fps(&mut self) {
        let delta = self.last_fps_update.elapsed().as_secs_f64();
        if delta > 0.5 {
            let fps = (self.tmp_frame_count as f64 * (0.5 / delta) * 2.0).round();
            self.fps_text = t!("menu.fps_text", fps = fps).into_owned();
            self.tmp_frame_count = 1;
            self.last_fps_update = Instant::now();
        }
        else {
            self.tmp_frame_count += 1;
        }
    }

    fn process_notification_requests(&mut self) {
        let requests = if let Ok(mut queue) = NOTIFICATION_REQUESTS.lock() {
            std::mem::take(&mut *queue)
        } else {
            Vec::new()
        };

        for req in requests {
            match req {
                NotificationRequest::ConfigLoadError => {
                    self.show_notification(&t!("notification.config_error"));
                }
                NotificationRequest::TLRepoChanged => {
                    self.show_notification(&t!("notification.tl_repo_changed"));
                }
                NotificationRequest::TLFolderMissing => {
                    self.show_notification(&t!("notification.tl_repo_folder_missing"));
                }
                NotificationRequest::Custom(msg) => {
                    self.show_notification(&msg);
                }
            }
        }
    }

    fn process_plugin_windows(&mut self) {
        let new_windows = drain_plugin_windows_to_show();
        let new_ids: Vec<i32> = new_windows.iter().map(|w| w.id).collect();
        let close_ids = take_plugin_windows_to_close();

        if !new_ids.is_empty() || !close_ids.is_empty() {
            self.windows.retain_mut(|w| {
                if let Some(id) = w.plugin_window_id() {
                    !new_ids.contains(&id) && !close_ids.contains(&id)
                } else {
                    true
                }
            });
        }

        for window in new_windows {
            self.show_window(Box::new(window));
        }
    }

    fn run_live_slider(&mut self, ctx: &egui::Context) {
        if !IS_LIVE_SCENE.load(atomic::Ordering::Acquire) {
            return;
        }

        let config = Hachimi::instance().config.load();

        use crate::il2cpp::{ext::Il2CppStringExt, hook::UnityEngine_CoreModule::{SceneManager, Scene}};

        let sm = UmaSceneManager::instance();
        let in_photo_mode = if !sm.is_null() {
            let photo_check = UmaSceneManager::get_PhotoCheckObject(sm);
            let photo_library = UmaSceneManager::get_PhotoLibraryObject(sm);
            !photo_check.is_null() || !photo_library.is_null()
        } else {
            false
        };
        if in_photo_mode {
            return;
        }

        let scene = SceneManager::GetActiveScene();
        let name_ptr = Scene::GetNameInternal(scene.handle);
        let scene_name = if name_ptr.is_null() { String::new() } else { unsafe { (*name_ptr).as_utf16str().to_string() } };

        if scene_name != "Live" {
            if IS_LIVE_SLIDER_ACTIVE.load(atomic::Ordering::Acquire) {
                ctx.input(|_i| {});
            }
            IS_LIVE_SCENE.store(false, atomic::Ordering::Release);
            IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
            live_utils::reset_live_drag_state();
            return;
        }

        IS_LIVE_SCENE.store(true, atomic::Ordering::Release);

        let director = Director::instance();
        if director.is_null() { return; }

        let mut current = Director::get_LiveCurrentTime(director);
        let total = Director::get_LiveTotalTime(director);
        if total <= 0.0 { return; }

        if config.live_playback_loop && current >= total - 0.1 {
            live_utils::move_live_playback(0.0);
            current = 0.0;
        }

        let is_paused = Director::is_live_paused();
        if !config.live_slider_always_show && !is_paused {
            IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
            return;
        }

        IS_LIVE_SLIDER_ACTIVE.store(true, atomic::Ordering::Release);

        let scale = get_scale(ctx);
        egui::Area::new(egui::Id::new("live_slider_area"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0 * scale))
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_black_alpha(150))
                    .inner_margin(egui::Margin::symmetric((16.0 * scale) as i8, (8.0 * scale) as i8))
                    .corner_radius(10.0 * scale)
                    .show(ui, |ui| {
                        ui.set_width(ctx.content_rect().width() * 0.7);
                        ui.horizontal(|ui| {
                            let curr_m = (current / 60.0).floor() as i32;
                            let curr_s = (current % 60.0).floor() as i32;
                            let tot_m = (total / 60.0).floor() as i32;
                            let tot_s = (total % 60.0).floor() as i32;
                            ui.label(format!("{:02}:{:02} / {:02}:{:02}", curr_m, curr_s, tot_m, tot_s));

                            let available_w = ui.available_width();

                            ui.scope(|ui| {
                                ui.spacing_mut().slider_width = available_w - (16.0 * scale);

                                let res = ui.add(
                                    egui::Slider::new(&mut current, 0.0..=total)
                                        .show_value(false)
                                        .trailing_fill(true)
                                );

                                if res.drag_started() {
                                    live_utils::begin_live_drag();
                                }

                                if res.changed() {
                                    live_utils::move_live_playback(current);
                                }

                                if res.drag_stopped() {
                                    live_utils::end_live_drag();
                                }
                            });
                        });
                    });
            });
    }

    pub fn run(&mut self) -> egui::FullOutput {
        if let Ok(mut lock) = PENDING_THEME.lock() {
            if let Some(config) = lock.take() {
                self.config = config.clone();
                Self::apply_theme(&self.context, &mut self.default_style, &config);

                let mut style = self.default_style.clone();
                style.scale(self.gui_scale);
                self.context.set_style(style)
            }
        }

        self.update_fps();
        let input = self.take_input();

        let live_scale = Hachimi::instance().config.load().gui_scale;
        if self.gui_scale != live_scale {
            self.gui_scale = live_scale;
            if !self.context.is_using_pointer() {
                self.finalized_scale = live_scale;
            }

            let mut style = self.default_style.clone();
            if live_scale != 1.0 {
                style.scale(live_scale);
            }
            self.context.set_style(style);
        }

        self.context.data_mut(|d| {
            d.insert_temp(egui::Id::new("gui_scale"), live_scale);
            d.insert_temp(egui::Id::new("gui_scale_salt"), self.finalized_scale);
        });

        let mut style = self.default_style.clone();
        if live_scale != 1.0 {
            style.scale(live_scale);
        }
        self.context.set_style(style);

        self.context.begin_pass(input);

        if self.menu_visible { self.run_menu(); }
        if self.update_progress_visible { self.run_update_progress(); }

        self.process_plugin_windows();
        self.run_windows();
        self.run_notifications();
        self.run_free_camera_overlay();

        if self.splash_visible { self.run_splash(); }
        self.process_notification_requests();

        #[cfg(target_os = "windows")]
        {
            use crate::il2cpp::hook::UnityEngine_InputLegacyModule::Input::set_imeCompositionMode;

            let focused = self.context.memory(|m| m.focused());
            let wants_kb = self.context.wants_keyboard_input();

            if focused != self.last_focused {
                if wants_kb {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(1);
                    });
                } else if self.last_focused.is_some() {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(0);
                    });
                }
            }
            self.last_focused = focused;
        }
        #[cfg(target_os = "android")]
        {
            use crate::android::utils::{set_keyboard_visible, check_keyboard_status, BACK_BUTTON_PRESSED, IS_IME_VISIBLE};

            let focused = self.context.memory(|m| m.focused());
            let wants_kb = self.context.wants_keyboard_input();

            if let Ok(mut owner_lock) = KEYBOARD_OWNER.try_lock() {
                if focused.is_some() && focused != self.last_focused && wants_kb {
                    if owner_lock.is_none() {
                        if !IS_IME_VISIBLE.load(atomic::Ordering::Acquire) {
                            set_keyboard_visible(true);
                            if let Some(id) = focused {
                                *owner_lock = Some(KeyboardOwner::JNI(id));
                            }
                            self.ime_cooldown = Some(Instant::now() + std::time::Duration::from_millis(500));
                        }
                    }
                } else if focused.is_none() && self.last_focused.is_some() {
                    if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
                        set_keyboard_visible(false);
                        *owner_lock = None;
                    }
                }

                if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
                    if BACK_BUTTON_PRESSED.swap(false, atomic::Ordering::AcqRel) {
                        *owner_lock = None;
                        set_keyboard_visible(false);
                        self.context.memory_mut(|mem| mem.stop_text_input());
                        IS_IME_VISIBLE.store(false, atomic::Ordering::Release);
                        self.last_focused = None;
                        self.ime_cooldown = None;
                    }
                }
            }

            // zombie check
            if self.tmp_frame_count % 20 == 0 {
                let should_check = if let Some(until) = self.ime_cooldown {
                    Instant::now() > until
                } else {
                    true
                };

                if should_check && IS_IME_VISIBLE.load(atomic::Ordering::Acquire) {
                    if !check_keyboard_status() {
                        self.context.memory_mut(|mem| mem.stop_text_input());
                        IS_IME_VISIBLE.store(false, atomic::Ordering::Release);

                        if let Ok(mut lock) = KEYBOARD_OWNER.try_lock() {
                            if let Some(KeyboardOwner::JNI(_)) = *lock {
                                *lock = None;
                            }
                        }
                        self.last_focused = None;
                        self.ime_cooldown = None;
                    }
                }
            }

            self.last_focused = focused;
        }

        let ctx = self.context.clone();
        self.run_live_slider(&ctx);

        let has_interactive_widgets = IS_LIVE_SCENE.load(atomic::Ordering::Relaxed);
        let free_camera_input_capture = free_camera::wants_windows_input_capture();

        // Store these as atomic values so the input thread can check them without locking the gui
        GUI_INPUT_ACTIVE.store(self.is_consuming_input(), atomic::Ordering::Release);
        IS_CONSUMING_INPUT.store(
            self.is_consuming_input() || has_interactive_widgets || free_camera_input_capture,
            atomic::Ordering::Release
        );

        WANTS_INPUT.store(
            self.context.wants_pointer_input() || 
            self.context.is_pointer_over_area() || 
            self.context.wants_keyboard_input() ||
            free_camera_input_capture,
            atomic::Ordering::Release
        );

        self.context.end_pass()
    }

    const ICON_IMAGE: egui::ImageSource<'static> = egui::include_image!("../../assets/icon.png");
    fn icon<'a>(ctx: &egui::Context) -> egui::Image<'a> {
        let scale = get_scale(ctx);
        egui::Image::new(Self::ICON_IMAGE)
            .fit_to_exact_size(egui::Vec2::new(24.0 * scale, 24.0 * scale))
    }

    fn icon_2x<'a>(ctx: &egui::Context) -> egui::Image<'a> {
        let scale = get_scale(ctx);
        egui::Image::new(Self::ICON_IMAGE)
            .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale))
    }

    fn run_splash(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);

        let id = egui::Id::from("splash");
        let Some(tween_val) = self.splash_tween.run(ctx, id.with("tween")) else {
            self.splash_visible = false;
            return;
        };

        egui::Area::new(id)
        .fixed_pos(egui::Pos2 {
            x: (-250.0 * scale) * (1.0 - tween_val),
            y: 16.0 * scale
        })
        .show(ctx, |ui| {
            egui::Frame::NONE
            .fill(self.config.ui_panel_fill)
            .inner_margin(egui::Margin::same((10.0 * scale) as i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(Self::icon(ctx));
                    ui.heading("Hachimi");
                    ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                });
                ui.label(&self.splash_sub_str);
            });
        });
    }

    fn run_menu(&mut self) {
        let hachimi = Hachimi::instance();
        let localized_data = hachimi.localized_data.load();
        let localize_dict_count = localized_data.localize_dict.len().to_string();
        let hashed_dict_count = localized_data.hashed_dict.len().to_string();

        let mut show_notification: Option<Cow<'_, str>> = None;
        let mut show_window: Option<BoxedWindow> = None;
        {
            let ctx = &self.context;
            let scale = get_scale(ctx);
            let salt = self.finalized_scale;

            let mut min_w = 96.0 * scale;
            let mut max_w = f32::INFINITY;

            if let Ok(mut lock) = REQUESTED_WIDTH.lock() {
                if let Some(w) = lock.take() {
                    min_w = w;
                    max_w = w;
                }
            }

            let panel_res = egui::SidePanel::left(egui::Id::new("hachimi_menu").with(salt.to_bits()))
                .min_width(min_w)
                .max_width(max_w)
                .default_width(200.0 * scale)
                .show_animated(ctx, self.show_menu, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
                    #[cfg(target_os = "windows")]
                    {
                        ui.horizontal(|ui| {
                            ui.add(Self::icon(ctx));
                            ui.heading(t!("hachimi"));
                            if ui.button(" \u{f29c} ").clicked() {
                                show_window = Some(Box::new(AboutWindow::new()));
                            }
                        });
                        ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                        if ui.button(t!("menu.close_menu")).clicked() {
                            self.show_menu = false;
                            self.menu_anim_time = None;
                        }
                    }
                    // did this because android phones have a notch
                    #[cfg(target_os = "android")]
                    {
                        ui.horizontal(|ui| {
                            ui.add(Self::icon(ctx));
                            ui.heading(t!("hachimi"));
                        });
                        ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                        ui.horizontal(|ui| {
                            if ui.button(t!("menu.close_menu")).clicked() {
                                self.show_menu = false;
                                self.menu_anim_time = None;
                            }
                            if ui.button(" \u{f29c} ").clicked() {
                                show_window = Some(Box::new(AboutWindow::new()));
                            }
                        });
                    }
                    if ui.button(t!("menu.check_for_updates")).clicked() {
                        Hachimi::instance().updater.clone().check_for_updates(|_| {});
                    }
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(t!("menu.stats_heading"));
                        ui.label(&self.fps_text);
                        ui.label(t!("menu.localize_dict_entries", count = localize_dict_count));
                        ui.label(t!("menu.hashed_dict_entries", count = hashed_dict_count));
                        ui.separator();

                        ui.heading(t!("menu.config_heading"));
                        if ui.button(t!("menu.open_config_editor")).clicked() {
                            show_window = Some(Box::new(ConfigEditor::new()));
                        }
                        if ui.button(t!("menu.reload_config")).clicked() {
                            hachimi.reload_config();
                            show_notification = Some(t!("notification.config_reloaded"));
                        }
                        if ui.button(t!("menu.open_first_time_setup")).clicked() {
                            show_window = Some(Box::new(FirstTimeSetupWindow::new()));
                        }
                        ui.separator();

                        ui.heading(t!("menu.graphics_heading"));
                        ui.horizontal(|ui| {
                            ui.label(t!("menu.fps_label"));
                            let res = ui.add(egui::Slider::new(&mut self.menu_fps_value, 30..=1000));
                            if res.lost_focus() || res.drag_stopped() {
                                hachimi.target_fps.store(self.menu_fps_value, atomic::Ordering::Relaxed);
                                Thread::main_thread().schedule(|| {
                                    Application::set_targetFrameRate(30);
                                });
                            }
                        });
                        #[cfg(target_os = "windows")]
                        {
                            use crate::windows::{discord, utils::set_window_topmost, wnd_hook};

                            ui.horizontal(|ui| {
                                let prev_value = self.menu_vsync_value;

                                ui.label(t!("menu.vsync_label"));
                                Self::run_vsync_combo(ui, &mut self.menu_vsync_value);

                                if prev_value != self.menu_vsync_value {
                                    hachimi.vsync_count.store(self.menu_vsync_value, atomic::Ordering::Relaxed);
                                    Thread::main_thread().schedule(|| {
                                        QualitySettings::set_vSyncCount(1);
                                    });
                                }
                            });
                            ui.horizontal(|ui| {
                                let mut value = hachimi.window_always_on_top.load(atomic::Ordering::Relaxed);

                                ui.label(t!("menu.stay_on_top"));
                                if ui.checkbox(&mut value, "").changed() {
                                    hachimi.window_always_on_top.store(value, atomic::Ordering::Relaxed);
                                    Thread::main_thread().schedule(|| {
                                        let topmost = Hachimi::instance().window_always_on_top.load(atomic::Ordering::Relaxed);
                                        unsafe { _ = set_window_topmost(wnd_hook::get_target_hwnd(), topmost); }
                                    });
                                }
                            });
                            ui.horizontal(|ui| {
                                let mut value = hachimi.discord_rpc.load(atomic::Ordering::Relaxed);

                                ui.label(t!("menu.discord_rpc"));
                                if ui.checkbox(&mut value, "").changed() {
                                    hachimi.discord_rpc.store(value, atomic::Ordering::Relaxed);
                                    if let Err(e) = if value { discord::start_rpc() } else { discord::stop_rpc() } {
                                        error!("{}", e);
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(t!("config_editor.enable_smtc"));
                                });
                                let mut enable_smtc = hachimi.config.load().windows.enable_smtc;
                                if ui.checkbox(&mut enable_smtc, "").changed() {
                                    use crate::windows::{smtc, wnd_hook::get_target_hwnd};
                                    let mut config = hachimi.config.load().as_ref().clone();
                                    config.windows.enable_smtc = enable_smtc;
                                    save_and_reload_config(config);
                                    self.config.windows.enable_smtc = enable_smtc;
                                    if UmaSceneManager::is_home_init() {
                                        if enable_smtc {
                                            smtc::init(get_target_hwnd());
                                        } else {
                                            smtc::unregister();
                                        }
                                    }
                                }
                            });
                            ui.end_row();
                        }
                        ui.separator();

                        ui.heading(t!("menu.translation_heading"));
                        if ui.button(t!("menu.change_translation_repo")).clicked() {
                            show_window = Some(Box::new(ChangeTranslationRepoWindow::new()));
                        }
                        if ui.button(t!("menu.reload_localized_data")).clicked() {
                            hachimi.load_localized_data();
                            show_notification = Some(t!("notification.localized_data_reloaded"));
                        }
                        if ui.button(t!("menu.tl_check_for_updates")).clicked() {
                            hachimi.tl_updater.skip_update(None);
                            hachimi.tl_updater.clone().check_for_updates(false, false);
                        }
                        if ui.button(t!("menu.tl_check_for_updates_pedantic")).clicked() {
                            hachimi.tl_updater.skip_update(None);
                            hachimi.tl_updater.clone().check_for_updates(true, false);
                        }
                        if hachimi.config.load().translator_mode {
                            if ui.button(t!("menu.dump_localize_dict")).clicked() {
                                Thread::main_thread().schedule(|| {
                                    let data = Localize::dump_strings();
                                    let dict_path = Hachimi::instance().get_data_path("localize_dump.json");
                                    let mut gui = Gui::instance().unwrap().lock().unwrap();
                                    if let Err(e) = utils::write_json_file(&data, dict_path) {
                                        gui.show_notification(&e.to_string())
                                    }
                                    else {
                                        gui.show_notification(&t!("notification.saved_localize_dump"))
                                    }
                                })
                            }
                        }
                        if ui.button(t!("menu.edit_excludes")).clicked() {
                            show_window = Some(Box::new(ExcludesEditorWindow::new()));
                        }
                        ui.separator();

                        let plugin_items = get_plugin_menu_items();
                        if !plugin_items.is_empty() {
                            ui.heading("Plugins");
                            for item in plugin_items {
                                let icon = get_plugin_menu_icon(&item.label);
                                let clicked = if let Some(icon) = icon {
                                    let size = 18.0 * scale;
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::Image::new((icon.uri, icon.bytes))
                                                .fit_to_exact_size(egui::Vec2::splat(size))
                                        );
                                        ui.button(&item.label).clicked()
                                    })
                                    .inner
                                }
                                else {
                                    ui.button(&item.label).clicked()
                                };
                                if clicked {
                                    if let Some(callback) = item.callback {
                                        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                            callback(item.userdata as *mut c_void);
                                        }))
                                        .inspect_err(|_| {
                                            error!("plugin menu item callback panicked: {}", item.label);
                                        });
                                    }
                                }
                            }
                            ui.separator();
                        }

                        let plugin_sections = get_plugin_menu_sections();
                        if !plugin_sections.is_empty() {
                            for section in plugin_sections {
                                if let Some(title) = section.title.clone() {
                                    let icon = section.icon.clone();
                                    let size = 18.0 * scale;
                                    ui.horizontal(|ui| {
                                        if let Some(icon) = icon {
                                            ui.add(
                                                egui::Image::new((icon.uri, icon.bytes))
                                                    .fit_to_exact_size(egui::Vec2::splat(size))
                                            );
                                        }
                                        ui.heading(title);
                                    });
                                }
                                let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                    (section.callback)(ui as *mut _ as *mut c_void, section.userdata as *mut c_void);
                                }))
                                .inspect_err(|_| {
                                    error!("plugin menu section callback panicked");
                                });
                            }
                            ui.separator();
                        }

                        ui.heading(t!("menu.danger_zone_heading"));
                        ui.vertical(|ui| {
                            ui.label(t!("menu.danger_zone_warning"));
                        });
                        if ui.button(t!("menu.soft_restart")).clicked() {
                            show_window = Some(Box::new(SimpleYesNoDialog::new(&t!("confirm_dialog_title"), &t!("soft_restart_confirm_content"), |ok| {
                                if !ok { return; }
                                Thread::main_thread().schedule(|| {
                                    GameSystem::SoftwareReset(GameSystem::instance());
                                });
                            })));
                        }
                        #[cfg(not(target_os = "windows"))]
                        if ui.button(t!("menu.open_in_game_browser")).clicked() {
                            show_window = Some(Box::new(SimpleYesNoDialog::new(&t!("confirm_dialog_title"), &t!("in_game_browser_confirm_content"), |ok| {
                                if !ok { return; }
                                Thread::main_thread().schedule(|| {
                                    WebViewManager::quick_open(&t!("browser_dialog_title"), &Hachimi::instance().config.load().open_browser_url);
                                });
                            })));
                        }
                        if ui.button(t!("menu.toggle_game_ui")).clicked() {
                            Thread::main_thread().schedule(Self::toggle_game_ui);
                        }

                        #[cfg(target_os = "android")]
                        {
                            let padding = ime_scroll_padding(ui.ctx());
                            if padding > 0.0 {
                                ui.add_space(padding);
                            }
                        }
                    });
                });
            });

            if let Some(inner) = &panel_res {
                let current_width = inner.response.rect.width();
                if let Ok(mut prev_lock) = PREV_MENU_WIDTH.lock() {
                    *prev_lock = current_width;
                }
            }
        }

        for message in drain_plugin_notifications() {
            self.show_notification(&message);
        }

        if !self.show_menu {
            if let Some(time) = self.menu_anim_time {
                if time.elapsed().as_secs_f32() >= self.context.style().animation_time {
                    self.menu_visible = false;
                }
            }
            else {
                self.menu_anim_time = Some(Instant::now());
            }
        }

        if let Some(content) = show_notification {
            self.show_notification(content.as_ref());
        }

        if let Some(window) = show_window {
            self.show_window(window);
        }
    }

    pub fn toggle_game_ui() {
        use crate::il2cpp::hook::{
            UnityEngine_CoreModule::{Object, Behaviour, GameObject},
            UnityEngine_UIModule::Canvas,
            Plugins::AnimateToUnity::AnRoot
        };

        let canvas_array = Object::FindObjectsOfType(Canvas::type_object(), true);
        let an_root_array = Object::FindObjectsOfType(AnRoot::type_object(), true);
        let canvas_iter = unsafe { canvas_array.as_slice().iter() };
        let an_root_iter = unsafe { an_root_array.as_slice().iter() };

        let mut disabled_uis = DISABLED_GAME_UIS.lock().unwrap();

        if disabled_uis.is_empty() {
            for canvas in canvas_iter {
                if Behaviour::get_enabled(*canvas) {
                    Behaviour::set_enabled(*canvas, false);
                    disabled_uis.insert(SendPtr(*canvas));
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if GameObject::get_activeSelf(top_object) {
                    GameObject::SetActive(top_object, false);
                    disabled_uis.insert(SendPtr(top_object));
                }
            }
        }
        else {
            for canvas in canvas_iter {
                if disabled_uis.contains(&SendPtr(*canvas)) {
                    Behaviour::set_enabled(*canvas, true);
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if disabled_uis.contains(&SendPtr(top_object)) {
                    GameObject::SetActive(top_object, true);
                }
            }
            disabled_uis.clear();
        }
    }

    #[cfg(target_os = "windows")]
    fn run_vsync_combo(ui: &mut egui::Ui, value: &mut i32) {
        Self::run_combo(ui, "vsync_combo", value, &[
            (-1, &t!("default")),
            (0, &t!("off")),
            (1, &t!("on")),
            (2, "1/2"),
            (3, "1/3"),
            (4, "1/4")
        ]);
    }

    fn run_combo<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_child: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)]
    ) -> bool {
        let mut selected = "Unknown";
        for choice in choices.iter() {
            if *value == choice.0 {
                selected = choice.1;
            }
        }

        let mut changed = false;
        egui::ComboBox::new(ui.id().with(id_child), "")
        .wrap_mode(egui::TextWrapMode::Extend)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for choice in choices.iter() {
                changed |= ui.selectable_value(value, choice.0, choice.1).changed();
            }
        });

        changed
    }

    fn run_combo_menu<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
        search_term: &mut String,
    ) -> bool {
        let mut changed = false;
        let scale = get_scale(ui.ctx());
        let fixed_width = 145.0 * scale;
        let row_height = 24.0 * scale;
        let padding = ui.spacing().button_padding;

        let button_id = ui.make_persistent_id(id_salt);
        let popup_id = button_id.with("popup");

        let selected_text = choices.iter()
            .find(|(v, _)| v == value)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        let (rect, _) = ui.allocate_exact_size(egui::vec2(fixed_width, row_height), egui::Sense::hover());
        let button_res = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
            let visuals = if is_open {
                &ui.visuals().widgets.open
            } else {
                ui.style().interact(&button_res)
            };

            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::epaint::StrokeKind::Inside
            );

            let icon_size = 12.0 * scale;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - padding.x - icon_size / 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size)
            );
            Self::down_triangle_icon(ui.painter(), icon_rect, visuals);

            let galley = ui.painter().layout_no_wrap(
                selected_text.to_owned(),
                egui::TextStyle::Button.resolve(ui.style()),
                visuals.text_color()
            );

            let text_pos = egui::pos2(
                rect.left() + padding.x,
                rect.center().y - galley.size().y / 2.0
            );
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }

        egui::Popup::menu(&button_res)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(fixed_width);
            ui.set_max_width(fixed_width);

            ui.horizontal(|ui| {
                let _res = ui.add_sized(
                    [ui.available_width() - 30.0 * scale, row_height],
                    egui::TextEdit::singleline(search_term).hint_text(t!("search_filter"))
                );
                #[cfg(target_os = "android")]
                handle_android_keyboard(&_res, search_term);

                if ui.button("\u{f00d}").clicked() {
                    search_term.clear();
                }
            });

            ui.separator();

            egui::ScrollArea::vertical()
            .max_height(250.0 * scale)
            .hscroll(false)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    for (choice_val, label) in choices {
                        if !search_term.is_empty() && !label.to_lowercase().contains(&search_term.to_lowercase()) {
                            continue;
                        }

                        let is_selected = value == choice_val;
                        if ui.add(egui::Button::selectable(is_selected, *label)).clicked() {
                            *value = *choice_val;
                            changed = true;
                            egui::Popup::close_id(ui.ctx(), popup_id);
                            search_term.clear();
                        }
                    }
                });
            });
        });

        changed
    }

    // egui's code originally (https://github.com/emilk/egui/blob/main/crates/egui/src/containers/combo_box.rs)
    pub fn down_triangle_icon(painter: &egui::Painter, rect: egui::Rect, visuals: &egui::style::WidgetVisuals) {
        let rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(rect.width() * 0.7, rect.height() * 0.45)
        );

        painter.add(egui::Shape::convex_polygon(
            vec![
                rect.left_top(),
                rect.right_top(),
                rect.center_bottom()
            ],
            visuals.fg_stroke.color,
            visuals.fg_stroke
        ));
    }

    fn run_update_progress(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);

        let progress = Hachimi::instance().tl_updater.progress().unwrap_or_else(|| {
            // Assume that update is complete
            self.update_progress_visible = false;
            tl_repo::UpdateProgress::new(1, 1)
        });
        let ratio = progress.current as f32 / progress.total as f32;

        egui::Area::new("update_progress".into())
        .fixed_pos(egui::Pos2 {
            x: 4.0 * scale,
            y: 4.0 * scale
        })
        .show(ctx, |ui| {
            egui::Frame::NONE
            .fill(self.config.ui_panel_fill)
            .inner_margin(egui::Margin::same((4.0 * scale) as i8))
            .corner_radius(4.0 * scale)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(t!("tl_updater.title"));
                    ui.add_space(26.0 * scale);
                    ui.label(format!("{:.2}%", ratio * 100.0));
                });
                ui.add(
                    egui::ProgressBar::new(ratio)
                    .desired_height(4.0 * scale)
                    .desired_width(140.0 * scale)
                );
                ui.label(
                    egui::RichText::new(t!("tl_updater.warning"))
                    .font(egui::FontId::proportional(10.0 * scale))
                );
            });
        });
    }

    fn run_notifications(&mut self) {
        let mut offset: f32 = -16.0;
        self.notifications.retain_mut(|n| n.run(&self.context, &mut offset));
    }

    fn run_free_camera_overlay(&mut self) {
        let Some((content, alpha)) = free_camera::overlay_message() else {
            return;
        };

        let ctx = &self.context;
        let scale = get_scale(ctx);
        let fill = egui::Color32::from_black_alpha((170.0 * alpha) as u8);
        let text = self.config.ui_text_color.linear_multiply(alpha);

        egui::Area::new(egui::Id::new("free_camera_overlay"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0 * scale, 16.0 * scale))
        .show(ctx, |ui| {
            egui::Frame::NONE
            .fill(fill)
            .inner_margin(egui::Margin::symmetric((10.0 * scale) as i8, (6.0 * scale) as i8))
            .corner_radius(6.0 * scale)
            .show(ui, |ui| {
                ui.set_min_width(260.0 * scale);
                ui.visuals_mut().override_text_color = Some(text);
                ui.label(content);
            });
        });
    }

    fn run_windows(&mut self) {
        self.windows.retain_mut(|w| w.run(&self.context));
    }

    pub fn is_empty(&self) -> bool {
        !self.splash_visible && !self.menu_visible && !self.update_progress_visible &&
        self.notifications.is_empty() && self.windows.is_empty() &&
        !IS_LIVE_SCENE.load(atomic::Ordering::Acquire) &&
        !free_camera::has_overlay_message()
    }

    pub fn is_consuming_input(&self) -> bool {
        self.menu_visible || !self.windows.is_empty() || IS_LIVE_SLIDER_ACTIVE.load(atomic::Ordering::Acquire)
    }

    pub fn is_consuming_input_atomic() -> bool {
        IS_CONSUMING_INPUT.load(atomic::Ordering::Acquire)
    }

    pub fn is_gui_input_active_atomic() -> bool {
        GUI_INPUT_ACTIVE.load(atomic::Ordering::Relaxed)
    }

    pub fn set_consuming_input(&mut self, val: bool) {
        if !self.windows.is_empty() && !val {
            self.windows.clear();
        }

        self.menu_visible = val;
        GUI_INPUT_ACTIVE.store(val, atomic::Ordering::Release);
        IS_CONSUMING_INPUT.store(val, atomic::Ordering::Release);
    }

    pub fn wants_input_atomic() -> bool {
        WANTS_INPUT.load(atomic::Ordering::Acquire)
    }

    pub fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        // Menu is always visible on show, but not immediately invisible on hide
        if self.show_menu {
            self.menu_visible = true;
        }
        else {
            self.menu_anim_time = None;
        }
    }

    pub fn show_notification(&mut self, content: &str) {
        self.add_notification(content, false);
    }

    pub fn show_persistent_notification(&mut self, content: &str) -> u32 {
        self.add_notification(content, true)
    }

    fn add_notification(&mut self, content: &str, persistent: bool) -> u32 {
        let id = self.next_notification_id;
        self.notifications.push(Notification::new(id, content.to_owned(), persistent));        
        self.next_notification_id = self.next_notification_id.wrapping_add(1);
        id
    }

    pub fn close_notification(&mut self, id: u32) {
        self.notifications.retain(|n| n.id != id);
    }

    pub fn show_window(&mut self, window: BoxedWindow) {
        self.windows.push(window);
    }
}

struct TweenInOutWithDelay {
    tween_time: f32,
    delay_duration: f32,
    easing: Easing,

    started: bool,
    delay_start: Option<Instant>
}

enum Easing {
    //Linear,
    //InQuad,
    OutQuad
}

impl TweenInOutWithDelay {
    fn new(tween_time: f32, delay_duration: f32, easing: Easing) -> TweenInOutWithDelay {
        TweenInOutWithDelay {
            tween_time,
            delay_duration,
            easing,

            started: false,
            delay_start: None
        }
    }

    fn run(&mut self, ctx: &egui::Context, id: egui::Id) -> Option<f32> {
        let anim_dir = if let Some(start) = self.delay_start {
            // Hold animation at peak position until duration passes
            start.elapsed().as_secs_f32() < self.delay_duration
        }
        else {
            // On animation start, initialize to 0.0. Next calls will start tweening to 1.0
            let v = self.started;
            self.started = true;
            v
        };
        let tween_val = ctx.animate_bool_with_time(id, anim_dir, self.tween_time);

        // Switch on delay when animation hits peak (next call makes tween_val < 1.0)
        if tween_val == 1.0 && self.delay_start.is_none() {
            self.delay_start = Some(Instant::now());
        }
        // Check if everything's done
        else if tween_val == 0.0 && self.delay_start.is_some() {
            return None;
        }

        Some(
            match self.easing {
                //Easing::Linear => tween_val,
                //Easing::InQuad => tween_val * tween_val,
                Easing::OutQuad => 1.0 - (1.0 - tween_val) * (1.0 - tween_val)
            }
        )
    }
}

// quick n dirty random id generator
fn random_id() -> egui::Id {
    egui::Id::new(egui::epaint::ahash::RandomState::new().hash_one(0))
}

pub struct NotificationGuard(pub u32); 

impl Drop for NotificationGuard {
    fn drop(&mut self) {
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.close_notification(self.0);
            }
        }
    }
}

struct Notification {
    id: u32,
    content: String,
    config: hachimi::Config,
    tween: TweenInOutWithDelay,
    egui_id: egui::Id
}

impl Notification {
    fn new(id: u32, content: String, persistent: bool) -> Notification {
        Notification {
            id,
            content,
            config: (**Hachimi::instance().config.load()).clone(),
            tween: TweenInOutWithDelay::new(
                0.2, 
                if persistent { f32::MAX } else { 3.0 }, 
                Easing::OutQuad
            ),
            egui_id: random_id()
        }
    }

    const WIDTH: f32 = 150.0;

    fn run(&mut self, ctx: &egui::Context, offset: &mut f32) -> bool {
        let scale = get_scale(ctx);

        let Some(tween_val) = self.tween.run(ctx, self.egui_id.with("tween")) else {
            return false;
        };

        let frame_rect = egui::Area::new(self.egui_id)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            egui::Vec2::new(
                (Self::WIDTH * scale) * (1.0 - tween_val),
                *offset
            )
        )
        .show(ctx, |ui| {
            egui::Frame::NONE
            .fill(self.config.ui_panel_fill)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(Self::WIDTH * scale);
                ui.label(&self.content);
            }).response.rect
        }).inner;

        *offset -= (2.0 * scale) + frame_rect.height() * tween_val;
        true
    }
}

pub trait Window {
    fn run(&mut self, ctx: &egui::Context) -> bool;
    fn plugin_window_id(&self) -> Option<i32> { None }
}

// Shared window creation function
fn new_window<'a>(ctx: &egui::Context, id: egui::Id, title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    let scale = get_scale(ctx);
    let salt = get_scale_salt(ctx);

    egui::Window::new(title)
    .id(id.with(salt.to_bits()))
    .pivot(egui::Align2::CENTER_CENTER)
    .fixed_pos(ctx.viewport_rect().max / 2.0)
    .min_width(96.0 * scale)
    .max_width(320.0 * scale)
    .max_height(250.0 * scale)
    .collapsible(false)
    .resizable(false)
    .constrain(false)
}

fn simple_window_layout(ui: &mut egui::Ui, id: egui::Id, add_contents: impl FnOnce(&mut egui::Ui), add_buttons: impl FnOnce(&mut egui::Ui)) {
    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), add_contents);

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), add_buttons);
    });
}

fn centered_and_wrapped_text(ui: &mut egui::Ui, text: &str) {
    let rect = ui.available_rect_before_wrap();

    let text_style = egui::TextStyle::Body;
    let text_font = ui.style().text_styles.get(&text_style).cloned().unwrap_or_default();
    let text_color = ui.style().visuals.text_color();

    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        text_font,
        text_color,
        rect.width()
    );
    job.halign = egui::Align::Center;

    let galley = ui.painter().layout_job(job);

    let text_rect = galley.rect;
    let text_size = text_rect.size();

    let center_pos = rect.min + (rect.size() - text_size) / 2.0;

    let paint_pos = center_pos - text_rect.min.to_vec2();
    ui.painter().galley(paint_pos, galley, text_color);
}

fn paginated_window_layout(
    ui: &mut egui::Ui,
    id: egui::Id,
    i: &mut usize,
    page_count: usize,
    allow_next: bool,
    add_page_content: impl FnOnce(&mut egui::Ui, usize)
) -> bool {
    let mut open = true;

    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            add_page_content(ui, *i);
        });

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if *i < page_count - 1 {
                if allow_next && ui.button(t!("next")).clicked() {
                    *i += 1;
                }
            } else {
                if ui.button(t!("done")).clicked() {
                    open = false;
                }
            }
            if *i > 0 && ui.button(t!("previous")).clicked() {
                *i -= 1;
            }
        });
    });

    open
}

fn tl_repo_list_ui(
    ui: &mut egui::Ui,
    request: &Arc<AsyncRequest<Vec<RepoInfo>>>,
    on_retry: impl FnOnce(),
    current_tl_repo: &mut Option<String>,
    has_auto_selected: &mut bool,
    current_lang_str: &str,
    show_skip: bool,
    check_already_downloaded: bool,
) {
    let Some(result) = &**request.result.load() else {
        if !request.running() {
            request.clone().call();
        }
        ui.centered_and_justified(|ui| {
            ui.label(t!("loading_label"));
        });
        return;
    };

    let repo_list = match result {
        Ok(v) => v,
        Err(e) => {
            let rect = ui.available_rect_before_wrap();
            let text_style = egui::TextStyle::Body;
            let text_font = ui.style().text_styles.get(&text_style).cloned().unwrap_or_default();
            let text_color = ui.visuals().text_color();
            let mut text_job = egui::text::LayoutJob::simple(e.to_string(), text_font, text_color, rect.width());
            text_job.halign = egui::Align::Center;
            let text_galley = ui.painter().layout_job(text_job.clone());
            let text_height = text_galley.size().y;
            let btn_text = t!("retry");
            let btn_style = egui::TextStyle::Button;
            let btn_font = ui.style().text_styles.get(&btn_style).cloned().unwrap_or_default();
            let btn_job = egui::text::LayoutJob::simple(btn_text.to_string(), btn_font, text_color, f32::INFINITY);
            let btn_galley = ui.painter().layout_job(btn_job);
            let btn_padding = ui.style().spacing.button_padding;
            let btn_height = btn_galley.size().y + btn_padding.y * 2.0;
            let spacing = ui.spacing().item_spacing.y;
            let total_height = text_height + spacing + btn_height;
            let center_y = rect.center().y;
            let top_y = (center_y - total_height / 2.0).max(rect.top());
            let content_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), top_y),
                egui::vec2(rect.width(), total_height)
            );
            let builder = egui::UiBuilder::new().max_rect(content_rect);
            ui.scope_builder(builder, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(text_job);
                    if ui.button(btn_text).clicked() {
                        on_retry();
                    }
                });
            });
            return;
        }
    };

    let hachimi = Hachimi::instance();

    let mut filtered_repos: Vec<_> = repo_list.iter()
        .filter(|repo| repo.region == hachimi.game.region)
        .collect();

    if !*has_auto_selected && current_tl_repo.is_none() {
        if let Some(matched) = filtered_repos.iter().find(|r| r.is_recommended(current_lang_str)) {
            *current_tl_repo = Some(matched.index.clone());
        }
        *has_auto_selected = true;
    }

    filtered_repos.sort_by_key(|repo| !repo.is_recommended(current_lang_str));

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(8, 0))
            .show(ui, |ui| {
                if filtered_repos.is_empty() {
                    ui.label(t!("first_time_setup.no_compatible_repo"));
                    return;
                }

                if show_skip {
                    ui.radio_value(current_tl_repo, None, t!("first_time_setup.skip_translation"));
                }

                let mut last_section: Option<bool> = None;

                for repo in filtered_repos.iter() {
                    let is_matched = repo.is_recommended(current_lang_str);
                    let is_selected = current_tl_repo.as_ref() == Some(&repo.index);

                    if let Some(prev_matched) = last_section {
                        if prev_matched != is_matched {
                            ui.separator();
                        }
                    }

                    let repo_label = if check_already_downloaded {
                        let manager = hachimi.tl_repo_manager.lock().unwrap();
                        let already_downloaded = manager.find_by_index(&repo.index).is_some();
                        drop(manager);

                        if already_downloaded {
                            format!("{} {}",
                                if is_matched && is_selected {
                                    format!("★ {}", repo.name)
                                } else {
                                    repo.name.clone()
                                },
                                t!("add_translation_repo.already_downloaded")
                            )
                        } else if is_matched && is_selected {
                            format!("★ {}", repo.name)
                        } else {
                            repo.name.clone()
                        }
                    } else if is_matched && is_selected {
                        format!("★ {}", repo.name)
                    } else {
                        repo.name.clone()
                    };

                    ui.radio_value(current_tl_repo, Some(repo.index.clone()), &repo_label);

                    if let Some(short_desc) = &repo.short_desc {
                        ui.label(egui::RichText::new(short_desc).small());
                    }

                    last_section = Some(is_matched);
                }
            });

        #[cfg(target_os = "android")]
        {
            let padding = ime_scroll_padding(ui.ctx());
            if padding > 0.0 {
                ui.add_space(padding);
            }
        }
    });
}

pub struct SimpleYesNoDialog {
    title: String,
    content: String,
    callback: Option<Box<dyn FnOnce(bool) + Send + Sync>>,
    id: egui::Id
}

impl SimpleYesNoDialog {
    pub fn new(title: &str, content: &str, callback: impl FnOnce(bool) + Send + Sync + 'static) -> SimpleYesNoDialog {
        SimpleYesNoDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            callback: Some(Box::new(callback)),
            id: random_id()
        }
    }
}

impl Window for SimpleYesNoDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;
        let mut result = false;

        new_window(ctx, self.id, &self.title)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom(self.id.with("bottom_panel"))
            .show_inside(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button(t!("no")).clicked() {
                        open2 = false;
                    }
                    if ui.button(t!("yes")).clicked() {
                        result = true;
                        open2 = false;
                    }
                })
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                centered_and_wrapped_text(ui, &self.content);
            });
        });

        if open && open2 {
            true
        }
        else {
            if let Some(cb) = self.callback.take() {
                cb(result);
            }
            false
        }
    }
}

pub struct SimpleOkDialog {
    title: String,
    content: String,
    scrollable: bool,
    callback: Option<Box<dyn FnOnce() + Send + Sync>>,
    id: egui::Id
}

impl SimpleOkDialog {
    pub fn new(title: &str, content: &str, scrollable: bool, callback: impl FnOnce() + Send + Sync + 'static) -> SimpleOkDialog {
        SimpleOkDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            scrollable,
            callback: Some(Box::new(callback)),
            id: random_id()
        }
    }
}

impl Window for SimpleOkDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;

        new_window(ctx, self.id, &self.title)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom(self.id.with("bottom_panel"))
            .show_inside(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button(t!("ok")).clicked() {
                        open2 = false;
                    }
                })
            });
 
            if self.scrollable {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(&self.content);
                });
            } else {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        centered_and_wrapped_text(ui, &self.content);
                    });
            }
        });

        if open && open2 {
            true
        }
        else {
            if let Some(cb) = self.callback.take() {
                cb();
            }
            false
        }
    }
}

struct ConfigEditor {
    last_ptr_config: usize,
    config: hachimi::Config,
    id: egui::Id,
    current_tab: ConfigEditorTab,
    search_term: String,
    champions_resources: Vec<String>,
    champions_live_max_year: i32,
    font_color_options: Vec<String>,
    outline_size_options: Vec<String>,
    outline_color_options: Vec<String>,
}

fn get_enum_options(class_name: &std::ffi::CStr) -> Vec<String> {
    use crate::il2cpp::{api::*, symbols::get_assembly_image, symbols::get_class};
    let mut options = Vec::new();
    let Ok(image) = get_assembly_image(c"umamusume.dll") else { return options };
    let Ok(klass) = get_class(image, c"Gallop", class_name) else { return options };

    if !il2cpp_class_is_enum(klass) { return options; }

    let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
    loop {
        let field = il2cpp_class_get_fields(klass, &mut iter);
        if field.is_null() { break; }
        let attrs = il2cpp_field_get_flags(field);
        if (attrs & 0x0040) != 0 {
            let name_ptr = il2cpp_field_get_name(field);
            if !name_ptr.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                if let Ok(s) = name.to_str() {
                    options.push(s.to_string());
                }
            }
        }
    }
    options
}

#[derive(Eq, PartialEq, Clone, Copy)]
enum ConfigEditorTab {
    General,
    Graphics,
    Gameplay
}

impl ConfigEditorTab {
    fn display_list() -> [(ConfigEditorTab, Cow<'static, str>); 3] {
        [
            (ConfigEditorTab::General, t!("config_editor.general_tab")),
            (ConfigEditorTab::Graphics, t!("config_editor.graphics_tab")),
            (ConfigEditorTab::Gameplay, t!("config_editor.gameplay_tab"))
        ]
    }
}

fn should_show_option(search: &str, label: &str) -> bool {
    search.is_empty() || label.to_lowercase().contains(&search.to_lowercase())
}

impl ConfigEditor {
    pub fn new() -> ConfigEditor {
        let handle = Hachimi::instance().config.load();
        ConfigEditor {
            last_ptr_config: Arc::as_ptr(&handle) as usize,
            config: (**Hachimi::instance().config.load()).clone(),
            id: random_id(),
            current_tab: ConfigEditorTab::General,
            search_term: String::new(),
            champions_resources: crate::il2cpp::sql::get_champions_resources(),
            champions_live_max_year: crate::il2cpp::sql::get_champions_live_max_year(),
            font_color_options: get_enum_options(c"FontColorType"),
            outline_size_options: get_enum_options(c"OutlineSizeType"),
            outline_color_options: get_enum_options(c"OutlineColorType"),
        }
    }

    fn restore_defaults(&mut self) {
        let current_language = self.config.language;
        self.config = hachimi::Config::default();
        self.config.language = current_language;
    }

    fn option_slider<Num: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, value: &mut Option<Num>, range: RangeInclusive<Num>) {
        let mut checked = value.is_some();
        ui.label(label);
        ui.checkbox(&mut checked, t!("enable"));
        ui.end_row();

        if checked && value.is_none() {
            *value = Some(*range.start())
        }
        else if !checked && value.is_some() {
            *value = None;
        }

        if let Some(num) = value.as_mut() {
            ui.label("");
            ui.add(egui::Slider::new(num, range));
            ui.end_row();
        }
    }

    fn run_options_grid(&self, config: &mut hachimi::Config, ui: &mut egui::Ui, tab: ConfigEditorTab, search: &str) {
        let scale = get_scale(ui.ctx());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let show_all = !search.is_empty();

        // General tab
        if show_all || tab == ConfigEditorTab::General {
            if should_show_option(search, &t!("config_editor.language")) {
                ui.label(t!("config_editor.language"));
                let lang_changed = Gui::run_combo(ui, "language", &mut config.language, Language::CHOICES);
                if lang_changed {
                    config.language.set_locale();
                }
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.disable_overlay")) {
                ui.label(t!("config_editor.disable_overlay"));
                if ui.checkbox(&mut config.disable_gui, "").clicked() {
                    if config.disable_gui {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.disable_overlay_warning"),
                                false,
                                || {}
                            )));
                        });
                    }
                }
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.ipv4_only")) {
                ui.label(t!("config_editor.ipv4_only"));
                ui.checkbox(&mut config.ipv4_only, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.meta_index_url")) {
                ui.label(t!("config_editor.meta_index_url"));
                let res = ui.add(egui::TextEdit::singleline(&mut config.meta_index_url).lock_focus(true));
                #[cfg(target_os = "android")]
                handle_android_keyboard(&res, &mut config.meta_index_url);
                #[cfg(target_os = "windows")]
                if res.has_focus() {
                    ui.memory_mut(|mem| mem.set_focus_lock_filter(
                        res.id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                            ..Default::default()
                        }
                    ));
                }
                ui.end_row();
                if res.lost_focus() && config.meta_index_url.trim().is_empty() {
                    config.meta_index_url = hachimi::Config::default().meta_index_url;
                }
            }

            if should_show_option(search, &t!("config_editor.gui_scale")) {
                ui.label(t!("config_editor.gui_scale"));
                ui.add(egui::Slider::new(&mut config.gui_scale, 0.25..=2.0).step_by(0.05));
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                if should_show_option(search, &t!("config_editor.gui_landscape_ratio")) {
                    ui.label(t!("config_editor.gui_landscape_ratio"));
                    ui.checkbox(&mut config.windows.enable_gui_landscape_ratio, t!("enable"));
                    ui.end_row();

                    if config.windows.enable_gui_landscape_ratio {
                        ui.label("");
                        ui.add(egui::Slider::new(&mut config.windows.gui_landscape_ratio, 0.25..=1.0).step_by(0.05).fixed_decimals(2));
                        ui.end_row();
                    }
                }
            }

            if should_show_option(search, &t!("theme_editor.title")) {
                ui.label(t!("theme_editor.title"));
                ui.horizontal(|ui| {
                    if ui.button(t!("open")).clicked() {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(ThemeEditorWindow::new()));
                        });
                    }
                });
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                if should_show_option(search, &t!("config_editor.discord_rpc")) {
                    ui.label(t!("config_editor.discord_rpc"));
                    ui.checkbox(&mut config.windows.discord_rpc, "");
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.menu_open_key")) {
                ui.label(t!("config_editor.menu_open_key"));
                ui.horizontal(|ui| {
                    #[cfg(target_os = "windows")]
                    ui.label(crate::windows::utils::vk_to_display_label(config.windows.menu_open_key));
                    #[cfg(target_os = "android")]
                    ui.label(crate::android::gui_impl::keymap::keycode_display_label(config.android.menu_open_key));

                    if ui.button(t!("bind_key")).clicked() {
                        std::thread::spawn(|| {
                            let Some(gui_mutex) = Gui::instance() else { return };
                            let mut gui = gui_mutex.lock().unwrap();
                            gui.show_window(Box::new(SetKeybindWindow::new(|result| {
                                let Some(raw) = result else { return };

                                let hachimi = Hachimi::instance();
                                let mut new_config = hachimi.config.load().as_ref().clone();

                                #[cfg(target_os = "windows")]
                                { new_config.windows.menu_open_key = raw; }
                                #[cfg(target_os = "android")]
                                { new_config.android.menu_open_key = raw; }

                                save_and_reload_config(new_config);
                            })));
                        });
                    }
                });
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.debug_mode")) {
                ui.label(t!("config_editor.debug_mode"));
                ui.checkbox(&mut config.debug_mode, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.enable_file_logging")) {
                ui.label(t!("config_editor.enable_file_logging"));
                ui.checkbox(&mut config.enable_file_logging, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.apply_atlas_workaround")) {
                ui.label(t!("config_editor.apply_atlas_workaround"));
                ui.checkbox(&mut config.apply_atlas_workaround, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.translator_mode")) {
                ui.label(t!("config_editor.translator_mode"));
                ui.checkbox(&mut config.translator_mode, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.skip_first_time_setup")) {
                ui.label(t!("config_editor.skip_first_time_setup"));
                ui.checkbox(&mut config.skip_first_time_setup, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.lazy_translation_updates")) {
                ui.label(t!("config_editor.lazy_translation_updates"));
                ui.checkbox(&mut config.lazy_translation_updates, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.etag_translation_updates")) {
                ui.label(t!("config_editor.etag_translation_updates"));
                ui.checkbox(&mut config.etag_translation_updates, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.disable_auto_update_check")) {
                ui.label(t!("config_editor.disable_auto_update_check"));
                ui.checkbox(&mut config.disable_auto_update_check, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.tl_auto_updater_mode")) {
                ui.label(t!("config_editor.tl_auto_updater_mode"));
                Gui::run_combo(ui, "tl_auto_updater_mode", &mut config.tl_auto_updater_mode, &[
                    (hachimi::TLAutoUpdaterMode::Disabled, &t!("disabled")),
                    (hachimi::TLAutoUpdaterMode::Periodic, &t!("config_editor.tl_auto_updater_periodic")),
                    (hachimi::TLAutoUpdaterMode::Silent, &t!("config_editor.tl_auto_updater_silent"))
                ]);
                ui.end_row();
            }

            if config.tl_auto_updater_mode != hachimi::TLAutoUpdaterMode::Disabled {
                if should_show_option(search, &t!("config_editor.tl_auto_updater_interval")) {
                    ui.label(t!("config_editor.tl_auto_updater_interval"));
                    let mut minutes = (config.tl_auto_updater_interval_sec / 60) as i32;
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut minutes).speed(1.0).range(1..=10080));
                        ui.label(t!("minutes"));
                    });
                    config.tl_auto_updater_interval_sec = (minutes as u64) * 60;
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.disable_translations")) {
                ui.label(t!("config_editor.disable_translations"));
                ui.checkbox(&mut config.disable_translations, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.enable_ipc")) {
                ui.label(t!("config_editor.enable_ipc"));
                ui.checkbox(&mut config.enable_ipc, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.ipc_listen_all")) {
                ui.label(t!("config_editor.ipc_listen_all"));
                ui.checkbox(&mut config.ipc_listen_all, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.hide_now_loading")) {
                ui.label(t!("config_editor.hide_now_loading"));
                ui.checkbox(&mut config.hide_now_loading, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.replace_to_builtin_font")) {
                ui.label(t!("config_editor.replace_to_builtin_font"));
                ui.checkbox(&mut config.replace_to_builtin_font, "");
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                if should_show_option(search, &t!("config_editor.ui_loading_show_orientation_guide")) {
                    ui.label(t!("config_editor.ui_loading_show_orientation_guide"));
                    ui.checkbox(&mut config.windows.ui_loading_show_orientation_guide, "");
                    ui.end_row();
                }
                
                if should_show_option(search, &t!("config_editor.custom_title_name")) {
                    ui.label(t!("config_editor.custom_title_name"));
                    let mut title_val = config.windows.custom_title_name.clone().unwrap_or_default();
                    let _ = ui.add(egui::TextEdit::singleline(&mut title_val).hint_text(t!("default")));
                    config.windows.custom_title_name = if title_val.is_empty() { None } else { Some(title_val) };
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.auto_translate_stories")) {
                ui.label(t!("config_editor.auto_translate_stories"));
                if ui.checkbox(&mut config.auto_translate_stories, "").clicked() {
                    if config.auto_translate_stories {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.auto_tl_warning"),
                                false,
                                || {}
                            )));
                        });
                    }
                }
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.auto_translate_ui")) {
                ui.label(t!("config_editor.auto_translate_ui"));
                if ui.checkbox(&mut config.auto_translate_localize, "").clicked() {
                    if config.auto_translate_localize {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("warning"),
                                &t!("config_editor.auto_tl_warning"),
                                false,
                                || {}
                            )));
                        });
                    }
                }
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                if should_show_option(search, &t!("config_editor.taskbar_show_progress_on_download")) {
                    ui.label(t!("config_editor.taskbar_show_progress_on_download"));
                    ui.checkbox(&mut config.windows.taskbar_show_progress_on_download, "");
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.taskbar_show_progress_on_connecting")) {
                    ui.label(t!("config_editor.taskbar_show_progress_on_connecting"));
                    ui.checkbox(&mut config.windows.taskbar_show_progress_on_connecting, "");
                    ui.end_row();
                }
            }
        }
        // General tab end

        // Graphics tab
        if show_all || tab == ConfigEditorTab::Graphics {
            if should_show_option(search, &t!("config_editor.target_fps")) {
                Self::option_slider(ui, &t!("config_editor.target_fps"), &mut config.target_fps, 30..=1000);
            }

            if should_show_option(search, &t!("config_editor.virtual_resolution_multiplier")) {
                ui.label(t!("config_editor.virtual_resolution_multiplier"));
                ui.add(egui::Slider::new(&mut config.virtual_res_mult, 1.0..=4.0).step_by(0.1));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.ui_scale")) {
                ui.label(t!("config_editor.ui_scale"));
                ui.add(egui::Slider::new(&mut config.ui_scale, 0.1..=10.0).step_by(0.05));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.ui_animation_scale")) {
                ui.label(t!("config_editor.ui_animation_scale"));
                ui.add(egui::Slider::new(&mut config.ui_animation_scale, 0.1..=10.0).step_by(0.1));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.render_scale")) {
                ui.label(t!("config_editor.render_scale"));
                ui.add(egui::Slider::new(&mut config.render_scale, 0.1..=10.0).step_by(0.1));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.msaa")) {
                ui.label(t!("config_editor.msaa"));
                Gui::run_combo(ui, "msaa", &mut config.msaa, &[
                    (MsaaQuality:: Disabled, &t!("default")),
                    (MsaaQuality::_2x, "2x"),
                    (MsaaQuality::_4x, "4x"),
                    (MsaaQuality::_8x, "8x")
                ]);
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.aniso_level")) {
                ui.label(t!("config_editor.aniso_level"));
                Gui::run_combo(ui, "aniso_level", &mut config.aniso_level, &[
                    (AnisoLevel::Default, &t!("default")),
                    (AnisoLevel::_2x, "2x"),
                    (AnisoLevel::_4x, "4x"),
                    (AnisoLevel::_8x, "8x"),
                    (AnisoLevel::_16x, "16x")
                ]);
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.shadow_resolution")) {
                ui.label(t!("config_editor.shadow_resolution"));
                Gui::run_combo(ui, "shadow_resolution", &mut config.shadow_resolution, &[
                    (ShadowResolution::Default, &t!("default")),
                    (ShadowResolution::_256, "256x"),
                    (ShadowResolution::_512, "512x"),
                    (ShadowResolution::_1024, "1K"),
                    (ShadowResolution::_2048, "2K"),
                    (ShadowResolution::_4096, "4K")
                ]);
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.graphics_quality")) {
                ui.label(t!("config_editor.graphics_quality"));
                Gui::run_combo(ui, "graphics_quality", &mut config.graphics_quality, &[
                    (GraphicsQuality::Default, &t!("default")),
                    (GraphicsQuality::Toon1280, "Toon1280"),
                    (GraphicsQuality::Toon1280x2, "Toon1280x2"),
                    (GraphicsQuality::Toon1280x4, "Toon1280x4"),
                    (GraphicsQuality::ToonFull, "ToonFull"),
                    (GraphicsQuality::Max, "Max")
                ]);
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                use crate::windows::hachimi_impl::{FullScreenMode, ResolutionScaling};
                let supports_freeform_window = Hachimi::instance().game.region != Region::Global;

                if should_show_option(search, &t!("config_editor.vsync")) {
                    ui.label(t!("config_editor.vsync"));
                    Gui::run_vsync_combo(ui, &mut config.windows.vsync_count);
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.auto_full_screen")) {
                    ui.label(t!("config_editor.auto_full_screen"));
                    ui.checkbox(&mut config.windows.auto_full_screen, "");
                    ui.end_row();
                }

                if supports_freeform_window &&
                    should_show_option(search, &t!("config_editor.freeform_window"))
                {
                    ui.label(t!("config_editor.freeform_window"));
                    ui.checkbox(&mut config.windows.freeform_window, "");
                    ui.end_row();
                }

                if supports_freeform_window && config.windows.freeform_window {
                    if should_show_option(search, &t!("config_editor.freeform_ui_scale_auto")) {
                        ui.label(t!("config_editor.freeform_ui_scale_auto"));
                        ui.checkbox(&mut config.windows.freeform_ui_scale_auto, "");
                        ui.end_row();
                    }

                    if config.windows.freeform_ui_scale_auto &&
                        should_show_option(search, &t!("config_editor.freeform_ui_scale_auto_ratio"))
                    {
                        ui.label(t!("config_editor.freeform_ui_scale_auto_ratio"));
                        ui.add(
                            egui::Slider::new(
                                &mut config.windows.freeform_ui_scale_auto_ratio,
                                0.25..=3.0
                            )
                                .step_by(0.05)
                                .fixed_decimals(2)
                        );
                        ui.end_row();
                    }
                }

                if should_show_option(search, &t!("config_editor.full_screen_mode")) {
                    ui.label(t!("config_editor.full_screen_mode"));
                    Gui::run_combo(ui, "full_screen_mode", &mut config.windows.full_screen_mode, &[
                        (FullScreenMode::ExclusiveFullScreen, &t!("config_editor.full_screen_mode_exclusive")),
                        (FullScreenMode::FullScreenWindow, &t!("config_editor.full_screen_mode_borderless"))
                    ]);
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.block_minimize_in_full_screen")) {
                    ui.label(t!("config_editor.block_minimize_in_full_screen"));
                    ui.checkbox(&mut config.windows.block_minimize_in_full_screen, "");
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.resolution_scaling")) {
                    ui.label(t!("config_editor.resolution_scaling"));
                    Gui::run_combo(ui, "resolution_scaling", &mut config.windows.resolution_scaling, &[
                        (ResolutionScaling::Default, &t!("config_editor.resolution_scaling_default")),
                        (ResolutionScaling::ScaleToScreenSize, &t!("config_editor.resolution_scaling_ssize")),
                        (ResolutionScaling::ScaleToWindowSize, &t!("config_editor.resolution_scaling_wsize"))
                    ]);
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.window_always_on_top")) {
                    ui.label(t!("config_editor.window_always_on_top"));
                    ui.checkbox(&mut config.windows.window_always_on_top, "");
                    ui.end_row();
                }
            }
        }
        // Graphics tab end

        // Gameplay tab
        if show_all || tab == ConfigEditorTab::Gameplay {
            if should_show_option(search, &t!("config_editor.physics_update_mode")) {
                ui.label(t!("config_editor.physics_update_mode"));
                Gui::run_combo(ui, "physics_update_mode", &mut config.physics_update_mode, &[
                    (None, &t!("default")),
                    (SpringUpdateMode::ModeNormal.into(), "ModeNormal"),
                    (SpringUpdateMode::Mode60FPS.into(), "Mode60FPS"),
                    (SpringUpdateMode::SkipFrame.into(), "SkipFrame"),
                    (SpringUpdateMode::SkipFramePostAlways.into(), "SkipFramePostAlways")
                ]);
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.cyspring_mono_uncap_frame_scale")) {
                ui.label(t!("config_editor.cyspring_mono_uncap_frame_scale"));
                ui.checkbox(&mut config.cyspring_mono_uncap_frame_scale, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.story_choice_auto_select_delay")) {
                ui.label(t!("config_editor.story_choice_auto_select_delay"));
                ui.add(egui::Slider::new(&mut config.story_choice_auto_select_delay, 0.1..=10.0).step_by(0.05));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.story_text_speed_multiplier")) {
                ui.label(t!("config_editor.story_text_speed_multiplier"));
                ui.add(egui::Slider::new(&mut config.story_tcps_multiplier, 0.1..=10.0).step_by(0.1));
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.force_allow_dynamic_camera")) {
                ui.label(t!("config_editor.force_allow_dynamic_camera"));
                ui.checkbox(&mut config.force_allow_dynamic_camera, "");
                ui.end_row();
            }

            #[cfg(target_os = "windows")]
            {
                if should_show_option(search, &t!("config_editor.free_camera")) {
                    ui.label(t!("config_editor.free_camera"));
                    let was_enabled = config.free_camera.enabled;
                    if ui.checkbox(&mut config.free_camera.enabled, "").changed() &&
                        !was_enabled &&
                        config.free_camera.enabled
                    {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                                .lock().unwrap()
                                .show_notification(&t!("notification.free_camera_input_disabled"));
                        });
                    }
                    ui.end_row();

                    ui.label("");
                    if ui.button(t!("config_editor.free_camera_settings")).clicked() {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(FreeCameraSettingsWindow::new()));
                        });
                    }
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.live_theater_allow_same_chara")) {
                ui.label(t!("config_editor.live_theater_allow_same_chara"));
                ui.checkbox(&mut config.live_theater_allow_same_chara, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.live_vocals_swap")) {
                ui.label(t!("config_editor.live_vocals_swap"));
                ui.horizontal(|ui| {
                    if ui.button(t!("open")).clicked() {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(LiveVocalsSwapWindow::new()));
                        });
                    }
                });
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.skill_info_dialog")) {
                ui.label(t!("config_editor.skill_info_dialog"));
                ui.checkbox(&mut config.skill_info_dialog, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.homescreen_bgseason")) {
                ui.label(t!("config_editor.homescreen_bgseason"));
                // Season text from TextId enum
                let default_label = t!("default");
                let spring = get_localized_string("Common0108");
                let summer = get_localized_string("Common0109");
                let fall = get_localized_string("Common0110");
                let winter = get_localized_string("Common0111");
                let cherry = get_localized_string("Common0112");

                let mut seasons: Vec<(BgSeason, &str)> = vec![
                    (BgSeason::None, &default_label),
                    (BgSeason::Spring, spring.as_str())
                ];
                if Hachimi::instance().game.region == Region::Japan {
                    seasons.push((BgSeason::Summer, summer.as_str()));
                    seasons.push((BgSeason::Fall, fall.as_str()));
                    seasons.push((BgSeason::Winter, winter.as_str()));
                    seasons.push((BgSeason::CherryBlossom, cherry.as_str()));
                }
                Gui::run_combo(ui, "homescreen_bgseason", &mut config.homescreen_bgseason, &seasons);
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.disable_skill_name_translation")) {
                ui.label(t!("config_editor.disable_skill_name_translation"));
                ui.checkbox(&mut config.disable_skill_name_translation, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.hide_ingame_ui_hotkey")) {
                ui.label(t!("config_editor.hide_ingame_ui_hotkey"));
                if ui.checkbox(&mut config.hide_ingame_ui_hotkey, "").clicked() {
                    if config.hide_ingame_ui_hotkey {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(SimpleOkDialog::new(
                                &t!("info"),
                                &t!("config_editor.hide_ingame_ui_hotkey_info"),
                                false,
                                || {}
                            )));
                        });
                    }
                }
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.live_slider_always_show")) {
                ui.label(t!("config_editor.live_slider_always_show"));
                ui.checkbox(&mut config.live_slider_always_show, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.live_playback_loop")) {
                ui.label(t!("config_editor.live_playback_loop"));
                ui.checkbox(&mut config.live_playback_loop, "");
                ui.end_row();
            }

            if should_show_option(search, &t!("config_editor.champions_live_show_text")) {
                ui.label(t!("config_editor.champions_live_show_text"));
                ui.checkbox(&mut config.champions_live_show_text, "");
                ui.end_row();
            }

            if config.champions_live_show_text {
                if should_show_option(search, &t!("config_editor.champions_live_resource_id")) {
                    ui.label(t!("config_editor.champions_live_resource_id"));
                    let mut choices: Vec<(i32, &str)> = Vec::new();
                    for (i, name) in self.champions_resources.iter().enumerate() {
                        choices.push(((i + 1) as i32, name.as_str()));
                    }
                    Gui::run_combo(ui, "champions_live_resource_id", &mut config.champions_live_resource_id, &choices);
                    ui.end_row();
                    ui.label(t!("config_editor.champions_live_year"));
                    ui.add(egui::DragValue::new(&mut config.champions_live_year).range(2022..=self.champions_live_max_year));
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.captions")) {
                ui.label(t!("config_editor.captions"));
                ui.checkbox(&mut config.caption.caption_enable, "");
                ui.end_row();
            }

            if config.caption.caption_enable {
                if should_show_option(search, &t!("config_editor.caption_lines_char_count")) {
                    ui.label(t!("config_editor.caption_lines_char_count"));
                    ui.add(egui::Slider::new(&mut config.caption.caption_lines_char_count, 10..=100));
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_font_size")) {
                    ui.label(t!("config_editor.caption_font_size"));
                    ui.add(egui::Slider::new(&mut config.caption.caption_font_size, 10..=128));
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_pos_x")) {
                    ui.label(t!("config_editor.caption_pos_x"));
                    ui.add(egui::Slider::new(&mut config.caption.caption_pos_x, -10.0..=10.0));
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_pos_y")) {
                    ui.label(t!("config_editor.caption_pos_y"));
                    ui.add(egui::Slider::new(&mut config.caption.caption_pos_y, -10.0..=10.0));
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_bg_alpha")) {
                    ui.label(t!("config_editor.caption_bg_alpha"));
                    ui.add(egui::Slider::new(&mut config.caption.caption_bg_alpha, 0.0..=1.0));
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_color")) {
                    ui.label(t!("config_editor.caption_color"));
                    egui::ComboBox::new(ui.id().with("caption_color"), "")
                        .selected_text(&config.caption.caption_color)
                        .show_ui(ui, |ui| {
                            for option in &self.font_color_options {
                                ui.selectable_value(&mut config.caption.caption_color, option.clone(), option);
                            }
                        });
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_outline_size")) {
                    ui.label(t!("config_editor.caption_outline_size"));
                    egui::ComboBox::new(ui.id().with("caption_outline_size"), "")
                        .selected_text(&config.caption.caption_outline_size)
                        .show_ui(ui, |ui| {
                            for option in &self.outline_size_options {
                                ui.selectable_value(&mut config.caption.caption_outline_size, option.clone(), option);
                            }
                        });
                    ui.end_row();
                }

                if should_show_option(search, &t!("config_editor.caption_outline_color")) {
                    ui.label(t!("config_editor.caption_outline_color"));
                    egui::ComboBox::new(ui.id().with("caption_outline_color"), "")
                        .selected_text(&config.caption.caption_outline_color)
                        .show_ui(ui, |ui| {
                            for option in &self.outline_color_options {
                                ui.selectable_value(&mut config.caption.caption_outline_color, option.clone(), option);
                            }
                        });
                    ui.end_row();
                }
            }

            if should_show_option(search, &t!("config_editor.hide_ingame_ui_hotkey_bind")) {
                ui.label(t!("config_editor.hide_ingame_ui_hotkey_bind"));
                ui.horizontal(|ui| {
                    #[cfg(target_os = "windows")]
                    ui.label(crate::windows::utils::vk_to_display_label(config.windows.hide_ingame_ui_hotkey_bind));
                    #[cfg(target_os = "android")]
                    ui.label(crate::android::gui_impl::keymap::keycode_display_label(config.android.hide_ingame_ui_hotkey_bind));

                    if ui.button(t!("bind_key")).clicked() {
                        std::thread::spawn(|| {
                            let Some(gui_mutex) = Gui::instance() else { return };
                            let mut gui = gui_mutex.lock().unwrap();
                            gui.show_window(Box::new(SetKeybindWindow::new(|result| {
                                let Some(raw) = result else { return };

                                let hachimi = Hachimi::instance();
                                let mut new_config = hachimi.config.load().as_ref().clone();

                                #[cfg(target_os = "windows")]
                                { new_config.windows.hide_ingame_ui_hotkey_bind = raw; }
                                #[cfg(target_os = "android")]
                                { new_config.android.hide_ingame_ui_hotkey_bind = raw; }

                                save_and_reload_config(new_config);
                            })));
                        });
                    }
                });
                ui.end_row();
            }
        }
        // Gameplay tab end

        // Column widths workaround
        ui.horizontal(|ui| ui.add_space(100.0 * scale));
        ui.horizontal(|ui| ui.add_space(150.0 * scale));
        ui.end_row();
    }
}

impl Window for ConfigEditor {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);

        let mut open = true;
        let mut open2 = true;
        let global_handle = Hachimi::instance().config.load();
        let global_ptr = Arc::as_ptr(&global_handle) as usize;

        // sync config between diff windows
        if global_ptr != self.last_ptr_config {
            self.config = (**global_handle).clone();
            self.last_ptr_config = global_ptr;
        }
        let mut config = self.config.clone();
        #[cfg(target_os = "windows")]
        {
            config.windows.menu_open_key = global_handle.windows.menu_open_key;
        }
        let mut reset_clicked = false;
        let mut save_clicked = false;

        new_window(ctx, self.id, t!("config_editor.title"))
        .max_height(270.0 * scale + {
            #[cfg(target_os = "android")]
            { ime_scroll_padding(ctx) }
            #[cfg(target_os = "windows")]
            { 0.0 }
        })
        .open(&mut open)
        .show(ctx, |ui| {
            simple_window_layout(ui, self.id,
                |ui| {
                    ui.horizontal(|ui| {
                        // search bar
                        let _search_res = ui.add_sized(
                            [ui.available_width() - 30.0 * scale, 24.0 * scale],
                            egui::TextEdit::singleline(&mut self.search_term).hint_text(t!("search_filter"))
                        );
                        #[cfg(target_os = "android")]
                        handle_android_keyboard(&_search_res, &mut self.search_term);

                        if ui.button("\u{f00d}").clicked() {
                            self.search_term.clear();
                        }
                    });
                    ui.add_space(4.0);

                    if self.search_term.is_empty() {
                        egui::ScrollArea::horizontal()
                        .id_salt("tabs_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let style = ui.style_mut();
                                style.spacing.button_padding = egui::vec2(8.0, 5.0);
                                style.spacing.item_spacing = egui::Vec2::ZERO;
                                let widgets = &mut style.visuals.widgets;
                                widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
                                widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
                                widgets.active.corner_radius = egui::CornerRadius::ZERO;
    
                                for (tab, label) in ConfigEditorTab::display_list() {
                                    if ui.selectable_label(self.current_tab == tab, label.as_ref()).clicked() {
                                        self.current_tab = tab;
                                    }
                                }
                            });
                        });
                    }

                    ui.add_space(4.0);

                    ui.scope(|ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical()
                        .id_salt("body_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(8, 0))
                            .show(ui, |ui| {
                                egui::Grid::new(self.id.with("options_grid"))
                                .striped(true)
                                .num_columns(2)
                                .spacing([40.0 * scale, 4.0 * scale])
                                .show(ui, |ui| {
                                    self.run_options_grid(&mut config, ui, self.current_tab, &self.search_term);
                                });
                            });
                            #[cfg(target_os = "android")]
                            {
                                let padding = ime_scroll_padding(ui.ctx());
                                if padding > 0.0 {
                                    ui.add_space(padding);
                                }
                            }
                        });
                    });
                },
                |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui.button(t!("config_editor.restore_defaults")).clicked() {
                            reset_clicked = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button(t!("cancel")).clicked() {
                                open2 = false;
                            }
                            if ui.button(t!("save")).clicked() {
                                save_clicked = true;
                                open2 = false;
                            }
                        });
                    });
                }
            );
        });

        self.config = config;

        if save_clicked {
            #[cfg(target_os = "windows")]
            {
                *PENDING_TITLE.lock().unwrap() = Some(self.config.windows.custom_title_name.clone());
                use windows::{core::HSTRING, Win32::UI::WindowsAndMessaging::SetWindowTextW};
                Thread::main_thread().schedule(move || {
                    let hwnd = crate::windows::wnd_hook::get_target_hwnd();
                    let title = PENDING_TITLE.lock().unwrap().take().flatten();
                    if let Some(t) = title {
                        let _ = unsafe { SetWindowTextW(hwnd, &HSTRING::from(t.as_str())) };
                    } else {
                        let hachimi = Hachimi::instance();
                        let default_title = if hachimi.game.region == Region::Japan && hachimi.game.is_steam_release {
                            HSTRING::from("UmamusumePrettyDerby_Jpn")
                        } else {
                            HSTRING::from("umamusume")
                        };
                        let _ = unsafe { SetWindowTextW(hwnd, &default_title) };
                    }
                });
            }
            save_and_reload_config(self.config.clone());
        }

        if reset_clicked {
            self.restore_defaults();
        }

        open &= open2;
        if !open {
            let config_locale = Hachimi::instance().config.load().language.locale_str();
            if config_locale != &*rust_i18n::locale() {
                rust_i18n::set_locale(config_locale);
            }
        }

        open
    }
}

fn save_and_reload_config(config: hachimi::Config) {
    let notif = match Hachimi::instance().save_and_reload_config(config) {
        Ok(_) => {
            #[cfg(target_os = "windows")]
            crate::windows::wnd_hook::apply_freeform_window_config();
            free_camera::reload_runtime_config();

            t!("notification.config_saved").into_owned()
        },
        Err(e) => e.to_string()
    };

    // workaround since we can't get a mutable ref to the Gui and
    // locking the mutex on the current thread would cause a deadlock
    thread::spawn(move || {
        Gui::instance().unwrap()
        .lock().unwrap()
        .show_notification(&notif);
    });
}

struct FirstTimeSetupWindow {
    id: egui::Id,
    meta_index_url: String,
    config: hachimi::Config,
    index_request: Arc<AsyncRequest<Vec<RepoInfo>>>,
    current_page: usize,
    current_tl_repo: Option<String>,
    has_auto_selected: bool
}

impl FirstTimeSetupWindow {
    fn new() -> FirstTimeSetupWindow {
        let config = (**Hachimi::instance().config.load()).clone();
        FirstTimeSetupWindow {
            id: random_id(),
            meta_index_url: config.meta_index_url.clone(),
            config,
            index_request: Arc::new(tl_repo::new_meta_index_request()),
            current_page: 0,
            current_tl_repo: None,
            has_auto_selected: false
        }
    }
}

impl Window for FirstTimeSetupWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut page_open = true;

        new_window(ctx, self.id, t!("first_time_setup.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            let allow_next = match self.current_page {
                1 => {
                    (**self.index_request.result.load()).as_ref().map_or(false, |r| r.is_ok())
                },
                _ => true
            };

            page_open = paginated_window_layout(ui, self.id, &mut self.current_page, 4, allow_next, |ui, i| {
                match i {
                    0 => {
                        ui.heading(t!("first_time_setup.welcome_heading"));
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.language"));
                            let mut language = self.config.language;
                            let lang_changed = Gui::run_combo(ui, "language", &mut language, Language::CHOICES);
                            if lang_changed {
                                self.config.language = language;
                                save_and_reload_config(self.config.clone());
                                self.current_tl_repo = None;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.meta_index_url"));
                            let res = ui.add(egui::TextEdit::singleline(&mut self.meta_index_url).lock_focus(true));
                            #[cfg(target_os = "android")]
                            handle_android_keyboard(&res, &mut self.meta_index_url);
                            #[cfg(target_os = "windows")]
                            if res.has_focus() {
                                ui.memory_mut(|mem| mem.set_focus_lock_filter(
                                    res.id,
                                    egui::EventFilter {
                                        tab: true,
                                        horizontal_arrows: true,
                                        vertical_arrows: true,
                                        escape: true,
                                        ..Default::default()
                                    }
                                ));
                            }

                            if res.lost_focus() {
                                if self.meta_index_url.trim().is_empty() {
                                    self.meta_index_url = hachimi::Config::default().meta_index_url;
                                }
                                if self.meta_index_url != self.config.meta_index_url {
                                    self.config.meta_index_url = self.meta_index_url.clone();
                                    save_and_reload_config(self.config.clone());
                                    self.index_request = Arc::new(tl_repo::new_meta_index_request());
                                }
                            }
                        });
                        ui.separator();
                        ui.label(t!("first_time_setup.welcome_content"));
                    }
                    1 => {
                        ui.heading(t!("first_time_setup.translation_repo_heading"));
                        ui.separator();
                        ui.label(t!("first_time_setup.select_translation_repo"));
                        ui.add_space(4.0);

                        let mut retry_clicked = false;

                        tl_repo_list_ui(
                            ui,
                            &self.index_request,
                            || retry_clicked = true,
                            &mut self.current_tl_repo,
                            &mut self.has_auto_selected,
                            self.config.language.locale_str(),
                            true,
                            false
                        );

                        if retry_clicked {
                            self.index_request = Arc::new(tl_repo::new_meta_index_request());
                        }
                    }
                    2 => {
                        ui.heading(t!("first_time_setup.common_settings_heading"));
                        ui.separator();
                        ui.label(t!("first_time_setup.common_settings_content"));
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.target_fps"));
                            let mut enabled = self.config.target_fps.is_some();
                            if ui.checkbox(&mut enabled, t!("enable")).changed() {
                                if enabled {
                                    self.config.target_fps = Some(60);
                                } else {
                                    self.config.target_fps = None;
                                }
                            }
                        });
                        if let Some(ref mut fps) = self.config.target_fps {
                            ui.horizontal(|ui| {
                                ui.label("");
                                let _ = ui.add(egui::Slider::new(fps, 30..=1000));
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.disable_skill_name_translation"));
                            let _ = ui.checkbox(&mut self.config.disable_skill_name_translation, "");
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.menu_open_key"));
                            #[cfg(target_os = "windows")]
                            ui.label(crate::windows::utils::vk_to_display_label(self.config.windows.menu_open_key));
                            #[cfg(target_os = "android")]
                            ui.label(crate::android::gui_impl::keymap::keycode_display_label(self.config.android.menu_open_key));

                            if ui.button(t!("bind_key")).clicked() {
                                let config_clone = self.config.clone();
                                std::thread::spawn(move || {
                                    let Some(gui_mutex) = Gui::instance() else { return };
                                    let mut gui = gui_mutex.lock().unwrap();
                                    gui.show_window(Box::new(SetKeybindWindow::new(move |result| {
                                        let Some(raw) = result else { return };

                                        let mut new_config = config_clone.clone();

                                        #[cfg(target_os = "windows")]
                                        { new_config.windows.menu_open_key = raw; }
                                        #[cfg(target_os = "android")]
                                        { new_config.android.menu_open_key = raw; }

                                        save_and_reload_config(new_config);
                                    })));
                                });
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("config_editor.ui_animation_scale"));
                            let _ = ui.add(egui::Slider::new(&mut self.config.ui_animation_scale, 0.1..=10.0).step_by(0.1));
                        });
                    }
                    3 => {
                        ui.heading(t!("first_time_setup.complete_heading"));
                        ui.separator();
                        ui.label(t!("first_time_setup.complete_content"));
                    }
                    _ => {}
                }
            });
        });

        let open_res = open && page_open;
        if !open_res {
            self.config.skip_first_time_setup = true;

            if !page_open {
                // user selected a tl repo (or skipped)
                if let Some(ref index) = self.current_tl_repo {
                    let hachimi = Hachimi::instance();
                    let mut manager = hachimi.tl_repo_manager.lock().unwrap();
                    let repos_path = hachimi.get_data_path(".tl_repos");

                    let id = if let Some(existing) = manager.find_by_index(index) {
                        existing
                    } else {
                        let new_id = manager.add(index.clone());
                        if let Err(e) = manager.save(&repos_path) {
                            warn!("Failed to persist .tl_repos: {e}");
                        }
                        new_id
                    };

                    self.config.selected_tl_repo_id = Some(id);
                    self.config.translation_repo_index = Some(index.clone());
                } else {
                    self.config.translation_repo_index = None;
                    self.config.selected_tl_repo_id = None;
                }
            }

            save_and_reload_config(self.config.clone());

            if !page_open {
                Hachimi::instance().tl_updater.clone().check_for_updates(false, false);
            }
        }

        open_res
    }
}

struct LiveVocalsSwapWindow {
    id: egui::Id,
    config: hachimi::Config,
    chara_choices: Vec<(i32, String)>,
    search_term: String
}

impl LiveVocalsSwapWindow {
    fn new() -> LiveVocalsSwapWindow {
        let hachimi = Hachimi::instance();
        let mut chara_choices: Vec<(i32, String)> = Vec::new();
        chara_choices.push((0, t!("default").into_owned()));

        let data = hachimi.chara_data.load();
        for &id in &data.chara_ids {
            chara_choices.push((id, data.get_name(id)));
        }
        chara_choices.sort_by_key(|choice| choice.0);

        LiveVocalsSwapWindow {
            id: random_id(),
            config: (**hachimi.config.load()).clone(),
            chara_choices,
            search_term: String::new()
        }
    }
}

impl Window for LiveVocalsSwapWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;

        let combo_items: Vec<(i32, &str)> = self.chara_choices
            .iter()
            .map(|&(id, ref name)| (id, name.as_str()))
            .collect();

        new_window(ctx, self.id, t!("config_editor.live_vocals_swap"))
        .open(&mut open)
        .show(ctx, |ui| {
            simple_window_layout(ui, self.id,
                |ui| {
                    egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .show(ui, |ui| {
                        egui::Grid::new(self.id.with("live_vocals_swap_grid"))
                        .striped(true)
                        .num_columns(2)
                        .spacing([40.0 * scale, 4.0 * scale])
                        .show(ui, |ui| {
                            for i in 0..6 {
                                ui.label(t!("config_editor.live_vocals_swap_character_n", index = i + 1));
                                Gui::run_combo_menu(ui, egui::Id::new("vocals_swap").with(i), &mut self.config.live_vocals_swap[i], &combo_items, &mut self.search_term);
                                ui.end_row();
                            }
                        });
                    });
                },
                |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button(t!("cancel")).clicked() {
                                open2 = false;
                            }
                            if ui.button(t!("save")).clicked() {
                                save_clicked = true;
                                open2 = false;
                            }
                        });
                    });
                }
            );
        });

        if save_clicked {
            save_and_reload_config(self.config.clone());
        }

        open &= open2;
        open
    }
}

struct FreeCameraSettingsWindow {
    id: egui::Id,
    config: hachimi::Config,
}

impl FreeCameraSettingsWindow {
    fn new() -> FreeCameraSettingsWindow {
        FreeCameraSettingsWindow {
            id: random_id(),
            config: (**Hachimi::instance().config.load()).clone(),
        }
    }

    #[cfg(target_os = "windows")]
    fn keybind_row(
        ui: &mut egui::Ui,
        label: Cow<'static, str>,
        key: u16,
        setter: fn(&mut free_camera::FreeCameraKeybinds, u16),
    ) {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.label(crate::windows::utils::vk_to_display_label(key));
            if ui.button(t!("bind_key")).clicked() {
                Self::open_keybind_window(setter);
            }
        });
        ui.end_row();
    }

    #[cfg(target_os = "windows")]
    fn open_keybind_window(setter: fn(&mut free_camera::FreeCameraKeybinds, u16)) {
        thread::spawn(move || {
            let Some(gui_mutex) = Gui::instance() else { return };
            let mut gui = gui_mutex.lock().unwrap();
            gui.show_window(Box::new(SetKeybindWindow::new(move |result| {
                let Some(raw) = result else { return };

                let hachimi = Hachimi::instance();
                let mut new_config = hachimi.config.load().as_ref().clone();
                setter(&mut new_config.free_camera.keybinds, raw);
                save_and_reload_config(new_config);
            })));
        });
    }
}

impl Window for FreeCameraSettingsWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;
        let mut reset_clicked = false;

        #[cfg(target_os = "windows")]
        {
            self.config.free_camera.keybinds =
                Hachimi::instance().config.load().free_camera.keybinds.clone();
        }

        let mode_free = t!("free_camera.mode_free");
        let mode_first_person = t!("free_camera.mode_first_person");
        let mode_selfie_stick = t!("free_camera.mode_selfie_stick");
        let mode_choices = [
            (FreeCameraMode::Free, mode_free.as_ref()),
            (FreeCameraMode::FirstPerson, mode_first_person.as_ref()),
            (FreeCameraMode::SelfieStick, mode_selfie_stick.as_ref()),
        ];
        let live_position_choices: Vec<(i32, &str)> = free_camera::LIVE_POSITION_CHOICES
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (i as i32, *name))
            .collect();
        let live_part_choices: Vec<(i32, &str)> = free_camera::LIVE_PART_CHOICES
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (i as i32, *name))
            .collect();

        new_window(ctx, self.id, t!("free_camera.title"))
        .default_width(340.0 * scale)
        .max_width(360.0 * scale)
        .max_height(430.0 * scale)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt(self.id.with("free_camera_settings_scroll"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(4, 0))
                        .show(ui, |ui| {
                            egui::Grid::new(self.id.with("free_camera_settings_grid"))
                            .striped(true)
                            .num_columns(2)
                            .spacing([12.0 * scale, 4.0 * scale])
                            .show(ui, |ui| {
                                let cfg = &mut self.config.free_camera;

                                ui.strong(t!("free_camera.section_general"));
                                ui.label("");
                                ui.end_row();

                                ui.label(t!("config_editor.free_camera"));
                                ui.checkbox(&mut cfg.enabled, "");
                                ui.end_row();

                                ui.label(t!("free_camera.remove_camera_effects"));
                                ui.checkbox(&mut cfg.remove_camera_effects, "");
                                ui.end_row();

                                ui.label(t!("free_camera.show_overlay"));
                                ui.checkbox(&mut cfg.show_overlay, "");
                                ui.end_row();

                                ui.label(t!("free_camera.selfie_use_head_transform"));
                                ui.checkbox(&mut cfg.selfie_use_head_transform, "");
                                ui.end_row();

                                ui.label(t!("free_camera.mode"));
                                Gui::run_combo(ui, "free_camera_mode", &mut cfg.mode, &mode_choices);
                                ui.end_row();

                                ui.label(t!("free_camera.live_move_step"));
                                ui.add(egui::DragValue::new(&mut cfg.live_move_step).speed(0.01).range(0.001..=100.0));
                                ui.end_row();

                                ui.label(t!("free_camera.race_move_step"));
                                ui.add(egui::DragValue::new(&mut cfg.race_move_step).speed(0.1).range(0.001..=100.0));
                                ui.end_row();

                                ui.label(t!("free_camera.look_step"));
                                ui.add(egui::DragValue::new(&mut cfg.look_step).speed(0.05).range(0.001..=30.0));
                                ui.end_row();

                                #[cfg(target_os = "windows")]
                                {
                                    ui.label(t!("free_camera.mouse_speed"));
                                    ui.add(egui::DragValue::new(&mut cfg.mouse_speed).speed(1.0).range(1.0..=1000.0));
                                    ui.end_row();
                                }

                                ui.label(t!("free_camera.live_fov"));
                                ui.add(egui::DragValue::new(&mut cfg.live_fov).speed(0.5).range(1.0..=120.0));
                                ui.end_row();

                                ui.label(t!("free_camera.race_fov"));
                                ui.add(egui::DragValue::new(&mut cfg.race_fov).speed(0.5).range(1.0..=120.0));
                                ui.end_row();

                                ui.label(t!("free_camera.gamepad_deadzone"));
                                ui.add(egui::Slider::new(&mut cfg.gamepad_deadzone, 0.0..=0.8).step_by(0.01));
                                ui.end_row();

                                ui.label(t!("free_camera.gamepad_move_speed"));
                                ui.add(egui::DragValue::new(&mut cfg.gamepad_move_speed).speed(0.05).range(0.1..=10.0));
                                ui.end_row();

                                ui.label(t!("free_camera.gamepad_look_speed"));
                                ui.add(egui::DragValue::new(&mut cfg.gamepad_look_speed).speed(0.05).range(0.1..=10.0));
                                ui.end_row();

                                ui.strong(t!("free_camera.section_live"));
                                ui.label("");
                                ui.end_row();

                                ui.label(t!("free_camera.live_disable_character_teleport"));
                                ui.checkbox(&mut cfg.live_disable_character_teleport, "");
                                ui.end_row();

                                ui.label(t!("free_camera.live_force_all_characters_visible"));
                                ui.checkbox(&mut cfg.live_force_all_characters_visible, "");
                                ui.end_row();

                                ui.label(t!("free_camera.live_target_position"));
                                Gui::run_combo(ui, "free_camera_live_target_position", &mut cfg.live_target_position_index, &live_position_choices);
                                ui.end_row();

                                ui.label(t!("free_camera.live_target_part"));
                                Gui::run_combo(ui, "free_camera_live_target_part", &mut cfg.live_target_part_index, &live_part_choices);
                                ui.end_row();

                                ui.label(t!("free_camera.live_selfie_horizontal_stabilization"));
                                ui.add(egui::DragValue::new(&mut cfg.live_selfie_horizontal_stabilization).speed(0.01).range(0.0..=5.0));
                                ui.end_row();

                                ui.label(t!("free_camera.live_selfie_vertical_stabilization"));
                                ui.add(egui::DragValue::new(&mut cfg.live_selfie_vertical_stabilization).speed(0.01).range(0.0..=5.0));
                                ui.end_row();

                                ui.label(t!("free_camera.live_follow_smooth"));
                                ui.checkbox(&mut cfg.live_follow_smooth, "");
                                ui.end_row();

                                ui.label(t!("free_camera.live_follow_smooth_pos_step"));
                                ui.add(egui::DragValue::new(&mut cfg.live_follow_smooth_pos_step).speed(0.01).range(0.02..=1.0));
                                ui.end_row();

                                ui.label(t!("free_camera.live_follow_smooth_lookat_step"));
                                ui.add(egui::DragValue::new(&mut cfg.live_follow_smooth_lookat_step).speed(0.01).range(0.02..=1.0));
                                ui.end_row();

                                ui.strong(t!("free_camera.section_race"));
                                ui.label("");
                                ui.end_row();

                                ui.label(t!("free_camera.race_target_index"));
                                ui.add(egui::DragValue::new(&mut cfg.race_target_index).speed(1.0).range(-1..=17));
                                ui.end_row();

                                #[cfg(target_os = "windows")]
                                {
                                    ui.strong(t!("free_camera.section_keybinds"));
                                    ui.label("");
                                    ui.end_row();

                                    macro_rules! keybind_row {
                                        ($field:ident, $key:literal) => {{
                                            let setter: fn(&mut free_camera::FreeCameraKeybinds, u16) =
                                                |keybinds, raw| keybinds.$field = raw;
                                            Self::keybind_row(
                                                ui,
                                                t!($key),
                                                cfg.keybinds.$field,
                                                setter,
                                            );
                                        }};
                                    }

                                    keybind_row!(move_forward, "free_camera.key_move_forward");
                                    keybind_row!(move_back, "free_camera.key_move_back");
                                    keybind_row!(move_left, "free_camera.key_move_left");
                                    keybind_row!(move_right, "free_camera.key_move_right");
                                    keybind_row!(move_down, "free_camera.key_move_down");
                                    keybind_row!(move_up, "free_camera.key_move_up");
                                    keybind_row!(look_up, "free_camera.key_look_up");
                                    keybind_row!(look_down, "free_camera.key_look_down");
                                    keybind_row!(look_left, "free_camera.key_look_left");
                                    keybind_row!(look_right, "free_camera.key_look_right");
                                    keybind_row!(fov_increase, "free_camera.key_fov_increase");
                                    keybind_row!(fov_decrease, "free_camera.key_fov_decrease");
                                    keybind_row!(follow_offset_up, "free_camera.key_follow_offset_up");
                                    keybind_row!(follow_offset_down, "free_camera.key_follow_offset_down");
                                    keybind_row!(follow_offset_left, "free_camera.key_follow_offset_left");
                                    keybind_row!(follow_offset_right, "free_camera.key_follow_offset_right");
                                    keybind_row!(target_previous, "free_camera.key_target_previous");
                                    keybind_row!(target_next, "free_camera.key_target_next");
                                    keybind_row!(part_previous, "free_camera.key_part_previous");
                                    keybind_row!(part_next, "free_camera.key_part_next");
                                    keybind_row!(reset, "free_camera.key_reset");
                                    keybind_row!(cycle_mode, "free_camera.key_cycle_mode");
                                    keybind_row!(reverse, "free_camera.key_reverse");
                                }
                            });
                        });
                    });

                ui.separator();

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    if ui.button(t!("config_editor.restore_defaults")).clicked() {
                        reset_clicked = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button(t!("cancel")).clicked() {
                            open2 = false;
                        }
                        if ui.button(t!("save")).clicked() {
                            save_clicked = true;
                            open2 = false;
                        }
                    });
                });
            });
        });

        if reset_clicked {
            self.config.free_camera = free_camera::FreeCameraConfig::default();
        }

        if save_clicked {
            save_and_reload_config(self.config.clone());
        }

        open &= open2;
        open
    }
}

struct ThemeEditorWindow {
    id: egui::Id,
    config: hachimi::Config,
    old_config: hachimi::Config
}

impl ThemeEditorWindow {
    fn new() -> ThemeEditorWindow {
        let current_cfg = (**Hachimi::instance().config.load()).clone();
        ThemeEditorWindow {
            id: random_id(),
            config: current_cfg.clone(),
            old_config: current_cfg
        }
    }
}

fn theme_color_row(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) -> bool {
    let mut changed = false;

    ui.columns(2, |cols| {
        cols[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(label);
        });

        cols[1].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.color_edit_button_srgba(color).changed() {
                changed = true;
            }
        });
    });
    ui.end_row();

    changed
}

impl Window for ThemeEditorWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut theme_changed = false;
        let mut cancel_clicked = false;
        let mut save_clicked = false;
        let mut reset_clicked = false;

        new_window(ctx, self.id, t!("theme_editor.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            simple_window_layout(ui, self.id,
                |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                    egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .show(ui, |ui| {
                        egui::Grid::new(self.id.with("theme_editor_grid"))
                        .striped(true)
                        .num_columns(2)
                        .spacing([40.0 * scale, 4.0 * scale])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                theme_changed |= theme_color_row(ui, &t!("theme_editor.ui_accent_color"), &mut self.config.ui_accent_color);
                                theme_changed |= theme_color_row(ui, &t!("theme_editor.ui_window_fill"), &mut self.config.ui_window_fill);
                                theme_changed |= theme_color_row(ui, &t!("theme_editor.ui_panel_fill"), &mut self.config.ui_panel_fill);
                                theme_changed |= theme_color_row(ui, &t!("theme_editor.ui_extreme_bg_color"), &mut self.config.ui_extreme_bg_color);
                                theme_changed |= theme_color_row(ui, &t!("theme_editor.ui_text_color"), &mut self.config.ui_text_color);

                                ui.horizontal(|ui| {
                                    ui.label(t!("theme_editor.ui_window_rounding"));
                                    if ui.add(egui::Slider::new(&mut self.config.ui_window_rounding, 0.0..=20.0)).changed() {
                                        theme_changed = true;
                                    }
                                });
                            });
                        });
                    });
                },
                |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui.button(t!("config_editor.restore_defaults")).clicked() {
                            reset_clicked = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button(t!("cancel")).clicked() {
                                cancel_clicked = true;
                                open2 = false;
                            }
                            if ui.button(t!("save")).clicked() {
                                save_clicked = true;
                                open2 = false;
                            }
                        });
                    });
                }
            );
        });

        if theme_changed {
            enqueue_theme_preview(self.config.clone());
        }

        if cancel_clicked {
            enqueue_theme_preview(self.old_config.clone());
            open2 = false;
        }

        if save_clicked {
            enqueue_theme_preview(self.config.clone());
            save_and_reload_config(self.config.clone());
        }

        if reset_clicked {
            let mut config = self.config.clone();
            config.ui_accent_color = hachimi::Config::default_ui_accent();
            config.ui_window_fill = hachimi::Config::default_window_fill();
            config.ui_panel_fill = hachimi::Config::default_panel_fill();
            config.ui_extreme_bg_color = hachimi::Config::default_extreme_bg();
            config.ui_text_color = hachimi::Config::default_text_color();
            config.ui_window_rounding = hachimi::Config::default_window_rounding();

            self.config = config.clone();
            enqueue_theme_preview(self.config.clone());
        }

        open &= open2;
        open
    }
}

#[derive(PartialEq)]
enum KeybindCapState {
    Waiting,
    Captured { raw: RawKeybind, display: String }
}

pub struct SetKeybindWindow {
    id: egui::Id,
    state: KeybindCapState,
    callback: Option<Box<dyn FnOnce(Option<RawKeybind>) + Send + Sync>>
}

impl SetKeybindWindow {
    pub fn new(
        callback: impl FnOnce(Option<RawKeybind>) + Send + Sync + 'static
    ) -> Self {
        start_keybind_capture();
        Self {
            id: random_id(),
            state: KeybindCapState::Waiting,
            callback: Some(Box::new(callback)),
        }
    }

    fn finish(&mut self, result: Option<RawKeybind>) -> bool {
        if let Some(cb) = self.callback.take() {
            cb(result);
        }
        false
    }
}

impl Window for SetKeybindWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        if self.state == KeybindCapState::Waiting {
            if let Some((raw, display)) = take_keybind_capture() {
                self.state = KeybindCapState::Captured { raw, display };
            }
        }

        let mut confirm_raw: Option<RawKeybind> = None;
        let mut cancelled = false;
        let mut rebind = false;
        let mut open = true;

        new_window(ctx, self.id, t!("set_keybind.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom(self.id.with("buttons"))
                    .show_separator_line(true)
                    .show_inside(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Min),
                            |ui| {
                                if ui.button(t!("cancel")).clicked() {
                                    cancelled = true;
                                }
                                if let KeybindCapState::Captured { raw, .. } = &self.state {
                                    let raw_copy = *raw;
                                    if ui.button(t!("save")).clicked() {
                                        confirm_raw = Some(raw_copy);
                                    }
                                    if ui.button(t!("retry")).clicked() {
                                        rebind = true;
                                    }
                                }
                            },
                        );
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        ui.centered_and_justified(|ui| match &self.state {
                            KeybindCapState::Waiting => {
                                ui.label(t!("set_keybind.press_any_key"));
                            }
                            KeybindCapState::Captured { display, .. } => {
                                ui.label(t!(
                                    "set_keybind.bound_key",
                                    key = display.as_str()
                                ));
                            }
                        });
                    });
            });

        if rebind {
            start_keybind_capture();
            self.state = KeybindCapState::Waiting;
        }

        if !open || cancelled {
            return self.finish(None);
        }

        if let Some(raw) = confirm_raw {
            return self.finish(Some(raw));
        }

        true
    }
}

enum ConfirmAction {
    Remove { index: usize },
    ConfirmEdit { index: usize, value: String },
}

static EXCLUDES_PATHS_CACHE: Lazy<Mutex<Option<(String, Vec<String>)>>> =
    Lazy::new(|| Mutex::new(None));

struct ExcludesEditorWindow {
    id: egui::Id,
    excludes: Vec<String>,
    search_term: String,
    edit_index: Option<usize>,
    edit_value: String,
    available_paths: Vec<String>,
    paths_result: Arc<Mutex<Option<Vec<String>>>>,
    add_selected: usize,
    add_search_term: String,
    confirm_action: Option<ConfirmAction>,
}

impl ExcludesEditorWindow {
    fn new() -> ExcludesEditorWindow {
        let excludes = Self::load_excludes();

        let (available_paths, paths_result) = match Hachimi::instance().get_active_tl_dir() {
            None => (Vec::new(), Arc::new(Mutex::new(None))),
            Some(ld_dir) => {
                let key = ld_dir.to_string_lossy().to_string();
                let cache = EXCLUDES_PATHS_CACHE.lock().unwrap();

                if let Some((cached_key, cached_paths)) = cache.as_ref() {
                    if cached_key == &key {
                        (cached_paths.clone(), Arc::new(Mutex::new(None)))
                    } else {
                        drop(cache);

                        let paths_result = Arc::new(Mutex::new(None));
                        let paths_result_clone = paths_result.clone();
                        let key_clone = key.clone();

                        std::thread::spawn(move || {
                            let paths = Self::get_available_paths();
                            *paths_result_clone.lock().unwrap() = Some(paths.clone());
                            let mut cache = EXCLUDES_PATHS_CACHE.lock().unwrap();
                            *cache = Some((key_clone, paths));
                        });
                        (Vec::new(), paths_result)
                    }
                } else {
                    drop(cache);

                    let paths_result = Arc::new(Mutex::new(None));
                    let paths_result_clone = paths_result.clone();
                    let key_clone = key.clone();

                    std::thread::spawn(move || {
                        let paths = Self::get_available_paths();
                        *paths_result_clone.lock().unwrap() = Some(paths.clone());
                        let mut cache = EXCLUDES_PATHS_CACHE.lock().unwrap();
                        *cache = Some((key_clone, paths));
                    });
                    (Vec::new(), paths_result)
                }
            }
        };

        ExcludesEditorWindow {
            id: random_id(),
            excludes,
            search_term: String::new(),
            edit_index: None,
            edit_value: String::new(),
            available_paths,
            paths_result,
            add_selected: 0,
            add_search_term: String::new(),
            confirm_action: None,
        }
    }

    fn load_excludes() -> Vec<String> {
        let excludes_path = Hachimi::instance().get_data_path(tl_repo::REPO_EXCLUDES_FILENAME);
        if excludes_path.exists() {
            std::fs::read_to_string(&excludes_path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.trim().replace("\\", "/"))
                .filter(|l| !l.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }

    fn save_excludes(excludes: &[String]) -> Result<(), String> {
        let excludes_path = Hachimi::instance().get_data_path(tl_repo::REPO_EXCLUDES_FILENAME);
        let content = excludes.join("\n");
        std::fs::write(&excludes_path, content).map_err(|e| e.to_string())
    }

    fn get_available_paths() -> Vec<String> {
        let Some(ld_dir) = Hachimi::instance().get_active_tl_dir() else {
            return Vec::new();
        };

        if !ld_dir.is_dir() {
            return Vec::new();
        }

        let mut paths: Vec<String> = Vec::new();
        Self::collect_relative_paths(&ld_dir, &ld_dir, &mut paths);
        paths.sort();
        paths
    }

    fn collect_relative_paths(root: &std::path::Path, current: &std::path::Path, paths: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                // skip hidden files/directories starting with '.'
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(root) {
                    let rel_str = rel.to_string_lossy().replace("\\", "/");
                    if path.is_dir() {
                        // include folder paths with trailing slash to distinguish
                        paths.push(format!("{}/", rel_str));
                        Self::collect_relative_paths(root, &path, paths);
                    } else {
                        paths.push(rel_str);
                    }
                }
            }
        }
    }

    fn get_non_excluded_paths(&self) -> Vec<(usize, String)> {
        self.available_paths
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let clean = p.trim_end_matches('/');
                !self.excludes.iter().any(|e| e == clean || e == *p)
            })
            .map(|(i, p)| (i, p.clone()))
            .collect()
    }
}

impl Window for ExcludesEditorWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        if let Ok(mut lock) = self.paths_result.try_lock() {
            if let Some(p) = lock.take() {
                self.available_paths = p;
            }
        }

        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;

        new_window(ctx, self.id, t!("excludes_editor.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let _search_res = ui.add_sized(
                    [ui.available_width() - 30.0 * scale, 24.0 * scale],
                    egui::TextEdit::singleline(&mut self.search_term).hint_text(t!("search_filter"))
                );
                #[cfg(target_os = "android")]
                handle_android_keyboard(&_search_res, &mut self.search_term);

                if ui.button("\u{f00d}").clicked() {
                    self.search_term.clear();
                }
            });

            ui.separator();

            if self.available_paths.is_empty() && {
                let guard = self.paths_result.try_lock();
                guard.map_or(true, |l| l.is_none())
            } {
                ui.label(t!("loading_label"));
            } else {
                let non_excluded = self.get_non_excluded_paths();
                if !non_excluded.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(t!("add"));

                        let combo_items: Vec<(usize, &str)> = non_excluded
                            .iter()
                            .enumerate()
                            .map(|(i, (_, label))| (i, label.as_str()))
                            .collect();

                        let mut selected = self.add_selected.min(non_excluded.len() - 1);

                        let changed = Gui::run_combo_menu(
                            ui,
                            self.id.with("add_combo"),
                            &mut selected,
                            &combo_items,
                            &mut self.add_search_term,
                        );

                        if changed && selected < non_excluded.len() {
                            let (orig_idx, _) = non_excluded[selected];
                            if let Some(path) = self.available_paths.get(orig_idx) {
                                let path_to_add = path.trim_end_matches('/').to_string();
                                if !path_to_add.is_empty() && !self.excludes.contains(&path_to_add) {
                                    self.excludes.push(path_to_add);
                                }
                            }

                            self.add_search_term.clear();
                            self.add_selected = 0;
                        }
                    });
                } else {
                    ui.label(t!("excludes_editor.no_paths_available"));
                }
            }

            ui.separator();

            simple_window_layout(ui, self.id,
                |ui| {
                    egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .show(ui, |ui| {
                        let mut to_remove: Option<usize> = None;
                        let mut to_edit: Option<usize> = None;

                        let display_items: Vec<(usize, String)> = self.excludes
                            .iter()
                            .enumerate()
                            .filter(|(_, exclude)| {
                                self.search_term.is_empty()
                                    || exclude.to_lowercase().contains(&self.search_term.to_lowercase())
                            })
                            .map(|(i, exclude)| (i, exclude.clone()))
                            .collect();

                        for (i, exclude_str) in &display_items {
                            let i = *i;
                            
                            if let Some(ConfirmAction::Remove { index }) = self.confirm_action.as_ref() {
                                if *index == i {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                        if ui.button(t!("no")).clicked() {
                                            self.confirm_action = None;
                                        }

                                        if ui.button(t!("yes")).clicked() {
                                            to_remove = Some(i);
                                            self.confirm_action = None;
                                        }

                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            ui.label(t!("excludes_editor.confirm_remove", path = exclude_str.as_str()));
                                        });
                                    });
                                    continue;
                                }
                            }

                            if let Some(ConfirmAction::ConfirmEdit { index, .. }) = self.confirm_action.as_ref() {
                                if *index == i {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                        if ui.button(t!("no")).clicked() {
                                            self.confirm_action = None;
                                        }

                                        if ui.button(t!("yes")).clicked() {
                                            if let Some(ConfirmAction::ConfirmEdit { value, .. }) = self.confirm_action.take() {
                                                self.excludes[i] = value;
                                            }
                                            self.edit_index = None;
                                            self.confirm_action = None;
                                        }

                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            ui.label(t!("save_changes"));
                                        });
                                    });
                                    continue;
                                }
                            }

                            if self.edit_index == Some(i) {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    if ui.button(t!("cancel")).clicked() {
                                        self.edit_index = None;
                                    }

                                    if ui.button(t!("done")).clicked() {
                                        if !self.edit_value.is_empty() {
                                            self.confirm_action = Some(ConfirmAction::ConfirmEdit {
                                                index: i,
                                                value: self.edit_value.clone(),
                                            });
                                        }
                                    }

                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                        let _edit_res = ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_value)
                                                .desired_width(ui.available_width())
                                        );

                                        #[cfg(target_os = "android")]
                                        handle_android_keyboard(&_edit_res, &mut self.edit_value);
                                    });
                                });
                            } else {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    if ui.button(t!("remove")).clicked() {
                                        self.confirm_action = Some(ConfirmAction::Remove { index: i });
                                    }
                                    if ui.button(t!("edit")).clicked() {
                                        to_edit = Some(i);
                                    }

                                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                        ui.label(exclude_str.as_str());
                                    });
                                });
                            }
                        }

                        if let Some(idx) = to_remove {
                            self.excludes.remove(idx);
                            if self.edit_index == Some(idx) {
                                self.edit_index = None;
                            } else if let Some(edit_idx) = self.edit_index {
                                if edit_idx > idx {
                                    self.edit_index = Some(edit_idx - 1);
                                }
                            }
                        }

                        if let Some(idx) = to_edit {
                            self.edit_value = self.excludes[idx].clone();
                            self.edit_index = Some(idx);
                        }
                    });
                },
                |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button(t!("cancel")).clicked() {
                                open2 = false;
                            }
                            if ui.button(t!("save")).clicked() {
                                save_clicked = true;
                                open2 = false;
                            }
                        });
                    });
                }
            );
        });

        if save_clicked {
            match Self::save_excludes(&self.excludes) {
                Ok(()) => {
                    thread::spawn(|| {
                        Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_notification(&t!("excludes_editor.saved"));
                    });
                }
                Err(e) => {
                    let err = e.clone();
                    thread::spawn(move || {
                        Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_notification(&err);
                    });
                }
            }
        }

        open &= open2;
        open
    }
}

struct ChangeTranslationRepoWindow {
    id: egui::Id,
    confirm_remove: Option<(u32, String)>,
    repo_cache: HashMap<u32, (Option<LocalRepoInfo>, Option<String>)>,
    was_updating: bool,
}

impl ChangeTranslationRepoWindow {
    fn new() -> ChangeTranslationRepoWindow {
        let hachimi = Hachimi::instance();
        let manager = hachimi.tl_repo_manager.lock().unwrap();

        let repo_cache: HashMap<u32, (Option<LocalRepoInfo>, Option<String>)> = manager.repos.iter()
            .map(|repo| {
                let info = match LocalRepoInfo::load(repo.id) {
                    Ok(data) => data,
                    Err(e) => {
                        let err = e.to_string();
                        thread::spawn(move || {
                            Gui::instance().unwrap()
                                .lock().unwrap()
                                .show_notification(&err);
                        });
                        None
                    }
                };
                let icon_path = Hachimi::instance().get_repo_dir(repo.id).join("icon.png");
                let icon_uri = if icon_path.exists() {
                    Some(format!("file://{}", icon_path.display()))
                } else {
                    None
                };
                (repo.id, (info, icon_uri))
            })
            .collect();

        ChangeTranslationRepoWindow {
            id: random_id(),
            confirm_remove: None,
            repo_cache,
            was_updating: false,
        }
    }
}

impl Window for ChangeTranslationRepoWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;

        let hachimi = Hachimi::instance();
        let manager = hachimi.tl_repo_manager.lock().unwrap().clone();
        let current_repo_id = hachimi.config.load().selected_tl_repo_id;
        let has_repos = !manager.repos.is_empty();

        let completed_id = REMOVED_TLREPO_ID.load(atomic::Ordering::Relaxed);
        if completed_id != u32::MAX {
            self.repo_cache.remove(&completed_id);
            self.confirm_remove = None;
            REMOVED_TLREPO_ID.store(u32::MAX, atomic::Ordering::Relaxed);
        }

        let is_updating = hachimi.tl_updater.is_updating();
        if self.was_updating && !is_updating {
            for repo in &manager.repos {
                self.refresh_repo_cache_entry(repo.id);
            }
        }
        self.was_updating = is_updating;

        for repo in &manager.repos {
            if !self.repo_cache.contains_key(&repo.id) {
                self.refresh_repo_cache_entry(repo.id);
            }
        }

        new_window(ctx, self.id, t!("change_translation_repo.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            simple_window_layout(ui, self.id,
                |ui| {
                    if !has_repos {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0 * scale);
                            ui.label(t!("change_translation_repo.no_repos"));
                            ui.add_space(10.0 * scale);
                        });

                        ui.separator();
 
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.button(t!("change_translation_repo.browse_repositories")).clicked() {
                                thread::spawn(|| {
                                    Gui::instance().unwrap()
                                    .lock().unwrap()
                                    .show_window(Box::new(AddTranslationRepoWindow::new()));
                                });
                            }
                        });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.heading(t!("change_translation_repo.active"));
                            ui.separator();

                            for repo in &manager.repos {
                                let is_active = current_repo_id == Some(repo.id);
                                if !is_active { continue; }

                                let cached = self.repo_cache.get(&repo.id);
                                let info = cached.and_then(|(info, _)| info.as_ref());

                                if let Some((ref repo_id, _)) = self.confirm_remove {
                                    let matched_id = *repo_id;
                                    if matched_id == repo.id {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                            if ui.button(t!("ok")).clicked() {
                                                self.confirm_remove = None;
                                            }
                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                                ui.label(t!("change_translation_repo.cannot_remove_active"));
                                            });
                                        });
                                        continue;
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    if let Some(ref info) = info {
                                        if ui.button(t!("remove")).clicked() {
                                            self.confirm_remove = Some((repo.id, info.name.clone()));
                                        }
                                        if ui.button(" \u{f05a} ").clicked() {
                                            let repo_id = repo.id;
                                            let index = repo.index.clone();
                                            thread::spawn(move || {
                                                Gui::instance().unwrap()
                                                .lock().unwrap()
                                                .show_window(Box::new(TranslationRepoInfoWindow::new(repo_id, index)));
                                            });
                                        }
                                        let name_width = ui.available_width() - 48.0 * scale - ui.style().spacing.item_spacing.x;
                                        ui.allocate_ui_with_layout(egui::vec2(name_width, 0.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            ui.add(egui::RadioButton::new(true, ""));
                                            ui.label(&info.name);
                                        });
                                    } else {
                                        if ui.button(t!("remove")).clicked() {
                                            self.confirm_remove = Some((repo.id, repo.index.clone()));
                                        }
                                        let name_width = ui.available_width() - 48.0 * scale - ui.style().spacing.item_spacing.x;
                                        ui.allocate_ui_with_layout(egui::vec2(name_width, 0.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            ui.add(egui::RadioButton::new(true, ""));
                                            ui.label(&repo.index);
                                        });
                                    }
                                    ui.add(match cached.and_then(|(_, uri)| uri.as_ref()) {
                                        Some(uri) => egui::Image::new(uri.clone())
                                            .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale)),
                                        None => Gui::icon_2x(ctx),
                                    });
                                });
                            }

                            ui.add_space(8.0 * scale);
                            ui.heading(t!("change_translation_repo.available"));
                            ui.separator();

                            for repo in &manager.repos {
                                let is_active = current_repo_id == Some(repo.id);
                                if is_active { continue; }

                                let cached = self.repo_cache.get(&repo.id);
                                let info = cached.and_then(|(info, _)| info.as_ref());

                                if let Some((ref repo_id, ref repo_name)) = self.confirm_remove {
                                    if *repo_id == repo.id {
                                        let remove_id = *repo_id;
                                        let remove_name = repo_name.clone();
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                            if ui.button(t!("no")).clicked() {
                                                self.confirm_remove = None;
                                            }
                                            if ui.button(t!("yes")).clicked() {
                                                if !REMOVING_TLREPO.load(atomic::Ordering::Relaxed) {
                                                    REMOVING_TLREPO.store(true, atomic::Ordering::Relaxed);
                                                    Self::remove_repo_async(remove_id);
                                                    self.confirm_remove = None;
                                                } else {
                                                    request_notification(NotificationRequest::Custom(t!("change_translation_repo.remove_in_progress").to_string()));
                                                }
                                            }
                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                                ui.label(t!("change_translation_repo.confirm_remove", name = remove_name.as_str()));
                                            });
                                        });
                                        continue;
                                    }
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                    if let Some(ref info) = info {
                                        if ui.button(t!("remove")).clicked() {
                                            self.confirm_remove = Some((repo.id, info.name.clone()));
                                        }
                                        if ui.button("\u{f05a}").clicked() {
                                            let repo_id = repo.id;
                                            let index = repo.index.clone();
                                            thread::spawn(move || {
                                                Gui::instance().unwrap()
                                                .lock().unwrap()
                                                .show_window(Box::new(TranslationRepoInfoWindow::new(repo_id, index)));
                                            });
                                        }
                                        let name_width = ui.available_width() - 48.0 * scale - ui.style().spacing.item_spacing.x;
                                        let name_resp = ui.allocate_ui_with_layout(egui::vec2(name_width, 0.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            let radio = ui.add(egui::RadioButton::new(false, ""));
                                            ui.label(&info.name);
                                            radio.clicked()
                                        });
                                        ui.add(match cached.and_then(|(_, uri)| uri.as_ref()) {
                                            Some(uri) => egui::Image::new(uri.clone())
                                                .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale)),
                                            None => Gui::icon_2x(ctx),
                                        });
                                        if name_resp.inner {
                                            Self::switch_to_repo(repo.id, &repo.index);
                                        }
                                    } else {
                                        if ui.button(t!("remove")).clicked() {
                                            self.confirm_remove = Some((repo.id, repo.index.clone()));
                                        }
                                        let name_width = ui.available_width() - 48.0 * scale - ui.style().spacing.item_spacing.x;
                                        let name_resp = ui.allocate_ui_with_layout(egui::vec2(name_width, 0.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                            let radio = ui.add(egui::RadioButton::new(false, ""));
                                            ui.label(&repo.index);
                                            radio.clicked()
                                        });
                                        ui.add(match cached.and_then(|(_, uri)| uri.as_ref()) {
                                            Some(uri) => egui::Image::new(uri.clone())
                                                .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale)),
                                            None => Gui::icon_2x(ctx),
                                        });
                                        if name_resp.inner {
                                            Self::switch_to_repo(repo.id, &repo.index);
                                        }
                                    }
                                });
                            }
                        });
                    }
                },
                |ui| {
                    if ui.button(t!("cancel")).clicked() {
                        open2 = false;
                    }
                    if ui.button(t!("change_translation_repo.browse_repositories")).clicked() {
                        thread::spawn(|| {
                            Gui::instance().unwrap()
                            .lock().unwrap()
                            .show_window(Box::new(AddTranslationRepoWindow::new()));
                        });
                    }
                }
            );
        });

        open &= open2;
        open
    }
}

impl ChangeTranslationRepoWindow {
    fn refresh_repo_cache_entry(&mut self, repo_id: u32) {
        let info = match LocalRepoInfo::load(repo_id) {
            Ok(data) => data,
            Err(e) => {
                let err = e.to_string();
                thread::spawn(move || {
                    Gui::instance().unwrap()
                        .lock().unwrap()
                        .show_notification(&err);
                });
                None
            }
        };
        let icon_path = Hachimi::instance().get_repo_dir(repo_id).join("icon.png");
        let icon_uri = if icon_path.exists() {
            Some(format!("file://{}", icon_path.display()))
        } else {
            None
        };
        self.repo_cache.insert(repo_id, (info, icon_uri));
    }

    fn switch_to_repo(repo_id: u32, index: &str) {
        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let mut new_config = (**config).clone();
        new_config.selected_tl_repo_id = Some(repo_id);
        new_config.translation_repo_index = Some(index.to_string());
        drop(config);
        save_and_reload_config(new_config);
        hachimi.tl_updater.clone().check_for_updates(false, false);
    }

    fn remove_repo_async(repo_id: u32) {
        std::thread::spawn(move || {
            let notif_guard = if let Some(mutex) = Gui::instance() {
                let id = mutex.lock().unwrap().show_persistent_notification(
                    &t!("change_translation_repo.removing")
                );
                Some(NotificationGuard(id))
            } else {
                None
            };

            let hachimi = Hachimi::instance();
            let repo_dir = hachimi.get_repo_dir(repo_id);
            if repo_dir.is_dir() {
                let _ = std::fs::remove_dir_all(&repo_dir);
            }

            let cache_path = hachimi.get_data_path(format!(".tl_repo_cache_{}", repo_id));
            if cache_path.exists() {
                let _ = std::fs::remove_file(&cache_path);
            }

            let repos_path = hachimi.get_data_path(".tl_repos");
            {
                let mut manager = hachimi.tl_repo_manager.lock().unwrap();
                manager.repos.retain(|r| r.id != repo_id);
                if let Err(e) = manager.save(&repos_path) {
                    warn!("Failed to save .tl_repos after removal: {e}");
                }
            }

            let config = hachimi.config.load();
            if config.selected_tl_repo_id == Some(repo_id) {
                let mut new_config = (**config).clone();
                new_config.selected_tl_repo_id = None;
                new_config.translation_repo_index = None;
                drop(config);
                save_and_reload_config(new_config);
            }

            drop(notif_guard);
    
            REMOVED_TLREPO_ID.store(repo_id, atomic::Ordering::Relaxed);
            REMOVING_TLREPO.store(false, atomic::Ordering::Relaxed);
        });
    }
}

struct AddTranslationRepoWindow {
    id: egui::Id,
    index_request: Arc<AsyncRequest<Vec<RepoInfo>>>,
    config: hachimi::Config,
    current_tl_repo: Option<String>,
    has_auto_selected: bool,
    save_clicked: bool,
}

impl AddTranslationRepoWindow {
    fn new() -> AddTranslationRepoWindow {
        let config = (**Hachimi::instance().config.load()).clone();
        AddTranslationRepoWindow {
            id: random_id(),
            index_request: Arc::new(tl_repo::new_meta_index_request()),
            config,
            current_tl_repo: None,
            has_auto_selected: false,
            save_clicked: false,
        }
    }
}

impl Window for AddTranslationRepoWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;

        new_window(ctx, self.id, t!("add_translation_repo.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.heading(t!("add_translation_repo.select_translation_repo"));
            ui.add_space(4.0);

            let mut retry_clicked = false;

            tl_repo_list_ui(
                ui,
                &self.index_request,
                || retry_clicked = true,
                &mut self.current_tl_repo,
                &mut self.has_auto_selected,
                self.config.language.locale_str(),
                false,
                true
            );

            if retry_clicked {
                self.index_request = Arc::new(tl_repo::new_meta_index_request());
            }

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.button(t!("cancel")).clicked() {
                    open2 = false;
                }
                if ui.button(t!("save")).clicked() {
                    self.save_clicked = true;
                    open2 = false;
                }
            });
        });

        if self.save_clicked {
            if let Some(ref index) = self.current_tl_repo {
                let hachimi = Hachimi::instance();
                let mut manager = hachimi.tl_repo_manager.lock().unwrap();
                let repos_path = hachimi.get_data_path(".tl_repos");

                if let Some(existing_id) = manager.find_by_index(index) {
                    let config = hachimi.config.load();
                    let mut new_config = (**config).clone();
                    new_config.selected_tl_repo_id = Some(existing_id);
                    new_config.translation_repo_index = Some(index.clone());
                    drop(config);
                    drop(manager);
                    save_and_reload_config(new_config);
                } else {
                    let new_id = manager.add(index.clone());
                    if let Err(e) = manager.save(&repos_path) {
                        warn!("Failed to persist .tl_repos: {e}");
                    }

                    let config = hachimi.config.load();
                    let mut new_config = (**config).clone();
                    new_config.selected_tl_repo_id = Some(new_id);
                    new_config.translation_repo_index = Some(index.clone());
                    drop(config);
                    drop(manager);
                    save_and_reload_config(new_config);
                    hachimi.tl_updater.clone().check_for_updates(false, false);
                }
            }
        }

        open &= open2;
        open
    }
}

struct TranslationRepoInfoWindow {
    id: egui::Id,
    index_url: String,
    info: Option<LocalRepoInfo>,
    icon_uri: Option<String>,
    contributors_text: Option<String>,
    contributors_fetch_result: Arc<Mutex<Option<String>>>,
}

impl TranslationRepoInfoWindow {
    fn new(repo_id: u32, index_url: String) -> TranslationRepoInfoWindow {
        let info = match LocalRepoInfo::load(repo_id) {
            Ok(data) => data,
            Err(e) => {
                let err = e.to_string();
                thread::spawn(move || {
                    Gui::instance().unwrap()
                        .lock().unwrap()
                        .show_notification(&err);
                });
                None
            }
        };

        let icon_path = Hachimi::instance().get_repo_dir(repo_id).join("icon.png");
        let icon_uri = if icon_path.exists() {
            Some(format!("file://{}", icon_path.display()))
        } else {
            None
        };

        let contributors_text = info.as_ref().and_then(|i| i.format_contributors());

        let contributors_fetch_result = Arc::new(Mutex::new(None));
        if let Some(ref i) = info {
            if i.is_contributors_txt_url() {
                if let Some(url) = i.contributors.as_str() {
                    let fetch_result = contributors_fetch_result.clone();
                    let url = url.to_string();

                    std::thread::spawn(move || {
                        let agent = ureq::Agent::new_with_config(ureq_config());

                        if let Ok(res) = agent.get(&url).call() {
                            if let Ok(text) = res.into_body().read_to_string() {
                                let sanitized: String = text.chars()
                                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
                                    .collect();
                                let names: Vec<&str> = sanitized.lines()
                                    .map(|l| l.trim())
                                    .filter(|l| !l.is_empty())
                                    .collect();
                                let joined = names.join(", ");

                                *fetch_result.lock().unwrap() = Some(joined);
                            }
                        }
                    });
                }
            }
        }

        TranslationRepoInfoWindow {
            id: random_id(),
            index_url,
            info,
            icon_uri,
            contributors_text,
            contributors_fetch_result,
        }
    }
}

impl Window for TranslationRepoInfoWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;

        if self.contributors_text.is_none() {
            if let Ok(mut lock) = self.contributors_fetch_result.try_lock() {
                if let Some(text) = lock.take() {
                    self.contributors_text = Some(text);
                }
            }
        }

        new_window(ctx, self.id, t!("translation_repo_info.details_title"))
        .max_width(350.0 * scale)
        .max_height(440.0 * scale)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                if let Some(ref info) = self.info {
                    ui.horizontal(|ui| {
                        ui.add(match &self.icon_uri {
                            Some(uri) => egui::Image::new(uri.clone())
                                .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale)),
                            None => Gui::icon_2x(ctx)
                        });
                        ui.vertical(|ui| {
                            ui.heading(&info.name);
                            if !info.language.is_empty() {
                                ui.label(egui::RichText::new(&info.language).small().italics());
                            }
                        });
                    });

                    if !info.description.is_empty() {
                        ui.add_space(6.0 * scale);
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(&info.description);
                            });
                    }

                    if !info.homepage.is_empty() || !info.maintainer.is_empty() {
                        ui.add_space(6.0 * scale);
                        egui::Grid::new(self.id.with("info_grid"))
                            .num_columns(2)
                            .min_col_width(95.0 * scale)
                            .spacing([12.0 * scale, 6.0 * scale])
                            .show(ui, |ui| {
                                if !info.homepage.is_empty() {
                                    ui.label(egui::RichText::new(
                                        t!("translation_repo_info.homepage")
                                    ).strong());
                                    if ui.button(t!("open")).clicked() {
                                        Application::OpenURL(info.homepage.to_il2cpp_string());
                                    }
                                    ui.end_row();
                                }

                                if !info.maintainer.is_empty() {
                                    ui.label(egui::RichText::new(
                                        t!("translation_repo_info.maintainer")
                                    ).strong());
                                    ui.label(&info.maintainer);
                                    ui.end_row();
                                }
                            });
                    }

                    if !info.contributors.is_null() {
                        ui.add_space(6.0 * scale);
                        ui.separator();
                        ui.add_space(4.0 * scale);
                        ui.label(egui::RichText::new(t!("translation_repo_info.contributors")).strong());

                        if let Some(ref text) = self.contributors_text {
                            ui.label(text);
                        } else if info.is_contributors_txt_url() {
                            ui.label(egui::RichText::new(t!("loading_label")).italics());
                        } else if info.is_contributors_url() {
                            if let Some(url) = info.contributors.as_str() {
                                if ui.button(t!("translation_repo_info.view_contributors")).clicked() {
                                    Application::OpenURL(url.to_il2cpp_string());
                                }
                            }
                        }
                    }

                    if !info.links.is_empty() {
                        ui.add_space(6.0 * scale);
                        ui.separator();
                        ui.add_space(4.0 * scale);
                        ui.label(egui::RichText::new(t!("translation_repo_info.links")).strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0 * scale;
                            for link in &info.links {
                                if ui.button(&link[0]).clicked() {
                                    Application::OpenURL(link[1].to_il2cpp_string());
                                }
                            }
                        });
                    }
                } else {
                    ui.add(match &self.icon_uri {
                        Some(uri) => egui::Image::new(uri.clone())
                            .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale)),
                        None => Gui::icon_2x(ctx)
                    });
                    ui.add_space(8.0 * scale);
                    ui.label(&self.index_url);
                }
            });
        });

        open
    }
}

pub struct TranslationRepoUpdateWindow {
    title: String,
    content: String,
    changelog_is_markdown: bool,
    callback: Option<Box<dyn FnOnce(bool) + Send + Sync>>,
    id: egui::Id,
    changelog_fetch_result: Arc<Mutex<Option<Result<String, String>>>>,
    changelog_cached: Option<Result<String, String>>,
}

impl TranslationRepoUpdateWindow {
    pub fn new(title: &str, content: &str, changelog_url: &str, changelog_is_markdown: bool, callback: impl FnOnce(bool) + Send + Sync + 'static) -> TranslationRepoUpdateWindow {
        let fetch_result = Arc::new(Mutex::new(None));
        let fetch_result_clone = fetch_result.clone();
        let url = changelog_url.to_owned();
        let url_cloned = url.clone();

        std::thread::spawn(move || {
            let agent = ureq::Agent::new_with_config(ureq_config());
            let result = match agent.get(&url_cloned).call() {
                Ok(res) => {
                    match res.into_body().read_to_string() {
                        Ok(text) => {
                            if text.contains('\0') {
                                Err(t!("tl_update_dialog.changelog_invalid").into_owned())
                            } else {
                                Ok(text)
                            }
                        }
                        Err(e) => Err(format!("{}: {}", t!("tl_update_dialog.changelog_fetch_failed"), e))
                    }
                }
                Err(e) => Err(format!("{}: {}", t!("tl_update_dialog.changelog_fetch_failed"), e))
            };
            *fetch_result_clone.lock().unwrap() = Some(result);
        });

        TranslationRepoUpdateWindow {
            title: title.to_owned(),
            content: content.to_owned(),
            changelog_is_markdown,
            callback: Some(Box::new(callback)),
            id: random_id(),
            changelog_fetch_result: fetch_result,
            changelog_cached: None,
        }
    }
}

impl Window for TranslationRepoUpdateWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;
        let mut result = false;

        // check if changelog fetch completed, cache the result
        if self.changelog_cached.is_none() {
            if let Ok(mut lock) = self.changelog_fetch_result.try_lock() {
                if let Some(fetch_result) = lock.take() {
                    // sanitize plaintext: strip control chars except newline/tab
                    let processed = match fetch_result {
                        Ok(content) => {
                            if !self.changelog_is_markdown {
                                Ok(content.chars()
                                    .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r')
                                    .collect())
                            } else {
                                Ok(content)
                            }
                        }
                        Err(e) => Err(e),
                    };
                    self.changelog_cached = Some(processed);
                }
            }
        }

        new_window(ctx, self.id, &self.title)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom(self.id.with("bottom_panel"))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    // changelog button: loading > show changelog > error
                    if self.changelog_cached.is_none() {
                        ui.label(egui::RichText::new(t!("loading_label")).italics());
                    } else if let Some(Ok(_)) = &self.changelog_cached {
                        if ui.button(t!("tl_update_dialog.show_changelog")).clicked() {
                            let content = self.changelog_cached.as_ref().unwrap().as_ref().unwrap().clone();
                            // .md / .markdown
                            if self.changelog_is_markdown {
                                thread::spawn(move || {
                                    Gui::instance().unwrap()
                                    .lock().unwrap()
                                    .show_window(Box::new(SimpleMarkdownDialog::new(
                                        &t!("tl_update_dialog.changelog_title"),
                                        &content,
                                    )));
                                });
                            } else {
                                // plaintext
                                thread::spawn(move || {
                                    Gui::instance().unwrap()
                                    .lock().unwrap()
                                    .show_window(Box::new(SimpleOkDialog::new(
                                        &t!("tl_update_dialog.changelog_title"),
                                        &content,
                                        true,
                                        || {}
                                    )));
                                });
                            }
                        }
                    } else if let Some(Err(msg)) = &self.changelog_cached {
                        ui.label(egui::RichText::new(msg).color(ui.visuals().error_fg_color));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button(t!("no")).clicked() {
                            open2 = false;
                        }
                        if ui.button(t!("yes")).clicked() {
                            result = true;
                            open2 = false;
                        }
                    });
                });
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                centered_and_wrapped_text(ui, &self.content);
            });
        });

        if open && open2 {
            true
        }
        else {
            if let Some(cb) = self.callback.take() {
                cb(result);
            }
            false
        }
    }
}

pub struct SimpleMarkdownDialog {
    title: String,
    content: String,
    id: egui::Id,
    cache: egui_commonmark::CommonMarkCache,
}

impl SimpleMarkdownDialog {
    pub fn new(title: &str, content: &str) -> SimpleMarkdownDialog {
        SimpleMarkdownDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            id: random_id(),
            cache: egui_commonmark::CommonMarkCache::default(),
        }
    }
}

impl Window for SimpleMarkdownDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;

        new_window(ctx, self.id, &self.title)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom(self.id.with("bottom_panel"))
            .show_inside(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button(t!("ok")).clicked() {
                        open2 = false;
                    }
                })
            });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui_commonmark::CommonMarkViewer::new()
                            .show(ui, &mut self.cache, &self.content);
                    });
                });
        });

        open && open2
    }
}

struct AboutWindow {
    id: egui::Id
}

impl AboutWindow {
    fn new() -> AboutWindow {
        AboutWindow {
            id: random_id()
        }
    }
}

impl Window for AboutWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;

        new_window(ctx, self.id, t!("about.title"))
        .max_width(310.0 * scale)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(Gui::icon_2x(ctx));
                ui.vertical(|ui| {
                    ui.heading(t!("hachimi"));
                    ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                });
            });
            ui.label(t!("about.copyright", year = Utc::now().year()));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                if ui.button(t!("about.view_license")).clicked() {
                    thread::spawn(|| {
                        Gui::instance().unwrap()
                        .lock().unwrap()
                        .show_window(Box::new(LicenseWindow::new()));
                    });
                }
                ui.end_row();

                if ui.button(t!("about.open_website")).clicked() {
                    Application::OpenURL(WEBSITE_URL.to_il2cpp_string());
                }

                if ui.button(t!("about.view_source_code")).clicked() {
                    Application::OpenURL(format!("https://github.com/{}", REPO_PATH).to_il2cpp_string());
                }

                if ui.button(t!("about.view_contributors")).clicked() {
                    Application::OpenURL(format!("https://github.com/{}/graphs/contributors?all=1", REPO_PATH).to_il2cpp_string());
                }
            });
        });

        open
    }
}

struct LicenseWindow {
    id: egui::Id
}

impl LicenseWindow {
    fn new() -> LicenseWindow {
        LicenseWindow {
            id: random_id()
        }
    }
}

impl Window for LicenseWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;

        new_window(ctx, self.id, t!("license.title"))
        .open(&mut open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.heading(t!("hachimi"));
                ui.collapsing(t!("license.gpl_v3_only_notice"), |ui| {
                    ui.add(egui::TextEdit::multiline(&mut include_str!("../../LICENSE"))
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .interactive(false)
                    );
                });
                ui.separator();

                ui.heading("Open Font Licenses (OFL)");
                ui.label(t!("license.ofl_fonts_header"));
                ui.group(|ui| {
                    ui.label(t!("license.font_inter"));
                    ui.label(t!("license.font_font_awesome"));
                });

                ui.add_space(4.0);
                ui.collapsing(t!("license.ofl_notice"), |ui| {
                    ui.add(egui::TextEdit::multiline(&mut include_str!("../../assets/fonts/OFL.txt"))
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(10)
                        .interactive(false)
                    );
                });

                ui.add_space(10.0);
                ui.separator();

                ui.heading(t!("license.font_alibaba_header"));
                ui.label(t!("license.font_alibaba_body"));
            });
        });

        open
    }
}

pub struct PersistentMessageWindow {
    id: egui::Id,
    title: String,
    content: String,
    show: Arc<AtomicBool>
}

impl PersistentMessageWindow {
    pub fn new(title: &str, content: &str, show: Arc<AtomicBool>) -> PersistentMessageWindow {
        PersistentMessageWindow {
            id: random_id(),
            title: title.to_owned(),
            content: content.to_owned(),
            show
        }
    }
}

impl Window for PersistentMessageWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        new_window(ctx, self.id, &self.title)
        .show(ctx, |ui| {
            simple_window_layout(ui, self.id,
                |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(&self.content);
                    });
                },
                |_| {
                }
            );
        });

        self.show.load(atomic::Ordering::Relaxed)
    }
}
