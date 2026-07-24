use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::{core::PCSTR, Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA}};

use crate::core::Hachimi;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XInputGamepad {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XInputState {
    pub packet_number: u32,
    pub gamepad: XInputGamepad,
}

type XInputGetStateFn = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

static GET_STATE: OnceCell<Option<XInputGetStateFn>> = OnceCell::new();
static GET_STATE_ORIG: AtomicUsize = AtomicUsize::new(0);
static GET_STATE_1_4_ORIG: AtomicUsize = AtomicUsize::new(0);
static GET_STATE_1_3_ORIG: AtomicUsize = AtomicUsize::new(0);
static GET_STATE_9_1_0_ORIG: AtomicUsize = AtomicUsize::new(0);

pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const LEFT_SHOULDER: u16 = 0x0100;
pub const RIGHT_SHOULDER: u16 = 0x0200;
pub const A: u16 = 0x1000;
pub const B: u16 = 0x2000;
pub const X: u16 = 0x4000;
pub const Y: u16 = 0x8000;

pub fn get_state(user_index: u32) -> Option<XInputState> {
    let get_state = GET_STATE.get_or_init(load_get_state).as_ref().copied()?;
    let mut state = XInputState::default();
    let result = unsafe { get_state(user_index, &mut state) };
    if result == 0 {
        Some(state)
    }
    else {
        None
    }
}

pub fn ensure_hook() {
    let _ = GET_STATE.get_or_init(load_get_state);
}

pub fn unhook() {
    let interceptor = &Hachimi::instance().interceptor;
    interceptor.unhook(get_state_1_4_hook as *const () as usize);
    interceptor.unhook(get_state_1_3_hook as *const () as usize);
    interceptor.unhook(get_state_9_1_0_hook as *const () as usize);
}

unsafe fn call_get_state_hook(
    orig_addr: usize,
    user_index: u32,
    state: *mut XInputState,
) -> u32 {
    let orig_fn: XInputGetStateFn = std::mem::transmute(orig_addr);
    let result = orig_fn(user_index, state);
    if result == 0 &&
        !state.is_null() &&
        Hachimi::instance().config.load().windows.free_camera.enabled
    {
        (*state).gamepad = XInputGamepad::default();
    }
    result
}

unsafe extern "system" fn get_state_1_4_hook(user_index: u32, state: *mut XInputState) -> u32 {
    call_get_state_hook(GET_STATE_1_4_ORIG.load(Ordering::Acquire), user_index, state)
}

unsafe extern "system" fn get_state_1_3_hook(user_index: u32, state: *mut XInputState) -> u32 {
    call_get_state_hook(GET_STATE_1_3_ORIG.load(Ordering::Acquire), user_index, state)
}

unsafe extern "system" fn get_state_9_1_0_hook(user_index: u32, state: *mut XInputState) -> u32 {
    call_get_state_hook(GET_STATE_9_1_0_ORIG.load(Ordering::Acquire), user_index, state)
}

fn load_get_state() -> Option<XInputGetStateFn> {
    let dlls: [(&[u8], unsafe extern "system" fn(u32, *mut XInputState) -> u32, &AtomicUsize); 3] = [
        (b"xinput1_4.dll\0", get_state_1_4_hook, &GET_STATE_1_4_ORIG),
        (b"xinput1_3.dll\0", get_state_1_3_hook, &GET_STATE_1_3_ORIG),
        (b"xinput9_1_0.dll\0", get_state_9_1_0_hook, &GET_STATE_9_1_0_ORIG),
    ];
    for (dll, hook, orig_slot) in dlls {
        let Ok(module) = (unsafe { LoadLibraryA(PCSTR(dll.as_ptr())) }) else {
            continue;
        };
        let Some(proc) = (unsafe { GetProcAddress(module, PCSTR(b"XInputGetState\0".as_ptr())) }) else {
            continue;
        };
        let proc_addr = proc as usize;
        if orig_slot.load(Ordering::Acquire) == 0 {
            match Hachimi::instance().interceptor.hook(
                proc_addr,
                hook as *const () as usize
            ) {
                Ok(orig) => {
                    orig_slot.store(orig, Ordering::Release);
                    if GET_STATE_ORIG.load(Ordering::Acquire) == 0 {
                        GET_STATE_ORIG.store(orig, Ordering::Release);
                    }
                },
                Err(e) => {
                    error!("Failed to hook XInputGetState: {}", e);
                    orig_slot.store(proc_addr, Ordering::Release);
                    if GET_STATE_ORIG.load(Ordering::Acquire) == 0 {
                        GET_STATE_ORIG.store(proc_addr, Ordering::Release);
                    }
                },
            }
        }
    }
    let orig = GET_STATE_ORIG.load(Ordering::Acquire);
    if orig == 0 {
        None
    }
    else {
        Some(unsafe { std::mem::transmute(orig) })
    }
}