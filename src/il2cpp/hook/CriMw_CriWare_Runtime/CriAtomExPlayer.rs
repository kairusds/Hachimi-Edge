use crate::{
    core::{captions, live_utils},
    il2cpp::{
        symbols::{get_method_addr, get_method_overload_addr},
        types::*
    }
};

// public CriAtomExPlayback Start() { }
static mut START_ADDR: usize = 0;
impl_addr_wrapper_fn!(Start, START_ADDR, live_utils::CriAtomExPlayback, this: *mut Il2CppObject);

// public Void Stop(Boolean ignoresReleaseTime) { }
static mut STOP_ADDR: usize = 0;
impl_addr_wrapper_fn!(Stop, STOP_ADDR, (), this: *mut Il2CppObject, ignores_release_time: bool);

// public Void StopWithoutReleaseTime() { }
static mut STOPWITHOUTRELEASETIME_ADDR: usize = 0;
impl_addr_wrapper_fn!(StopWithoutReleaseTime, STOPWITHOUTRELEASETIME_ADDR, (), this: *mut Il2CppObject);

// public Void SetStartTime(Int64 startTimeMs) { }
static mut SETSTARTTIME_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetStartTime, SETSTARTTIME_ADDR, (), this: *mut Il2CppObject, start_time_ms: i64);

// public Void Update(CriAtomExPlayback playback) { }
static mut UPDATE_ADDR: usize = 0;
impl_addr_wrapper_fn!(Update, UPDATE_ADDR, (), this: *mut Il2CppObject, playback: live_utils::CriAtomExPlayback);

// public Void Pause(Boolean sw) { }
static mut PAUSE_ADDR: usize = 0;
impl_addr_wrapper_fn!(Pause, PAUSE_ADDR, (), this: *mut Il2CppObject, sw: bool);

// public Void Stop()
type StopHookFn = extern "C" fn(this: *mut Il2CppObject);
pub extern "C" fn StopHook(this: *mut Il2CppObject) {
    get_orig_fn!(StopHook, StopHookFn)(this);
    captions::Captions::cleanup();
}

// public void StopWithoutReleaseTime()
pub type StopWithoutReleaseTimeHookFn = extern "C" fn(this: *mut Il2CppObject);
pub extern "C" fn StopWithoutReleaseTimeHook(this: *mut Il2CppObject) {
    get_orig_fn!(StopWithoutReleaseTimeHook, StopWithoutReleaseTimeHookFn)(this);
    captions::Captions::cleanup();
}

// public Void Pause() { }
type PauseHookFn = extern "C" fn(this: *mut Il2CppObject, sw: bool);
pub extern "C" fn PauseHook(this: *mut Il2CppObject, sw: bool) {
    get_orig_fn!(PauseHook, PauseHookFn)(this, sw);
    if !sw {
        captions::Captions::cleanup();
    }
}

pub fn init(CriMw_CriWare_Runtime: *const Il2CppImage) {
    get_class_or_return!(CriMw_CriWare_Runtime, CriWare, CriAtomExPlayer);

    unsafe {
        STOP_ADDR = get_method_addr(CriAtomExPlayer, c"Stop", 1);
        STOPWITHOUTRELEASETIME_ADDR = get_method_addr(CriAtomExPlayer, c"StopWithoutReleaseTime", 0);
        START_ADDR = get_method_addr(CriAtomExPlayer, c"Start", 0);
        PAUSE_ADDR = get_method_addr(CriAtomExPlayer, c"Pause", 1);
        SETSTARTTIME_ADDR = get_method_addr(CriAtomExPlayer, c"SetStartTime", 1);
        UPDATE_ADDR = get_method_addr(CriAtomExPlayer, c"Update", 1);
    }

    let stop_addr = get_method_addr(CriAtomExPlayer, c"Stop", 0);
    new_hook!(stop_addr, StopHook);

    let stop_without_release_time_addr = get_method_addr(CriAtomExPlayer, c"StopWithoutReleaseTime", 0);
    new_hook!(stop_without_release_time_addr, StopWithoutReleaseTimeHook);

    let pause_addr = get_method_overload_addr(CriAtomExPlayer, "Pause", &[Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN]);
    new_hook!(pause_addr, PauseHook);
}
