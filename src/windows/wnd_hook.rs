use std::{os::raw::c_uint, ptr, sync::{Arc, atomic::{self, AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicUsize}}};

use rust_i18n::t;
use windows::{core::{w, HSTRING}, Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{RedrawWindow, RDW_ALLCHILDREN, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
    UI::{
        Input::{Ime::ISC_SHOWUICOMPOSITIONWINDOW, KeyboardAndMouse::VK_RETURN},
        WindowsAndMessaging::{
            CallNextHookEx, CallWindowProcW, DefWindowProcW, FindWindowW, GetClientRect, GetWindowLongPtrW, GetWindowRect,
            SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW, UnhookWindowsHookEx, SetWindowTextW,
            GWLP_WNDPROC, HCBT_MINMAX, HHOOK, SW_RESTORE, WH_CBT, WM_CLOSE, WM_KEYDOWN, WM_SYSKEYDOWN, WNDPROC,
            WM_IME_SETCONTEXT, WM_IME_NOTIFY, WM_ACTIVATE, WA_INACTIVE, GWL_STYLE, SIZE_MAXIMIZED,
            SIZE_MINIMIZED,
            SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_ENTERSIZEMOVE,
            WM_EXITSIZEMOVE, WM_KEYUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVING, WM_RBUTTONDOWN,
            WM_RBUTTONUP, WM_SIZE, WM_SIZING, WM_SYSKEYUP, WM_INPUT, WINDOW_LONG_PTR_INDEX, WS_MAXIMIZEBOX
        },
    }
}};

use crate::{core::{free_camera, game::Region, gui, Gui, Hachimi}, il2cpp::{hook::{umamusume, UnityEngine_CoreModule}, symbols::{create_delegate, get_assembly_image, get_class, get_method_addr, Thread}, types::{Il2CppDelegate, RefreshRate}}, windows::utils};

use super::{gui_impl::input, discord, smtc, taskbar};

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
static ALT_ENTER_PRESSED: AtomicBool = AtomicBool::new(false);
static WNDPROC_ORIG: AtomicIsize = AtomicIsize::new(0);
static GAME_WNDPROC_ORIG: AtomicIsize = AtomicIsize::new(0);
static RESTORING_WNDPROC: AtomicBool = AtomicBool::new(false);
static WNDPROC_INLINE_HOOKED: AtomicBool = AtomicBool::new(false);
static RESIZE_WAIT_ACTIVE: AtomicBool = AtomicBool::new(false);
static RESIZE_WAIT_FRAMES: AtomicI32 = AtomicI32::new(0);
static RESIZE_GENERATION: AtomicU32 = AtomicU32::new(0);
static RESIZE_WAIT_FOR_END_FRAME_ADDR: AtomicUsize = AtomicUsize::new(0);
static FREEFORM_LANDSCAPE_CLOSE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static SET_WINDOW_LONG_PTR_W_HOOK_ID: AtomicBool = AtomicBool::new(false);
static SET_WINDOW_LONG_PTR_A_HOOK_ID: AtomicBool = AtomicBool::new(true);

pub fn get_target_hwnd() -> HWND {
    HWND(TARGET_HWND.load(atomic::Ordering::Relaxed) as *mut _)
}

pub fn get_client_size() -> Option<(i32, i32)> {
    let hwnd = get_target_hwnd();
    if hwnd.0 == ptr::null_mut() {
        return None;
    }

    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
        return None;
    }

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        None
    }
    else {
        Some((width, height))
    }
}

pub fn apply_freeform_window_style() {
    if close_freeform_window_for_landscape() {
        return;
    }

    if !Hachimi::instance().config.load().windows.freeform_window {
        return;
    }

    let hwnd = get_target_hwnd();
    if hwnd.0 == ptr::null_mut() {
        return;
    }

    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let maximize_style = WS_MAXIMIZEBOX.0 as isize;
        if style & maximize_style == 0 {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style | maximize_style);
            _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER
            );
        }
    }
}

