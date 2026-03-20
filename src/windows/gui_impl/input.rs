// Originally from sy1ntexx/egui-d3d11
use egui::{Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Vec2};
use std::ffi::CStr;
use windows::Win32::{
    Foundation::HWND,
    System::{
        DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard},
        Ole::CF_TEXT,
        SystemServices::{MK_CONTROL, MK_SHIFT}
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
            VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT, VK_LSHIFT, VK_NEXT, VK_PRIOR, VK_RETURN,
            VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
        },
        WindowsAndMessaging::{
            WHEEL_DELTA, WM_CHAR, WM_KEYDOWN, WM_KEYUP,
            WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDBLCLK,
            WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_INPUT
        },
    },
};

/// High-level overview of recognized `WndProc` messages.
#[repr(u8)]
pub enum InputResult {
    Unknown,
    MouseMove,
    MouseLeft,
    MouseRight,
    MouseMiddle,
    Character,
    Scroll,
    Zoom,
    Key,
}

pub fn process(input: &mut RawInput, zoom_factor: f32, umsg: u32, wparam: usize, lparam: isize) -> InputResult {
    match umsg {
        WM_MOUSEMOVE => {
            input.events.push(Event::PointerMoved(get_pos(lparam) / zoom_factor));
            InputResult::MouseMove
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseLeft
        }
        WM_LBUTTONUP => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseLeft
        }
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Secondary,
                pressed: true,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseRight
        }
        WM_RBUTTONUP => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Secondary,
                pressed: false,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseRight
        }
        WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Middle,
                pressed: true,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseMiddle
        }
        WM_MBUTTONUP => {
            input.events.push(Event::PointerButton {
                pos: get_pos(lparam) / zoom_factor,
                button: PointerButton::Middle,
                pressed: false,
                modifiers: get_modifiers(wparam),
            });
            InputResult::MouseMiddle
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(wparam as _) {
                if !ch.is_control() {
                    input.events.push(Event::Text(ch.into()));
                }
            }
            InputResult::Character
        }
        WM_MOUSEWHEEL => {
            let delta = (wparam >> 16) as i16 as f32 * 10. / WHEEL_DELTA as f32;

            if wparam & MK_CONTROL.0 as usize != 0 {
                input.events.push(Event::Zoom(if delta > 0. { 1.5 } else { 0.5 }));
                InputResult::Zoom
            } else {
                input.events.push(Event::MouseWheel {
                    unit: MouseWheelUnit::Line,
                    delta: Vec2::new(0., delta),
                    modifiers: Modifiers::default()
                });
                InputResult::Scroll
            }
        }
        WM_MOUSEHWHEEL => {
            let delta = (wparam >> 16) as i16 as f32 * 10. / WHEEL_DELTA as f32;

            if wparam & MK_CONTROL.0 as usize != 0 {
                input.events.push(Event::Zoom(if delta > 0. { 1.5 } else { 0.5 }));
                InputResult::Zoom
            } else {
                input.events.push(Event::MouseWheel {
                    unit: MouseWheelUnit::Line,
                    delta: Vec2::new(delta, 0.),
                    modifiers: Modifiers::default()
                });
                InputResult::Scroll
            }
        }
        msg @ (WM_KEYDOWN | WM_SYSKEYDOWN) => {
            if let Some(key) = get_key(wparam) {
                let events = &mut input.events;
                let mods = get_key_modifiers(msg);

                if key == Key::Space {
                    events.push(Event::Text(String::from(" ")));
                } else if key == Key::V && mods.ctrl {
                    if let Some(clipboard) = get_clipboard_text() {
                        events.push(Event::Text(clipboard));
                    }
                } else if key == Key::C && mods.ctrl {
                    events.push(Event::Copy);
                } else if key == Key::X && mods.ctrl {
                    events.push(Event::Cut);
                } else {
                    events.push(Event::Key {
                        key,
                        pressed: true,
                        modifiers: get_key_modifiers(msg),
                        physical_key: None,
                        repeat: false,
                    });
                }
            }
            InputResult::Key
        }
        msg @ (WM_KEYUP | WM_SYSKEYUP) => {
            if let Some(key) = get_key(wparam) {
                input.events.push(Event::Key {
                    key,
                    pressed: false,
                    modifiers: get_key_modifiers(msg),
                    physical_key: None,
                    repeat: false,
                });
            }
            InputResult::Key
        }
        _ => InputResult::Unknown,
    }
}

pub fn is_handled_msg(umsg: u32) -> bool {
    match umsg {
        WM_CHAR | WM_KEYDOWN | WM_KEYUP |
        WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MBUTTONDBLCLK | WM_MBUTTONDOWN |
        WM_MBUTTONUP | WM_MOUSEHWHEEL | WM_MOUSEMOVE | WM_MOUSEWHEEL | WM_RBUTTONDBLCLK |
        WM_RBUTTONDOWN | WM_RBUTTONUP | WM_SYSKEYDOWN | WM_SYSKEYUP => true,
        WM_INPUT => true,
        _ => false
    }
}

