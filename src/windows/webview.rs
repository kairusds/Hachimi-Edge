use crate::{
    core::{
        game::Region,
        gui::{self, Window},
        Gui, Hachimi,
    },
    il2cpp::{ext::Il2CppStringExt, types::*},
    windows::wnd_hook::get_target_hwnd,
};
use egui::Context;
use once_cell::sync::Lazy;
use rust_i18n::t;
use std::{
    collections::HashMap,
    ffi::c_uint,
    sync::{mpsc, Mutex, RwLock},
};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::{
        CreateCoreWebView2EnvironmentWithOptions, GetAvailableCoreWebView2BrowserVersionString,
        ICoreWebView2, ICoreWebView2Controller, ICoreWebView2Environment,
        ICoreWebView2Environment10, ICoreWebView2EnvironmentOptions,
        COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC,
    },
    CoreWebView2EnvironmentOptions, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, ExecuteScriptCompletedHandler, Result,
};
use windows::{
    core::{w, Interface, HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_POINTER, E_UNEXPECTED, HWND, LPARAM, RECT, TRUE},
        Globalization::{
            GetUserDefaultUILanguage, LCIDToLocaleName, LOCALE_ALLOW_NEUTRAL_NAMES, MAX_LOCALE_NAME,
        },
        Graphics::Gdi::{InvalidateRect, UpdateWindow},
        System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{PostMessageW, SW_SHOWNORMAL, WM_USER},
        },
    },
};

pub const WM_OPEN_WEBVIEW: u32 = WM_USER + 500;
pub const WM_CLOSE_WEBVIEW: u32 = WM_USER + 501;
pub const WM_SET_WEBVIEW_POSITION: u32 = WM_USER + 502;
pub const WM_WEBVIEW_GOBACK: u32 = WM_USER + 503;

static HAS_WEBVIEW: Lazy<bool> = Lazy::new(|| unsafe {
    let mut version_info = PWSTR::null();
    if GetAvailableCoreWebView2BrowserVersionString(None, &mut version_info).is_ok() {
        let available = !version_info.is_null() && version_info.to_string().is_ok();
        if !version_info.is_null() {
            CoTaskMemFree(Some(version_info.as_ptr() as *const _));
        }
        available
    } else {
        false
    }
});

const RPC_E_CHANGED_MODE: u32 = 0x80010111;
static COM_INITIALIZED: Lazy<bool> = Lazy::new(|| {
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        true
    } else if hr.0 == RPC_E_CHANGED_MODE as i32 {
        warn!("COM already initialized with different apartment type. WebView2 callbacks may not dispatch correctly on this thread.");
        false
    } else {
        warn!("COM initialization failed: {:?}", hr);
        false
    }
});

fn ensure_com_initialized() -> bool {
    *COM_INITIALIZED
}

struct DialogWebView {
    parent: HWND,
    webview: Option<InnerWebView>,
    pending_bounds: Option<RECT>,
}

impl DialogWebView {
    pub fn new(parent: HWND) -> DialogWebView {
        DialogWebView {
            parent,
            webview: None,
            pending_bounds: None,
        }
    }

    fn set_position(&mut self, position: RECT) {
        if let Some(ref webview) = self.webview {
            match webview.set_bounds(position) {
                Ok(_) => {}
                Err(e) => warn!("set_bound error: {:?}", e),
            }
        }
    }

    fn back(&self) {
        if let Some(ref webview) = self.webview {
            let _ = webview.execute_script("window.history.back()");
        }
    }
}

struct InnerWebView {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
}

impl InnerWebView {
    pub fn new(parent: HWND, url: &str) -> Result<InnerWebView> {
        if !ensure_com_initialized() {
            return Err(webview2_com::Error::WindowsError(windows::core::Error::from(E_UNEXPECTED)));
        }

        let env = Self::create_environment()?;
        let controller = Self::create_controller(parent, &env)?;
        let webview = Self::init_webview(&controller, url)?;

        Ok(InnerWebView { controller, webview })
    }