fn refresh_freeform_window() {
    let hwnd = get_target_hwnd();
    let mut client_rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client_rect) }.is_err() {
        return;
    }

    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;
    if width <= 0 || height <= 0 {
        return;
    }

    let mut window_rect = RECT::default();
    let (window_width, window_height) = if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_ok() {
        (window_rect.right - window_rect.left, window_rect.bottom - window_rect.top)
    }
    else {
        (width, height)
    };

    umamusume::StandaloneWindowResize::update_window_state(
        width,
        height,
        window_width,
        window_height
    );
    umamusume::UIManager::refresh_after_window_resize(width, height);
    umamusume::StandaloneWindowResize::finish_window_update();
    apply_freeform_window_style();
    unsafe {
        let _ = RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_FRAME
        );
    }
}

fn wait_for_resize_end_frame(callback: fn()) -> bool {
    let addr = RESIZE_WAIT_FOR_END_FRAME_ADDR.load(atomic::Ordering::Acquire);
    let game_system = umamusume::GameSystem::instance();
    let delegate_class = unsafe {
        UnityEngine_CoreModule::UnityAction::UNITYACTION_CLASS
    };
    if addr == 0 || game_system.is_null() || delegate_class.is_null() {
        return false;
    }

    let Some(delegate) = create_delegate(delegate_class, 0, callback) else {
        return false;
    };
    let wait_for_end_frame: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject, *mut Il2CppDelegate) =
        unsafe { std::mem::transmute(addr) };
    wait_for_end_frame(game_system, delegate);
    true
}

fn resize_end_frame_tick() {
    let generation = RESIZE_GENERATION.load(atomic::Ordering::Acquire);
    let frames = RESIZE_WAIT_FRAMES.fetch_sub(1, atomic::Ordering::AcqRel);
    if frames > 1 {
        if !wait_for_resize_end_frame(resize_end_frame_tick) {
            Thread::main_thread().schedule(resize_end_frame_tick);
        }
        return;
    }

    refresh_freeform_window();

    if generation != RESIZE_GENERATION.load(atomic::Ordering::Acquire) {
        RESIZE_WAIT_FRAMES.store(2, atomic::Ordering::Release);
        if !wait_for_resize_end_frame(resize_end_frame_tick) {
            Thread::main_thread().schedule(resize_end_frame_tick);
        }
        return;
    }

    RESIZE_WAIT_ACTIVE.store(false, atomic::Ordering::Release);
    if generation != RESIZE_GENERATION.load(atomic::Ordering::Acquire) &&
        !RESIZE_WAIT_ACTIVE.swap(true, atomic::Ordering::AcqRel)
    {
        RESIZE_WAIT_FRAMES.store(2, atomic::Ordering::Release);
        if !wait_for_resize_end_frame(resize_end_frame_tick) {
            Thread::main_thread().schedule(resize_end_frame_tick);
        }
    }
}

fn begin_resize_end_frame_wait() {
    if !wait_for_resize_end_frame(resize_end_frame_tick) {
        Thread::main_thread().schedule(resize_end_frame_tick);
    }
}

fn queue_freeform_resize(width: i32, height: i32) {
    if width <= 0 || height <= 0 {
        return;
    }

    RESIZE_GENERATION.fetch_add(1, atomic::Ordering::AcqRel);
    RESIZE_WAIT_FRAMES.store(2, atomic::Ordering::Release);
    if !RESIZE_WAIT_ACTIVE.swap(true, atomic::Ordering::AcqRel) {
        Thread::main_thread().schedule(begin_resize_end_frame_wait);
    }
}

fn queue_current_client_resize(hwnd: HWND) {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_ok() {
        queue_freeform_resize(rect.right - rect.left, rect.bottom - rect.top);
    }
}

fn is_freeform_window_supported() -> bool {
    Hachimi::instance().game.region != Region::Global
}

fn apply_freeform_window_disabled() {
    Thread::main_thread().schedule(|| {
        umamusume::StandaloneWindowResize::set_is_prevent_reshape(false);
        umamusume::StandaloneWindowResize::set_is_window_dragging(false);
        umamusume::StandaloneWindowResize::set_is_window_size_changing(false);
        umamusume::StandaloneWindowResize::finish_window_update();
        umamusume::UIManager::apply_ui_scale();

        let hwnd = get_target_hwnd();
        unsafe {
            let _ = RedrawWindow(
                Some(hwnd),
                None,
                None,
                RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_FRAME
            );
        }
    });
}

