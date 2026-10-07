#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
pub use windows::Win32::UI::Shell::{TBPF_NOPROGRESS, TBPF_INDETERMINATE, TBPF_NORMAL, TBPF_ERROR, TBPFLAG};

#[cfg(not(target_os = "windows"))]
pub type TBPFLAG = u32;
#[cfg(not(target_os = "windows"))]
pub const TBPF_NOPROGRESS: TBPFLAG = 0;
#[cfg(not(target_os = "windows"))]
pub const TBPF_INDETERMINATE: TBPFLAG = 1;
#[cfg(not(target_os = "windows"))]
pub const TBPF_NORMAL: TBPFLAG = 2;
#[cfg(not(target_os = "windows"))]
pub const TBPF_ERROR: TBPFLAG = 4;

#[cfg(target_os = "windows")]
static BLOCK_CONNECTING_STATE: AtomicBool = AtomicBool::new(false);

pub fn update_download_state(_state: TBPFLAG) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_download {
            crate::windows::taskbar::set_progress_state(_state);
        }
    }
}

pub fn update_download_value(_completed: u64, _total: u64) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_download {
            crate::windows::taskbar::set_progress_value(_completed, _total);
        }
    }
}

pub fn update_connecting_state(_state: TBPFLAG) {
    #[cfg(target_os = "windows")]
    {
        let block = BLOCK_CONNECTING_STATE.load(Ordering::Relaxed);
        if !block && crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_connecting {
            crate::windows::taskbar::set_progress_state(_state);
        }
    }
}

pub fn set_connecting_state_block(_blocked: bool) {
    #[cfg(target_os = "windows")]
    {
        BLOCK_CONNECTING_STATE.store(_blocked, Ordering::Relaxed);
    }
}

pub fn update_schedule_state(_state: TBPFLAG, _block_connecting: bool) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_schedule_book {
            crate::windows::taskbar::set_progress_state(_state);
            set_connecting_state_block(_block_connecting);
        } else {
            set_connecting_state_block(false);
        }
    }
}

pub fn update_schedule_value(_completed: u64, _total: u64) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_schedule_book {
            crate::windows::taskbar::set_progress_value(_completed, _total);
        }
    }
}