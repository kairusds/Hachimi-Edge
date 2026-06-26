use crate::windows::external_link;
use crate::windows::external_link::dialog::WEBVIEW_RECT;
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
                external_link::dialog::open(title.clone(), orig_url.clone());
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