fn disable_freeform_window_config() -> bool {
    let hachimi = Hachimi::instance();
    let config = hachimi.config.load();
    if !config.windows.freeform_window {
        return false;
    }

    let mut new_config = config.as_ref().clone();
    drop(config);
    new_config.windows.freeform_window = false;

    if let Err(e) = hachimi.save_and_reload_config(new_config.clone()) {
        error!("Failed to save disabled freeform window config: {}", e);
        hachimi.config.store(Arc::new(new_config));
    }

    true
}

fn disable_freeform_window() -> bool {
    if !disable_freeform_window_config() {
        return false;
    }

    apply_freeform_window_disabled();
    true
}

pub fn disable_freeform_window_if_unsupported() -> bool {
    if is_freeform_window_supported() {
        return false;
    }

    disable_freeform_window()
}

pub fn close_freeform_window_for_landscape() -> bool {
    if disable_freeform_window_if_unsupported() {
        return true;
    }

    if !umamusume::Screen::is_landscape_mode() {
        return false;
    }

    if !Hachimi::instance().config.load().windows.freeform_window {
        return false;
    }

    if FREEFORM_LANDSCAPE_CLOSE_IN_PROGRESS.swap(true, atomic::Ordering::AcqRel) {
        return true;
    }

    if disable_freeform_window() {
        gui::request_notification(gui::NotificationRequest::Custom(
            t!("notification.freeform_window_disabled_landscape").into_owned()
        ));
    }
    FREEFORM_LANDSCAPE_CLOSE_IN_PROGRESS.store(false, atomic::Ordering::Release);
    true
}

pub fn apply_freeform_window_config() {
    if disable_freeform_window_if_unsupported() {
        return;
    }

    if close_freeform_window_for_landscape() {
        return;
    }

    if !Hachimi::instance().config.load().windows.freeform_window {
        apply_freeform_window_disabled();
        return;
    }

    apply_freeform_window_style();
    queue_current_client_resize(get_target_hwnd());
}

fn toggle_freeform_full_screen() {
    use crate::il2cpp::hook::UnityEngine_CoreModule::{
        FullScreenMode_FullScreenWindow, FullScreenMode_Windowed, Screen
    };

    let resolution = Screen::get_currentResolution();
    let mode = if Screen::get_fullScreen() {
        FullScreenMode_Windowed
    }
    else {
        FullScreenMode_FullScreenWindow
    };
    let refresh_rate = RefreshRate { numerator: 0, denominator: 1 };
    Screen::set_resolution_direct(
        resolution.width,
        resolution.height,
        mode,
        &refresh_rate
    );
}

type SetWindowLongPtrFn = unsafe extern "system" fn(HWND, WINDOW_LONG_PTR_INDEX, isize) -> isize;

unsafe extern "system" fn set_window_long_ptr_w_hook(
    hwnd: HWND,
    index: WINDOW_LONG_PTR_INDEX,
    new_long: isize
) -> isize {
    std::hint::black_box(SET_WINDOW_LONG_PTR_W_HOOK_ID.load(atomic::Ordering::Relaxed));
    let orig_fn = get_orig_fn!(set_window_long_ptr_w_hook, SetWindowLongPtrFn);
    let target_hwnd = get_target_hwnd();

    if hwnd.0 == target_hwnd.0 &&
        index == GWLP_WNDPROC &&
        !RESTORING_WNDPROC.load(atomic::Ordering::Acquire)
    {
        if new_long != 0 && new_long != wnd_proc as *const () as isize {
            return GAME_WNDPROC_ORIG.swap(new_long, atomic::Ordering::AcqRel);
        }
        return GAME_WNDPROC_ORIG.load(atomic::Ordering::Acquire);
    }

    let mut new_long = new_long;
    if hwnd.0 == target_hwnd.0 &&
        index == GWL_STYLE &&
        Hachimi::instance().config.load().windows.freeform_window
    {
        new_long |= WS_MAXIMIZEBOX.0 as isize;
    }

    orig_fn(hwnd, index, new_long)
}

