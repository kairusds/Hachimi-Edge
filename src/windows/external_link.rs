use crate::windows::wnd_hook::get_target_hwnd;
use once_cell::sync::Lazy;
use std::ffi::c_uint;
use std::num::NonZeroIsize;
use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, TRUE};
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNORMAL, WM_USER};
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use wry::{Rect, WebView, WebViewBuilder};

use crate::core::game::Region;
use crate::core::gui::Window;
use crate::core::Gui;
use crate::core::Hachimi;
use crate::il2cpp::types::Il2CppString;
use egui::Context;
use rust_i18n::t;
use std::collections::HashMap;
use std::sync::RwLock;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use wry::dpi::{PhysicalPosition, PhysicalSize};

pub const WM_OPEN_WEBVIEW: u32 = WM_USER + 500;
pub const WM_CLOSE_WEBVIEW: u32 = WM_USER + 501;
pub const WM_SET_WEBVIEW_POSITION: u32 = WM_USER + 502;
pub const WM_WEBVIEW_GOBACK: u32 = WM_USER + 503;

struct WindowsWindowHandle {
    hwnd: HWND,
}

impl HasWindowHandle for WindowsWindowHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd_nonzero =
            NonZeroIsize::new(self.hwnd.0 as isize).ok_or(HandleError::Unavailable)?;
        let handle = Win32WindowHandle::new(hwnd_nonzero);
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}

struct DialogWebView {
    webview: Option<WebView>,
    handle: WindowsWindowHandle,
}

impl DialogWebView {
    pub fn new(hwnd: HWND) -> DialogWebView {
        DialogWebView {
            webview: None,
            handle: WindowsWindowHandle { hwnd },
        }
    }

    fn set_position(&self, position: Rect) {
        if let Some(ref webview) = self.webview {
            match webview.set_bounds(position) {
                Ok(_) => {}
                Err(e) => warn!("set_bound error：{:?}", e),
            }
        }
    }
    fn close(&mut self) {
        self.webview = None;
        unsafe {
            if InvalidateRect(Some(self.handle.hwnd), None, true) == TRUE {
                let _ = UpdateWindow(self.handle.hwnd);
            }
        }
    }

    fn back(&self) {
        if let Some(ref webview) = self.webview {
            if let Err(e) = webview.evaluate_script("window.history.back()") {
                warn!("webview back failure: {:?}", e);
            }
        }
    }

    fn open(&mut self, url: &String, title: &String, orig_url: &String) {
        let url_string = String::from(url);
        match WebViewBuilder::new()
            .with_url(url_string)
            .build_as_child(&self.handle)
        {
            Ok(wv) => {
                open_dialog(title.clone(), orig_url.clone());
                self.webview = Some(wv);
            }
            Err(e) => {
                warn!("Failed to build webview: {:?}", e);
                unsafe {
                    ShellExecuteW(
                        None,
                        w!("open"),
                        PWSTR(widestring::U16CString::from_str(orig_url).unwrap().as_ptr() as _),
                        None,
                        None,
                        SW_SHOWNORMAL,
                    );
                }
            }
        }
    }
}

static mut DIALOG_WEBVIEW: Lazy<DialogWebView> =
    Lazy::new(|| DialogWebView::new(get_target_hwnd()));

pub fn process_massage(umsg: c_uint, lparam: LPARAM) {
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
        let title = {
            if let Some(translated) =
                crate::_rust_i18n_try_translate(&rust_i18n::locale(), &translated_title_key)
            {
                translated.into()
            } else {
                rust_i18n::CowStr::from(t!("external_link_dialog.title.general").to_string())
                    .into_inner()
            }
        }
        .to_string();
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
fn pares_url(url: &String) -> (String, HashMap<String, String>) {
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
    if Hachimi::instance().game.region == Region::Japan
        && !Hachimi::instance()
            .config
            .load()
            .windows
            .open_external_link_with_hachimi
    {
        return false;
    }

    let url_string = il2cppstring_as_string(unsafe { &*url });

    let (base_url, params) = pares_url(&url_string);
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

static WEBVIEW_RECT: RwLock<Option<Rect>> = RwLock::new(None);

impl Window for ExternalLinkDialog {
    fn run(&mut self, ctx: &Context) -> bool {
        let mut open = true;
        let mut open1 = true;
        let view_rect = ctx.viewport_rect();
        let view_width = view_rect.width();
        let view_height = view_rect.height();

        let target_height = view_height * 0.9;
        let target_width = if view_width * 0.9 > 280f32 {
            view_width * 0.9
        } else {
            280f32
        };
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
                        if ui.button(t!("external_link_dialog.back")).clicked() {
                            let _ = PostMessageW(
                                Some(get_target_hwnd()),
                                WM_WEBVIEW_GOBACK,
                                Default::default(),
                                Default::default(),
                            );
                        }
                        if ui
                            .button(t!("external_link_dialog.open_origin_link"))
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
                    let h = inner_rect.height() * scale;
                    let x = inner_rect.min.x * scale;
                    let y = inner_rect.min.y * scale;
                    let w = inner_rect.width() * scale;
                    let rect = wry::Rect {
                        position: PhysicalPosition::new(x, y).into(),
                        size: PhysicalSize::new(w, h).into(),
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
