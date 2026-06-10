use std::{os::raw::c_uint, ptr, sync::{atomic::{self, AtomicIsize}}};

use windows::{core::{w, HSTRING}, Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::Ime::ISC_SHOWUICOMPOSITIONWINDOW,
        WindowsAndMessaging::{
            CallNextHookEx, DefWindowProcW, FindWindowW, GetWindowLongPtrW, SetWindowsHookExW, UnhookWindowsHookEx,
            SetWindowTextW,
            GWLP_WNDPROC, HCBT_MINMAX, HHOOK, SW_RESTORE, WH_CBT, WM_CLOSE, WM_KEYDOWN, WM_SYSKEYDOWN, WNDPROC,
            WM_IME_SETCONTEXT, WM_IME_NOTIFY, WM_ACTIVATE, WA_INACTIVE
        },
    }
}};

use crate::{core::{game::Region, gui, Gui, Hachimi}, il2cpp::{hook::UnityEngine_CoreModule, symbols::Thread}, windows::utils};

use super::{gui_impl::input, discord, smtc, taskbar};

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
pub fn get_target_hwnd() -> HWND {
    HWND(TARGET_HWND.load(atomic::Ordering::Relaxed) as *mut _)
}

// Safety: only modified once on init
static mut WNDPROC_ORIG: isize = 0;
static mut WNDPROC_RECALL: usize = 0;
extern "system" fn wnd_proc(hwnd: HWND, umsg: c_uint, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(orig_fn) = (unsafe { std::mem::transmute::<isize, WNDPROC>(WNDPROC_ORIG) }) else {
        return unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) };
    };

    match umsg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let current_key = wparam.0 as u16;

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
            if let Some(hook) = Hachimi::instance().interceptor.unhook(wnd_proc as *const () as _) {
                unsafe { WNDPROC_RECALL = hook.orig_addr; }
                Thread::main_thread().schedule(|| {
                    unsafe {
                        let orig_fn = std::mem::transmute::<usize, WNDPROC>(WNDPROC_RECALL).unwrap();
                        orig_fn(get_target_hwnd(), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                });
            }
            return LRESULT(0);
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

        info!("Hooking WndProc");
        let wnd_proc_addr = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        match hachimi.interceptor.hook(wnd_proc_addr as _, wnd_proc as *const () as _) {
            Ok(trampoline_addr) => WNDPROC_ORIG = trampoline_addr as _,
            Err(e) => error!("Failed to hook WndProc: {}", e)
        }

        info!("Adding CBT hook");
        if let Ok(hhook) = SetWindowsHookExW(WH_CBT, Some(cbt_proc), None, GetCurrentThreadId()) {
            HCBTHOOK = hhook;
        }

        // Apply always on top
        if hachimi.window_always_on_top.load(atomic::Ordering::Relaxed) {
            _ = utils::set_window_topmost(hwnd, true);
        }

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