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

pub fn update_download_state(state: TBPFLAG) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_download {
            crate::windows::taskbar::set_progress_state(state);
        }
    }
}

pub fn update_download_value(completed: u64, total: u64) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_download {
            crate::windows::taskbar::set_progress_value(completed, total);
        }
    }
}

pub fn update_connecting_state(state: TBPFLAG) {
    #[cfg(target_os = "windows")]
    {
        if crate::core::Hachimi::instance().config.load().windows.taskbar_show_progress_on_connecting {
            crate::windows::taskbar::set_progress_state(state);
        }
    }
}