    fn set_bounds(&self, bounds: RECT) -> Result<()> {
        unsafe { Ok(self.controller.SetBounds(bounds)?) }
    }

    fn execute_script(&self, js: &str) -> windows::core::Result<()> {
        unsafe {
            let js = HSTRING::from(js);
            self.webview.ExecuteScript(
                &js,
                &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
            )
        }
    }

    fn create_environment() -> Result<ICoreWebView2Environment> {
        let options = CoreWebView2EnvironmentOptions::default();
        let (tx, rx) = mpsc::channel();

        unsafe {
            options.set_are_browser_extensions_enabled(false);
            let lcid = GetUserDefaultUILanguage();
            let mut lang = [0; MAX_LOCALE_NAME as usize];
            LCIDToLocaleName(lcid as u32, Some(&mut lang), LOCALE_ALLOW_NEUTRAL_NAMES);
            options.set_language(String::from_utf16_lossy(&lang));

            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                &HSTRING::default(),
                &ICoreWebView2EnvironmentOptions::from(options),
                &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                    move |error_code, environment| {
                        let result: Result<ICoreWebView2Environment> = (|| {
                            error_code?;
                            environment.ok_or_else(|| windows::core::Error::from(E_POINTER).into())
                        })();
                        tx.send(result)
                            .map_err(|_| windows::core::Error::from(E_UNEXPECTED))
                    },
                )),
            )?;
        }

        webview2_com::wait_with_pump(rx)?
    }

    fn create_controller(
        hwnd: HWND,
        env: &ICoreWebView2Environment,
    ) -> Result<ICoreWebView2Controller> {
        let (tx, rx) = mpsc::channel();

        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |error_code, controller| {
                let result: Result<ICoreWebView2Controller> = (|| {
                    error_code?;
                    controller.ok_or_else(|| windows::core::Error::from(E_POINTER).into())
                })();
                tx.send(result)
                    .map_err(|_| windows::core::Error::from(E_UNEXPECTED))
            },
        ));

        unsafe {
            if let Ok(env10) = env.cast::<ICoreWebView2Environment10>() {
                let controller_opts = env10.CreateCoreWebView2ControllerOptions()?;
                controller_opts.SetIsInPrivateModeEnabled(false)?;
                env10.CreateCoreWebView2ControllerWithOptions(hwnd, &controller_opts, &handler)?;
            } else {
                env.CreateCoreWebView2Controller(hwnd, &handler)?;
            }
        }

        webview2_com::wait_with_pump(rx)?
    }

    fn init_webview(controller: &ICoreWebView2Controller, url: &str) -> Result<ICoreWebView2> {
        let webview = unsafe { controller.CoreWebView2()? };

        let h_url = HSTRING::from(url);
        unsafe {
            webview.Navigate(&h_url)?;
            controller.SetIsVisible(true)?;
            controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC)?;
        }

        Ok(webview)
    }
}

impl Drop for InnerWebView {
    fn drop(&mut self) {
        let _ = unsafe { self.controller.Close() };
    }
}

unsafe impl Send for InnerWebView {}
unsafe impl Sync for InnerWebView {}

unsafe impl Send for DialogWebView {}
unsafe impl Sync for DialogWebView {}

static DIALOG_WEBVIEW: Lazy<Mutex<DialogWebView>> = Lazy::new(|| Mutex::new(DialogWebView::new(get_target_hwnd())));

