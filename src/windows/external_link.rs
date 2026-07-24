use crate::core::game::Region;
use crate::core::gui::Window;
use crate::core::Hachimi;
use crate::core::{gui, Gui};
use crate::il2cpp::types::Il2CppString;
use crate::windows::wnd_hook::get_target_hwnd;
use egui::Context;
use once_cell::sync::Lazy;
use rust_i18n::t;
use std::collections::HashMap;
use std::ffi::c_uint;
use std::sync::{mpsc, RwLock};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    CreateCoreWebView2EnvironmentWithOptions, GetAvailableCoreWebView2BrowserVersionString,
    ICoreWebView2, ICoreWebView2Controller, ICoreWebView2Environment, ICoreWebView2Environment10,
    ICoreWebView2EnvironmentOptions, COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC,
};
use webview2_com::{
    CoreWebView2EnvironmentOptions, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, ExecuteScriptCompletedHandler, Result,
};
use windows::core::{w, Interface, HSTRING};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{E_POINTER, E_UNEXPECTED, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Globalization::{
    GetUserDefaultUILanguage, LCIDToLocaleName, LOCALE_ALLOW_NEUTRAL_NAMES, MAX_LOCALE_NAME,
};
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, SW_SHOWNORMAL, WM_USER};

pub const WM_OPEN_WEBVIEW: u32 = WM_USER + 500;
pub const WM_CLOSE_WEBVIEW: u32 = WM_USER + 501;
pub const WM_SET_WEBVIEW_POSITION: u32 = WM_USER + 502;
pub const WM_WEBVIEW_GOBACK: u32 = WM_USER + 503;

struct DialogWebView {
    parent: HWND,
    webview: Option<InnerWebView>,
}

impl DialogWebView {
    pub fn new(parent: HWND) -> DialogWebView {
        DialogWebView {
            parent,
            webview: None,
        }
    }

    fn set_position(&self, position: RECT) {
        if let Some(ref webview) = self.webview {
            match webview.set_bounds(position) {
                Ok(_) => {}
                Err(e) => warn!("set_bound error：{:?}", e),
            }
        }
    }
    fn close(&mut self) {
        unsafe {
            self.webview = None;
            if InvalidateRect(Some(self.parent), None, true) == TRUE {
                let _ = UpdateWindow(self.parent);
            }
        }
    }

    fn back(&self) {
        if let Some(ref webview) = self.webview {
            let _ = webview.execute_script(&"window.history.back()".to_string());
        }
    }

    fn open(&mut self, url: &String, title: &String, orig_url: &String) {
        if let Ok(webview) = InnerWebView::new(get_target_hwnd(), url) {
            self.webview = Some(webview);
            open_dialog(title.clone(), orig_url.clone());
        }
    }
}

struct InnerWebView {
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
}
impl InnerWebView {
    pub fn new(parent: HWND, url: &String) -> Result<InnerWebView> {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        let env = Self::create_environment()?;
        let controller = Self::create_controller(parent, &env)?;
        let webview = Self::init_webview(&controller, url)?;
        let w = Self {
            controller,
            webview,
        };
        Ok(w)
    }

    fn set_bounds(&self, bounds: RECT) -> Result<()> {
        unsafe { Ok(self.controller.SetBounds(bounds)?) }
    }