unsafe extern "system" fn set_window_long_ptr_a_hook(
    hwnd: HWND,
    index: WINDOW_LONG_PTR_INDEX,
    new_long: isize
) -> isize {
    std::hint::black_box(SET_WINDOW_LONG_PTR_A_HOOK_ID.load(atomic::Ordering::Relaxed));
    let orig_fn = get_orig_fn!(set_window_long_ptr_a_hook, SetWindowLongPtrFn);
    let target_hwnd = get_target_hwnd();

    if hwnd.0 == target_hwnd.0 &&
        index == GWLP_WNDPROC &&
        !RESTORING_WNDPROC.load(atomic::Ordering::Acquire)
    {
        if new_long != 0 && new_long != wnd_proc as *const () as isize {
            return GAME_WNDPROC_ORIG.swap(new_long, atomic::Ordering::AcqRel);
        }
        return GAME_WNDPROC_ORIG.load(atomic::Ordering::Acquire);
    }

    let mut new_long = new_long;
    if hwnd.0 == target_hwnd.0 &&
        index == GWL_STYLE &&
        Hachimi::instance().config.load().windows.freeform_window
    {
        new_long |= WS_MAXIMIZEBOX.0 as isize;
    }

    orig_fn(hwnd, index, new_long)
}

fn restore_original_wnd_proc(hwnd: HWND) {
    if WNDPROC_INLINE_HOOKED.load(atomic::Ordering::Acquire) {
        return;
    }

    let freeform_orig = WNDPROC_ORIG.swap(0, atomic::Ordering::AcqRel);
    let game_orig = GAME_WNDPROC_ORIG.swap(0, atomic::Ordering::AcqRel);
    let orig = if game_orig != 0 { game_orig } else { freeform_orig };
    if orig == 0 {
        return;
    }

    RESTORING_WNDPROC.store(true, atomic::Ordering::Release);
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, orig);
    }
    RESTORING_WNDPROC.store(false, atomic::Ordering::Release);
}