pub fn process_message(umsg: c_uint, lparam: LPARAM) {
    let mut dialog = match DIALOG_WEBVIEW.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    match umsg {
        WM_OPEN_WEBVIEW => {
            let url = unsafe { Box::from_raw(lparam.0 as *mut (String, String, String)) };
            info!("Received request to open webview with URL: {:?}", url);

            let old_webview = dialog.webview.take();
            dialog.pending_bounds = None;

            drop(dialog);
            drop(old_webview);

            let webview_result = InnerWebView::new(get_target_hwnd(), &url.0);

            let mut dialog = DIALOG_WEBVIEW.lock().unwrap();
            match webview_result {
                Ok(webview) => {
                    dialog.webview = Some(webview);

                    if let Some(bounds) = dialog.pending_bounds.take() {
                        dialog.set_position(bounds);
                    }

                    open_dialog(url.1.to_string(), url.2.to_string());
                }
                Err(e) => warn!("Failed to create WebView2: {:?}", e),
            }
        }
        WM_SET_WEBVIEW_POSITION => {
            if let Ok(ref rect) = WEBVIEW_RECT.read() {
                if let Some(rect) = rect.as_ref() {
                    dialog.set_position(*rect);
                }
            }
        }
        WM_CLOSE_WEBVIEW => {
            let webview = dialog.webview.take();
            dialog.pending_bounds = None;
            let parent = dialog.parent;

            drop(dialog);
            drop(webview);

            unsafe {
                if InvalidateRect(Some(parent), None, true) == TRUE {
                    let _ = UpdateWindow(parent);
                }
            }
        }
        WM_WEBVIEW_GOBACK => dialog.back(),
        _ => {}
    }
}

const URL_HANDLER: &[fn(&str, &str, &HashMap<&str, &str>) -> Option<(String, String)>] = &[news_url, general_url];
const BASE_API_URL: &str = "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/";
static GACHA_URL_ID_MAP: Lazy<RwLock<HashMap<String, i32>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn add_gacha_url(url: *mut Il2CppString, gacha_id: i32) {
    let url_string = unsafe { (*url).as_utf16str().to_string() };
    if let Ok(mut map) = GACHA_URL_ID_MAP.write() {
        map.insert(url_string, gacha_id);
    }
}

fn gacha_url(url: &str, params: &HashMap<&str, &str>) -> Option<String> {
    let map = GACHA_URL_ID_MAP.read().ok()?;
    let gacha_id = map.get(url)?;
    let v = params.get("v")?;
    let r = params.get("r")?;
    let p = params.get("p")?;
    Some(format!("{BASE_API_URL}gacha?v={}&r={}&g={}&p={}", v, r, gacha_id, p))
}

fn news_url(
    _url: &str,
    base_url: &str,
    _params: &HashMap<&str, &str>,
) -> Option<(String, String)> {
    if base_url == "https://dmg.umamusume.jp/news" {
        return Some((
            "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/info?p=2&c=0".to_string(),
            "Notice".to_string(),
        ));
    }
    None
}

fn general_url(
    url: &str,
    base_url: &str,
    params: &HashMap<&str, &str>,
) -> Option<(String, String)> {
    let (api_url, title_key) = if base_url.starts_with("https://www.games.umamusume.jp/#/") {
        let url_type = if url.contains('?') {
            &base_url[33..]
        } else {
            "general"
        };

        let parsed_url = match url_type {
            "gacha" => gacha_url(url, params)?,
            _ => url.replacen("https://www.games.umamusume.jp/#/", BASE_API_URL, 1),
        };

        (parsed_url, format!("ingame_webview_dialog.title.{url_type}"))
    } else if url.starts_with("http://") || url.starts_with("https://") {
        (url.to_string(), "ingame_webview_dialog.title.general".to_string())
    } else {
        return None;
    };

    let title = crate::_rust_i18n_try_translate(&rust_i18n::locale(), &title_key)
        .unwrap_or(t!("ingame_webview_dialog.title.general"));

    Some((api_url, title.to_string()))
}

fn parse_url(url: &str) -> (&str, HashMap<&str, &str>) {
    if let Some(pos) = url.find('?') {
        (
            &url[..pos],
            url[pos + 1..]
                .split('&')
                .filter_map(|p| p.split_once('='))
                .collect(),
        )
    } else {
        (url, HashMap::new())
    }
}

pub fn open(url: *mut Il2CppString) -> bool {
    if Hachimi::instance().game.region != Region::Japan
        || !has_available_webview()
        || !Hachimi::instance()
            .config
            .load()
            .windows
            .ingame_webview
    {
        return false;
    }

    let url_string = unsafe { (*url).as_utf16str().to_string() };

    let (base_url, params) = parse_url(&url_string);
    for handler in URL_HANDLER {
        if let Some((api_url, title)) = handler(&url_string, &base_url, &params) {
            unsafe {
                let _ = PostMessageW(
                    Some(get_target_hwnd()),
                    WM_OPEN_WEBVIEW,
                    Default::default(),
                    LPARAM(Box::into_raw(Box::new((api_url, title, url_string))) as isize),
                );
            }
            return true;
        }
    }
    false
}

