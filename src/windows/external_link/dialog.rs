use crate::core::gui::Window;
use crate::core::Gui;
use crate::windows::external_link::webview::{
    WM_CLOSE_WEBVIEW, WM_SET_WEBVIEW_POSITION, WM_WEBVIEW_GOBACK,
};
use crate::windows::wnd_hook::get_target_hwnd;
use egui::Context;
use std::sync::RwLock;
use rust_i18n::t;
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, SW_SHOWNORMAL};
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::Rect;

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

pub static WEBVIEW_RECT: RwLock<Option<Rect>> = RwLock::new(None);

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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min),|ui| {
                        if ui.button(t!("external_link_dialog.back")).clicked() {
                            let _ = PostMessageW(
                                Some(get_target_hwnd()),
                                WM_WEBVIEW_GOBACK,
                                Default::default(),
                                Default::default(),
                            );
                        }
                        if ui.button(t!("external_link_dialog.open_origin_link")).clicked() {
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
pub fn open(title: String, orig_url: String) {
    Gui::instance()
        .unwrap()
        .lock()
        .unwrap()
        .show_window(Box::new(ExternalLinkDialog::new(title, orig_url)));
}