extern "system" fn wnd_proc(hwnd: HWND, umsg: c_uint, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let freeform_window = Hachimi::instance().config.load().windows.freeform_window;
    let inline_hooked = WNDPROC_INLINE_HOOKED.load(atomic::Ordering::Acquire);

    let orig_addr = if inline_hooked || freeform_window {
        WNDPROC_ORIG.load(atomic::Ordering::Acquire)
    }
    else {
        GAME_WNDPROC_ORIG.load(atomic::Ordering::Acquire)
    };
    let Some(orig_fn) = (unsafe {
        std::mem::transmute::<isize, WNDPROC>(orig_addr)
    }) else {
        return unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) };
    };

    if freeform_window {
        if umsg == WM_SYSKEYDOWN &&
            wparam.0 == VK_RETURN.0 as usize &&
            lparam.0 & (1 << 29) != 0
        {
            if !ALT_ENTER_PRESSED.swap(true, atomic::Ordering::AcqRel) {
                Thread::main_thread().schedule(toggle_freeform_full_screen);
            }
            return LRESULT(0);
        }

        if umsg == WM_SYSKEYUP && wparam.0 == VK_RETURN.0 as usize {
            ALT_ENTER_PRESSED.store(false, atomic::Ordering::Release);
            return LRESULT(0);
        }

        if umsg == WM_SIZING {
            // Keep the proposed RECT untouched. The game's original WndProc
            // rewrites it here to enforce its portrait/landscape aspect ratio.
            return LRESULT(1);
        }
        else if umsg == WM_ENTERSIZEMOVE {
            umamusume::StandaloneWindowResize::set_is_window_size_changing(true);
        }
        else if umsg == WM_MOVING {
            umamusume::StandaloneWindowResize::set_is_window_dragging(true);
        }
        else if umsg == WM_EXITSIZEMOVE {
            let res = unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
            umamusume::StandaloneWindowResize::set_is_window_dragging(false);
            umamusume::StandaloneWindowResize::set_is_window_size_changing(false);
            queue_current_client_resize(hwnd);
            return res;
        }
        else if umsg == WM_SIZE {
            let res = unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
            if wparam.0 != SIZE_MINIMIZED as usize {
                let width = (lparam.0 & 0xFFFF) as u16 as i32;
                let height = ((lparam.0 >> 16) & 0xFFFF) as u16 as i32;
                queue_freeform_resize(width, height);

                if wparam.0 == SIZE_MAXIMIZED as usize {
                    unsafe {
                        let _ = RedrawWindow(
                            Some(hwnd),
                            None,
                            None,
                            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN | RDW_FRAME
                        );
                    }
                }
            }
            return res;
        }
    }

    match umsg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let current_key = wparam.0 as u16;
            let repeat = ((lparam.0 as usize) & (1usize << 30)) != 0;

            if gui::is_keybind_capture_active() {
                let display = utils::vk_to_display_label(current_key);
                gui::report_keybind_capture(current_key, display);
                return LRESULT(0);
            }

            if current_key == 0x4B { // Virtual keycode for "K", see the get_key method on gui_impl/input.rs
                let hotkey_vk = Hachimi::instance().config.load().windows.hide_ingame_ui_hotkey_bind;

                if unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(hotkey_vk as i32) < 0 } {
                    if let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) {
                        gui.set_consuming_input(false);
                    }
                    return LRESULT(0); 
                }
            }

            if current_key == Hachimi::instance().config.load().windows.menu_open_key {
                let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                    return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
                };
                gui.toggle_menu();
                return LRESULT(0);
            } else if current_key == Hachimi::instance().config.load().windows.hide_ingame_ui_hotkey_bind && Hachimi::instance().config.load().hide_ingame_ui_hotkey {
                Thread::main_thread().schedule(Gui::toggle_game_ui);
            }

            if !Gui::is_gui_input_active_atomic() {
                free_camera::on_windows_key(current_key, true, repeat);
                if free_camera::is_windows_key_bound(current_key) {
                    return LRESULT(0);
                }
            }
        },
        WM_KEYUP | WM_SYSKEYUP => {
            let current_key = wparam.0 as u16;
            if !Gui::is_gui_input_active_atomic() {
                free_camera::on_windows_key(current_key, false, false);
                if free_camera::is_windows_key_bound(current_key) {
                    return LRESULT(0);
                }
            }
        },
        WM_RBUTTONDOWN => {
            if !Gui::is_gui_input_active_atomic() {
                free_camera::on_mouse_button(true);
                if free_camera::is_game_input_capture_active() {
                    return LRESULT(0);
                }
            }
        },
        WM_RBUTTONUP => {
            if !Gui::is_gui_input_active_atomic() {
                free_camera::on_mouse_button(false);
                if free_camera::is_game_input_capture_active() {
                    return LRESULT(0);
                }
            }
        },
        WM_MOUSEMOVE => {
            if !Gui::is_gui_input_active_atomic() {
                let x = (lparam.0 & 0xffff) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
                free_camera::on_mouse_move(x, y);
                if free_camera::wants_windows_input_capture() {
                    return LRESULT(0);
                }
            }
        },
        WM_MOUSEWHEEL => {
            if !Gui::is_gui_input_active_atomic() {
                let delta = (wparam.0 >> 16) as u16 as i16;
                free_camera::on_mouse_wheel(delta);
                if free_camera::is_game_input_capture_active() {
                    return LRESULT(0);
                }
            }
        },
        WM_INPUT => {
            if !Gui::is_gui_input_active_atomic() && free_camera::is_game_input_capture_active() {
                return LRESULT(0);
            }
        },
        WM_ACTIVATE => {
            let res = unsafe { orig_fn(hwnd, umsg, wparam, lparam) };

            if (wparam.0 & 0xFFFF) != WA_INACTIVE as usize {
                std::thread::spawn(move || {
                    if let Some(gui) = Gui::instance().map(|m| m.lock().unwrap()) {
                        if gui.context.wants_keyboard_input() {
                            Thread::main_thread().schedule(|| {
                                crate::il2cpp::hook::UnityEngine_InputLegacyModule::Input::set_imeCompositionMode(1);
                            });
                        }
                    }
                });
            }
            return res;
        },
        WM_CLOSE => {
            return unsafe { CallWindowProcW(Some(orig_fn), hwnd, umsg, wparam, lparam) };
        },
        _ => ()
    }

    // Only capture input if gui needs it
    if !Gui::is_consuming_input_atomic() {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    if umsg == WM_IME_SETCONTEXT {
        let new_lparam = lparam.0 & !(ISC_SHOWUICOMPOSITIONWINDOW as isize);
        if Gui::is_consuming_input_atomic() {
            return unsafe { DefWindowProcW(hwnd, umsg, wparam, LPARAM(new_lparam)) };
        }
        return unsafe { orig_fn(hwnd, umsg, wparam, LPARAM(new_lparam)) };
    }

    if umsg == WM_IME_NOTIFY {
        if Gui::is_consuming_input_atomic() {
            return unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) };
        }
    }

    // Extract the IME data BEFORE spanning the thread
    let (is_ime, ime_commit, ime_preedit) = input::process_ime_sync(hwnd, umsg, lparam.0);

    // Check if the input processor handles this message (Skip check if it is an IME msg)
    if !input::is_handled_msg(umsg) && !is_ime {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    // A deadlock would *sometimes* consistently occur if this was done on the current thread
    // (when moving the window, etc.)
    // I assume that SwapChain::Present and WndProc are running on the same thread
    std::thread::spawn(move || {
        let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
            return;
        };

        // Inject IME strings directly into egui
        if let Some(s) = ime_commit {
            gui.input.events.push(egui::Event::Ime(egui::ImeEvent::Commit(s)));
        }
        if let Some(s) = ime_preedit {
            gui.input.events.push(egui::Event::Ime(egui::ImeEvent::Preedit(s)));
        }

        // Process standard Key/Mouse inputs ONLY if it wasn't an IME message
        if !is_ime {
            let zoom_factor = gui.context.zoom_factor();
            input::process(&mut gui.input, zoom_factor, umsg, wparam.0, lparam.0);
        }
    });

    if is_ime {
        return LRESULT(0);
    }

    if !Gui::wants_input_atomic() {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    LRESULT(0)
}