struct WebviewDialog {
    id: egui::Id,
    title: String,
    orig_url: String,
}

impl WebviewDialog {
    pub fn new(title: String, orig_url: String) -> WebviewDialog {
        WebviewDialog {
            id: egui::Id::new(egui::epaint::ahash::RandomState::new().hash_one(0)),
            title,
            orig_url,
        }
    }
}

static WEBVIEW_RECT: RwLock<Option<RECT>> = RwLock::new(None);

impl Window for WebviewDialog {
    fn run(&mut self, ctx: &Context) -> bool {
        let mut open = true;
        let mut open1 = true;
        let view_rect = ctx.viewport_rect();
        let view_width = view_rect.width();
        let view_height = view_rect.height();

        let gui_scale = gui::get_scale(ctx);

        let target_height = view_height * gui_scale;
        let mut target_width = view_width * gui_scale;
        if target_width > 320f32 {
            target_width = 320f32;
        }

        let resp = egui::Window::new(self.title.clone())
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(ctx.viewport_rect().max / 2.0)
            .fixed_size([target_width, target_height])
            .collapsible(false)
            .open(&mut open)
            .resizable(false)
            .id(self.id)
            .show(ctx, |ui| {
                let mut size = ui.available_size();
                size.y -= 60f32;
                ui.allocate_space(size);
                ui.add_space(4.0);
                unsafe {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.button(t!("back")).clicked() {
                            let _ = PostMessageW(
                                Some(get_target_hwnd()),
                                WM_WEBVIEW_GOBACK,
                                Default::default(),
                                Default::default(),
                            );
                        }
                        if ui
                            .button(t!("ingame_webview_dialog.open_browser"))
                            .clicked()
                        {
                            let url_hstring = HSTRING::from(&self.orig_url);
                            ShellExecuteW(
                                None,
                                w!("open"),
                                PCWSTR(url_hstring.as_ptr()),
                                None,
                                None,
                                SW_SHOWNORMAL,
                            );
                            open1 = false;
                        }
                    });
                }
                let mut max_rect = ui.max_rect();
                max_rect.set_height(max_rect.height() - 60f32);
                max_rect
            });

        if let Some(resp) = resp {
            if let Some(inner_rect) = resp.inner {
                unsafe {
                    let scale = ctx.pixels_per_point();
                    let rect = RECT {
                        left:  (inner_rect.min.x * scale) as i32,
                        top:   (inner_rect.min.y * scale) as i32,
                        right: (inner_rect.max.x * scale) as i32,
                        bottom:(inner_rect.max.y * scale) as i32,
                    };

                    if let Ok(mut lock) = WEBVIEW_RECT.write() {
                        let changed = match &*lock {
                            Some(existing) => existing != &rect,
                            None => true,
                        };
                        if changed {
                            *lock = Some(rect);
                            let _ = PostMessageW(
                                Some(get_target_hwnd()),
                                WM_SET_WEBVIEW_POSITION,
                                Default::default(),
                                Default::default(),
                            );
                        }
                    }
                }
            }
        }

        open &= open1;

        if !open {
            unsafe {
                *WEBVIEW_RECT.write().unwrap() = None;
                let _ = PostMessageW(
                    Some(get_target_hwnd()),
                    WM_CLOSE_WEBVIEW,
                    Default::default(),
                    Default::default(),
                );
            }
        }
        open
    }
}

fn open_dialog(title: String, orig_url: String) {
    Gui::instance()
        .unwrap()
        .lock()
        .unwrap()
        .show_window(Box::new(WebviewDialog::new(title, orig_url)));
}

pub fn has_available_webview() -> bool {
    *HAS_WEBVIEW
}