    fn execute_script(&self, js: &String) -> windows::core::Result<()> {
        unsafe {
            let js = HSTRING::from(js);
            self.webview.ExecuteScript(
                &js,
                &ExecuteScriptCompletedHandler::create(Box::new(|_, _| Ok(()))),
            )
        }
    }
    fn create_controller(
        hwnd: HWND,
        env: &ICoreWebView2Environment,
    ) -> Result<ICoreWebView2Controller> {
        let (tx, rx) = mpsc::channel();

        let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
            move |error_code, controller| {
                let result = (|| {
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
                env.CreateCoreWebView2Controller(hwnd, &handler)?
            }
        }

        webview2_com::wait_with_pump(rx)?
    }

    #[inline]
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
                        let result = (|| {
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

    #[inline]
    fn init_webview(controller: &ICoreWebView2Controller, url: &String) -> Result<ICoreWebView2> {
        let webview = unsafe { controller.CoreWebView2()? };

        let h_url = HSTRING::from(url);
        unsafe {
            webview.Navigate(&h_url)?;
        }

        unsafe {
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
static mut DIALOG_WEBVIEW: Lazy<DialogWebView> =
    Lazy::new(|| DialogWebView::new(get_target_hwnd()));

pub fn process_message(umsg: c_uint, lparam: LPARAM) {
    unsafe {
        match umsg {
            WM_OPEN_WEBVIEW => {
                let url = Box::from_raw(lparam.0 as *mut (String, String, String));
                info!("Received request to open webview with URL: {:?}", url);
                DIALOG_WEBVIEW.open(&url.0, &url.1, &url.2);
            }
            WM_SET_WEBVIEW_POSITION => {
                if let Ok(ref rect) = WEBVIEW_RECT.read() {
                    DIALOG_WEBVIEW.set_position(rect.unwrap());
                }
            }
            WM_CLOSE_WEBVIEW => DIALOG_WEBVIEW.close(),
            WM_WEBVIEW_GOBACK => DIALOG_WEBVIEW.back(),
            _ => {}
        }
    }
}

const URL_HANDLER: &[fn(&String, &String, &HashMap<String, String>) -> Option<(String, String)>] =
    &[news_url, general_url];

static mut GACHA_URL_ID_MAP: Lazy<HashMap<String, i32>> = Lazy::new(|| HashMap::new());
fn news_url(
    _url: &String,
    base_url: &String,
    _params: &HashMap<String, String>,
) -> Option<(String, String)> {
    if base_url == "https://dmg.umamusume.jp/news" {
        let api_url =
            "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/info?p=2&c=0"
                .to_string();
        return Some((api_url.to_string(), "Notice".to_string()));
    }
    None
}

pub fn add_gacha_url(url: *mut Il2CppString, gacha_id: i32) {
    let url_string = il2cppstring_as_string(unsafe { &*url });
    unsafe {
        GACHA_URL_ID_MAP.insert(url_string, gacha_id);
    }
}
const BASE_API_URL: &str = "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/";
fn gacha_url(url: &String, params: &HashMap<String, String>) -> Option<String> {
    let gacha_id = unsafe { GACHA_URL_ID_MAP.get(url)? };
    let v = params.get("v")?;
    let r = params.get("r")?;
    let p = params.get("p")?;
    let api_url = format!("{BASE_API_URL}gacha?v={}&r={}&g={}&p={}", v, r, gacha_id, p);
    Some(api_url)
}

fn general_url(
    url: &String,
    base_url: &String,
    params: &HashMap<String, String>,
) -> Option<(String, String)> {
    let mut url_type: String = "general".to_string();
    if base_url.starts_with("https://www.games.umamusume.jp/#/") {
        if let Some(pos) = url.find('?') {
            url_type = base_url[33..pos].to_string();
        }
        let translated_title_key = format!("external_link_dialog.title.{url_type}");
        let title = crate::_rust_i18n_try_translate(&rust_i18n::locale(), &translated_title_key)
            .unwrap_or(t!("external_link_dialog.title.general"));
        let api_url: Option<String> = match url_type.as_str() {
            "gacha" => gacha_url(url, params),
            _ => Some(url.replacen("https://www.games.umamusume.jp/#/", BASE_API_URL, 1)),
        };
        if api_url.is_none() {
            return None;
        }
        return Some((api_url.unwrap(), title.to_string()));
    }
    None
}
fn il2cppstring_as_string(string: &Il2CppString) -> String {
    let slice =
        unsafe { std::slice::from_raw_parts(string.chars.as_ptr(), string.length as usize) };
    String::from_utf16_lossy(slice)
}
fn parse_url(url: &String) -> (String, HashMap<String, String>) {
    if let Some(pos) = url.find('?') {
        (
            url[..pos].to_string(),
            url[pos + 1..]
                .split('&')
                .filter_map(|p| {
                    p.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                })
                .collect::<HashMap<String, String>>(),
        )
    } else {
        (url.clone(), HashMap::new())
    }
}
pub fn open(url: *mut Il2CppString) -> bool {
    if Hachimi::instance().game.region != Region::Japan
        || !has_available_webview()
        || !Hachimi::instance()
            .config
            .load()
            .windows
            .open_external_link_ingame
    {
        return false;
    }

    let url_string = il2cppstring_as_string(unsafe { &*url });

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

struct ExternalLinkDialog {
    id: egui::Id,
    title: String,
    orig_url: String,
}

impl ExternalLinkDialog {
    pub fn new(title: String, orig_url: String) -> ExternalLinkDialog {
        ExternalLinkDialog {
            id: egui::Id::new(egui::epaint::ahash::RandomState::new().hash_one(0)),
            title,
            orig_url,
        }
    }
}

static WEBVIEW_RECT: RwLock<Option<RECT>> = RwLock::new(None);

impl Window for ExternalLinkDialog {
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
                            .button(t!("external_link_dialog.open_original_link"))
                            .clicked()
                        {
                            ShellExecuteW(
                                None,
                                w!("open"),
                                PCWSTR(
                                    widestring::U16CString::from_str(self.orig_url.clone())
                                        .unwrap()
                                        .as_ptr(),
                                ),
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
                    let bottom = inner_rect.max.y * scale;
                    let left = inner_rect.min.x * scale;
                    let top = inner_rect.min.y * scale;
                    let right = inner_rect.max.x * scale;
                    let rect = RECT {
                        left: left as i32,
                        top: top as i32,
                        right: right as i32,
                        bottom: bottom as i32,
                    };

                    if let Ok(lock) = WEBVIEW_RECT.read() {
                        if lock.is_none() || lock.as_ref().unwrap() != &rect {
                            drop(lock);
                            *WEBVIEW_RECT.write().unwrap() = Some(rect);
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
        .show_window(Box::new(ExternalLinkDialog::new(title, orig_url)));
}
pub fn has_available_webview() -> bool {
    let mut version_info = PWSTR::null();
    unsafe {
        match GetAvailableCoreWebView2BrowserVersionString(None, &mut version_info) {
            Ok(_) => {
                let mut available = false;
                if !version_info.is_null() {
                    if version_info.to_string().is_ok() {
                        available = true
                    }
                    windows::Win32::System::Com::CoTaskMemFree(Some(
                        version_info.as_ptr() as *const _
                    ));
                }
                available
            }
            Err(_) => false,
        }
    }
}