fn get_pos(lparam: isize) -> Pos2 {
    let x = (lparam & 0xFFFF) as i16 as f32;
    let y = (lparam >> 16 & 0xFFFF) as i16 as f32;

    Pos2::new(x, y)
}

fn get_modifiers(wparam: usize) -> Modifiers {
    Modifiers {
        alt: false,
        ctrl: (wparam & MK_CONTROL.0 as usize) != 0,
        shift: (wparam & MK_SHIFT.0 as usize) != 0,
        mac_cmd: false,
        command: (wparam & MK_CONTROL.0 as usize) != 0,
    }
}

fn get_key_modifiers(msg: u32) -> Modifiers {
    let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as _) != 0 };
    let shift = unsafe { GetAsyncKeyState(VK_LSHIFT.0 as _) != 0 };

    Modifiers {
        alt: msg == WM_SYSKEYDOWN,
        mac_cmd: false,
        command: ctrl,
        shift,
        ctrl,
    }
}

fn get_key(wparam: usize) -> Option<Key> {
    let vk = VIRTUAL_KEY(wparam as u16);

    match vk {
        // nav/cmd keys
        VK_DOWN => Some(Key::ArrowDown),
        VK_LEFT => Some(Key::ArrowLeft),
        VK_RIGHT => Some(Key::ArrowRight),
        VK_UP => Some(Key::ArrowUp),
        VK_ESCAPE => Some(Key::Escape),
        VK_TAB => Some(Key::Tab),
        VK_BACK => Some(Key::Backspace),
        VK_RETURN => Some(Key::Enter),
        VK_SPACE => Some(Key::Space),
        VK_INSERT => Some(Key::Insert),
        VK_DELETE => Some(Key::Delete),
        VK_HOME => Some(Key::Home),
        VK_END => Some(Key::End),
        VK_PRIOR => Some(Key::PageUp),
        VK_NEXT => Some(Key::PageDown),

        // numbers 0-9
        VIRTUAL_KEY(0x30) => Some(Key::Num0),
        VIRTUAL_KEY(0x31) => Some(Key::Num1),
        VIRTUAL_KEY(0x32) => Some(Key::Num2),
        VIRTUAL_KEY(0x33) => Some(Key::Num3),
        VIRTUAL_KEY(0x34) => Some(Key::Num4),
        VIRTUAL_KEY(0x35) => Some(Key::Num5),
        VIRTUAL_KEY(0x36) => Some(Key::Num6),
        VIRTUAL_KEY(0x37) => Some(Key::Num7),
        VIRTUAL_KEY(0x38) => Some(Key::Num8),
        VIRTUAL_KEY(0x39) => Some(Key::Num9),

        // letters a-z
        VIRTUAL_KEY(0x41) => Some(Key::A),
        VIRTUAL_KEY(0x42) => Some(Key::B),
        VIRTUAL_KEY(0x43) => Some(Key::C),
        VIRTUAL_KEY(0x44) => Some(Key::D),
        VIRTUAL_KEY(0x45) => Some(Key::E),
        VIRTUAL_KEY(0x46) => Some(Key::F),
        VIRTUAL_KEY(0x47) => Some(Key::G),
        VIRTUAL_KEY(0x48) => Some(Key::H),
        VIRTUAL_KEY(0x49) => Some(Key::I),
        VIRTUAL_KEY(0x4A) => Some(Key::J),
        VIRTUAL_KEY(0x4B) => Some(Key::K),
        VIRTUAL_KEY(0x4C) => Some(Key::L),
        VIRTUAL_KEY(0x4D) => Some(Key::M),
        VIRTUAL_KEY(0x4E) => Some(Key::N),
        VIRTUAL_KEY(0x4F) => Some(Key::O),
        VIRTUAL_KEY(0x50) => Some(Key::P),
        VIRTUAL_KEY(0x51) => Some(Key::Q),
        VIRTUAL_KEY(0x52) => Some(Key::R),
        VIRTUAL_KEY(0x53) => Some(Key::S),
        VIRTUAL_KEY(0x54) => Some(Key::T),
        VIRTUAL_KEY(0x55) => Some(Key::U),
        VIRTUAL_KEY(0x56) => Some(Key::V),
        VIRTUAL_KEY(0x57) => Some(Key::W),
        VIRTUAL_KEY(0x58) => Some(Key::X),
        VIRTUAL_KEY(0x59) => Some(Key::Y),
        VIRTUAL_KEY(0x5A) => Some(Key::Z),

        _ => None
    }
}

fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(Some(HWND::default())).is_ok() {
            if let Ok(handle) = GetClipboardData(CF_TEXT.0 as u32) {
                let txt = handle.0 as *const i8;
                let data = Some(CStr::from_ptr(txt).to_str().ok()?.to_string());
                CloseClipboard().ok();
                return data;
            }
        }

        None
    }
}