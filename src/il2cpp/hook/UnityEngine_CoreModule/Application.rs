use std::sync::{atomic};

use crate::{core::Hachimi, il2cpp::{api::il2cpp_resolve_icall, symbols::get_method_addr, types::*}};

type SetTargetFrameRateFn = extern "C" fn(value: i32);
pub extern "C" fn set_targetFrameRate(mut value: i32) {
    #[cfg(target_os = "windows")]
    LAST_GAME_FPS.store(value, atomic::Ordering::Relaxed);

    let hachimi = Hachimi::instance();
    let target_fps = hachimi.target_fps.load(atomic::Ordering::Relaxed);
    if target_fps != -1 {
        value = target_fps;
    }
    #[cfg(target_os = "windows")]
    {
        let unfocused_fps = hachimi.target_fps_unfocused.load(atomic::Ordering::Relaxed);
        if unfocused_fps != -1 && crate::windows::wnd_hook::window_unfocused() {
            value = unfocused_fps;
        }
    }
    get_orig_fn!(set_targetFrameRate, SetTargetFrameRateFn)(value);
}

#[cfg(target_os = "windows")]
static LAST_GAME_FPS: atomic::AtomicI32 = atomic::AtomicI32::new(-1);

#[cfg(target_os = "windows")]
pub fn current_effective_frame_rate() -> i32 {
    let hachimi = Hachimi::instance();
    let target_fps = hachimi.target_fps.load(atomic::Ordering::Relaxed);
    let unfocused_fps = hachimi.target_fps_unfocused.load(atomic::Ordering::Relaxed);
    if unfocused_fps != -1 && crate::windows::wnd_hook::window_unfocused() {
        unfocused_fps
    } else if target_fps != -1 {
        target_fps
    } else {
        LAST_GAME_FPS.load(atomic::Ordering::Relaxed)
    }
}

#[cfg(target_os = "windows")]
pub fn poke_target_frame_rate() {
    let value = current_effective_frame_rate();
    set_targetFrameRate(value);
}

#[cfg(target_os = "windows")]
type OpenURLFn = extern "C" fn(il2cpp_url:*mut Il2CppString);
#[cfg(target_os = "windows")]
pub extern "C" fn OpenURL(url: *mut Il2CppString){
    if !crate::windows::webview::open(url){
        get_orig_fn!(OpenURL, OpenURLFn)(url);
    }
}

static mut GET_PERSISTENTDATAPATH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_persistentDataPath, GET_PERSISTENTDATAPATH_ADDR, *mut Il2CppString,);

#[cfg(target_os = "android")]
static mut OPENURL_ADDR: usize = 0;
#[cfg(target_os = "android")]
impl_addr_wrapper_fn!(OpenURL, OPENURL_ADDR, (), url: *mut Il2CppString);

static mut GET_SYSTEMLANGUAGE_ADDR: usize = 0;
impl_addr_wrapper_fn!(systemLanguage, GET_SYSTEMLANGUAGE_ADDR, i32, );

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Application);

    let set_targetFrameRate_addr = il2cpp_resolve_icall(
        c"UnityEngine.Application::set_targetFrameRate(System.Int32)".as_ptr()
    );
    new_hook!(set_targetFrameRate_addr, set_targetFrameRate);

    #[cfg(target_os = "windows")]
    {
        let openurl_addr = get_method_addr(Application, c"OpenURL", 1);
        new_hook!(openurl_addr, OpenURL);
    }

    unsafe {
        GET_PERSISTENTDATAPATH_ADDR = get_method_addr(Application, c"get_persistentDataPath", 0);
        #[cfg(target_os = "android")]
        {
            OPENURL_ADDR = get_method_addr(Application, c"OpenURL", 1);
        }
        GET_SYSTEMLANGUAGE_ADDR = il2cpp_resolve_icall(c"UnityEngine.Application::get_systemLanguage()".as_ptr());
    }
}