static mut HCBTHOOK: HHOOK = HHOOK(ptr::null_mut());
extern "system" fn cbt_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == HCBT_MINMAX as i32 &&
        lparam.0 as i32 != SW_RESTORE.0 &&
        Hachimi::instance().config.load().windows.block_minimize_in_full_screen &&
        UnityEngine_CoreModule::Screen::get_fullScreen()
    {
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(Some(HCBTHOOK), ncode, wparam, lparam) }
}

pub fn init() {
    unsafe {
        let hachimi = Hachimi::instance();
        let game = &hachimi.game;

        let window_name = if game.region == Region::Japan && game.is_steam_release {
            // lmao
            w!("UmamusumePrettyDerby_Jpn")
        }
        else {
            // global technically has "Umamusume" as its title but this api
            // is case insensitive so it works. why am i surprised
            w!("umamusume")
        };
        let hwnd = FindWindowW(w!("UnityWndClass"), window_name).unwrap_or_default();
        if hwnd.0 == ptr::null_mut() {
            error!("Failed to find game window");
            return;
        }
        TARGET_HWND.store(hwnd.0 as isize, atomic::Ordering::Relaxed);

        let title = hachimi.config.load().windows.custom_title_name.clone();
        if let Some(t) = title {
            let _ = SetWindowTextW(hwnd, &HSTRING::from(t));
        }

        taskbar::init(hwnd);

        if let Ok(umamusume) = get_assembly_image(c"umamusume.dll") {
            if let Ok(mono_behaviour_extension) =
                get_class(umamusume, c"Gallop", c"MonoBehaviourExtension")
            {
                RESIZE_WAIT_FOR_END_FRAME_ADDR.store(
                    get_method_addr(mono_behaviour_extension, c"WaitForEndFrame", 2),
                    atomic::Ordering::Release
                );
            }
        }

        info!("Subclassing game window");
        let wnd_proc_orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc as *const () as isize);
        if wnd_proc_orig == 0 {
            error!("Failed to subclass game window");
        }
        else {
            WNDPROC_ORIG.store(wnd_proc_orig, atomic::Ordering::Release);
            GAME_WNDPROC_ORIG.store(wnd_proc_orig, atomic::Ordering::Release);

            let actual_wndproc = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if actual_wndproc != 0 && actual_wndproc != wnd_proc as *const () as isize {
                info!("SetWindowLongPtrW was swallowed (localify detected), falling back to inline WndProc hook");
                match hachimi.interceptor.hook(
                    actual_wndproc as usize,
                    wnd_proc as *const () as _
                ) {
                    Ok(_) => {
                        let trampoline = hachimi.interceptor.get_trampoline_addr(
                            wnd_proc as *const () as usize
                        );
                        WNDPROC_ORIG.store(trampoline as isize, atomic::Ordering::Release);
                        GAME_WNDPROC_ORIG.store(trampoline as isize, atomic::Ordering::Release);
                        WNDPROC_INLINE_HOOKED.store(true, atomic::Ordering::Release);
                    }
                    Err(e) => {
                        error!("Failed to inline-hook window procedure: {}", e);
                    }
                }
            }

            if let Ok(user32) = GetModuleHandleW(w!("user32.dll")) {
                let set_window_long_ptr_w_addr = utils::get_proc_address(user32, c"SetWindowLongPtrW");
                let set_window_long_ptr_a_addr = utils::get_proc_address(user32, c"SetWindowLongPtrA");

                info!("Hooking SetWindowLongPtrW");
                if set_window_long_ptr_w_addr == 0 {
                    error!("Failed to find SetWindowLongPtrW");
                }
                else if let Err(e) = hachimi.interceptor.hook(
                    set_window_long_ptr_w_addr,
                    set_window_long_ptr_w_hook as *const () as _
                ) {
                    error!("Failed to hook SetWindowLongPtrW: {}", e);
                }

                info!("Hooking SetWindowLongPtrA");
                if set_window_long_ptr_a_addr == 0 {
                    error!("Failed to find SetWindowLongPtrA");
                }
                else if let Err(e) = hachimi.interceptor.hook(
                    set_window_long_ptr_a_addr,
                    set_window_long_ptr_a_hook as *const () as _
                ) {
                    error!("Failed to hook SetWindowLongPtrA: {}", e);
                }
            }
            else {
                error!("Failed to get user32.dll module handle");
            }
        }

        info!("Adding CBT hook");
        if let Ok(hhook) = SetWindowsHookExW(WH_CBT, Some(cbt_proc), None, GetCurrentThreadId()) {
            HCBTHOOK = hhook;
        }

        // Apply always on top
        if hachimi.window_always_on_top.load(atomic::Ordering::Relaxed) {
            _ = utils::set_window_topmost(hwnd, true);
        }

        apply_freeform_window_config();

        if hachimi.discord_rpc.load(atomic::Ordering::Relaxed) {
            if let Err(e) = discord::start_rpc() {
                 error!("{}", e);
             }
        }

        smtc::init(hwnd);
    }
}

pub fn uninit() {
    unsafe {
        restore_original_wnd_proc(get_target_hwnd());
        Hachimi::instance().interceptor.unhook(set_window_long_ptr_w_hook as *const () as _);
        Hachimi::instance().interceptor.unhook(set_window_long_ptr_a_hook as *const () as _);

        if HCBTHOOK.0 != ptr::null_mut() {
            info!("Removing CBT hook");
            if let Err(e) = UnhookWindowsHookEx(HCBTHOOK) {
                error!("Failed to remove CBT hook: {}", e);
            }
            HCBTHOOK = HHOOK(ptr::null_mut());
        }
        if let Err(e) = discord::stop_rpc() {
            error!("{}", e);
        }
    }
}
