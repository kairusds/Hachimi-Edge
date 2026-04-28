use std::sync::Mutex;
use once_cell::sync::Lazy;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::{
    ITaskbarList3, TaskbarList, TBPFLAG, TBPF_NOPROGRESS, TBPF_NORMAL
};

struct TaskbarWrapper(ITaskbarList3);
unsafe impl Send for TaskbarWrapper {}
unsafe impl Sync for TaskbarWrapper {}

static TASKBAR_LIST: Lazy<Mutex<Option<TaskbarWrapper>>> = Lazy::new(|| Mutex::new(None));
static mut TASKBAR_HWND: HWND = HWND(std::ptr::null_mut());
static mut CURRENT_STATE: TBPFLAG = TBPF_NOPROGRESS;
static mut CURRENT_VALUE: u64 = 0;

pub fn init(hwnd: HWND) {
    unsafe {
        TASKBAR_HWND = hwnd;
        if let Ok(taskbar) = CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) {
            let _ = taskbar.SetProgressState(hwnd, TBPF_NOPROGRESS);
            *TASKBAR_LIST.lock().unwrap() = Some(TaskbarWrapper(taskbar));
        }
    }
}

pub fn set_progress_state(state: TBPFLAG) {
    unsafe {
        if CURRENT_STATE == state { return; }
        CURRENT_STATE = state;
        if let Some(wrapper) = TASKBAR_LIST.lock().unwrap().as_ref() {
            let _ = wrapper.0.SetProgressState(TASKBAR_HWND, state);
        }
    }
}

pub fn set_progress_value(completed: u64, total: u64) {
    unsafe {
        if CURRENT_VALUE == completed && CURRENT_STATE == TBPF_NORMAL { return; }
        CURRENT_VALUE = completed;
        CURRENT_STATE = TBPF_NORMAL;
        if let Some(wrapper) = TASKBAR_LIST.lock().unwrap().as_ref() {
            let _ = wrapper.0.SetProgressState(TASKBAR_HWND, TBPF_NORMAL);
            let _ = wrapper.0.SetProgressValue(TASKBAR_HWND, completed, total);
        }
    }
}