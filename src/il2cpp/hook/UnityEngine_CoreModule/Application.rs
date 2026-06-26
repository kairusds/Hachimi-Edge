use std::sync::{atomic};

use crate::{core::Hachimi, il2cpp::{api::il2cpp_resolve_icall, symbols::get_method_addr, types::*}};

#[cfg(target_os = "windows")]
use crate::windows::external_link::open_url;

type SetTargetFrameRateFn = extern "C" fn(value: i32);
pub extern "C" fn set_targetFrameRate(mut value: i32) {
    let target_fps = Hachimi::instance().target_fps.load(atomic::Ordering::Relaxed);
    if target_fps != -1 {
        value = target_fps;
    }
    get_orig_fn!(set_targetFrameRate, SetTargetFrameRateFn)(value);
}
type OpenURLFn = extern "system" fn(il2cpp_url:*mut Il2CppString);

#[allow(non_snake_case)]
pub extern "system" fn OpenURL(url:*mut Il2CppString){
    #[cfg(target_os="windows")]
    {
        if !open_url::open(url) {
            get_orig_fn!(OpenURL,OpenURLFn)(url);
        }
    }
    #[cfg(not(target_os="windows"))]
    get_orig_fn!(OpenURL,OpenURLFn);
}


static mut GET_PERSISTENTDATAPATH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_persistentDataPath, GET_PERSISTENTDATAPATH_ADDR, *mut Il2CppString,);

// static mut OPENURL_ADDR: usize = 0;
// impl_addr_wrapper_fn!(OpenURL, OPENURL_ADDR, (), url: *mut Il2CppString);

static mut GET_SYSTEMLANGUAGE_ADDR: usize = 0;
impl_addr_wrapper_fn!(systemLanguage, GET_SYSTEMLANGUAGE_ADDR, i32, );

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Application);

    let set_targetFrameRate_addr = il2cpp_resolve_icall(
        c"UnityEngine.Application::set_targetFrameRate(System.Int32)".as_ptr()
    );
    new_hook!(set_targetFrameRate_addr, set_targetFrameRate);

    let openurl_addr= get_method_addr(Application, c"OpenURL", 1);
    new_hook!(openurl_addr, OpenURL);

    unsafe {
        GET_PERSISTENTDATAPATH_ADDR = get_method_addr(Application, c"get_persistentDataPath", 0);
        GET_SYSTEMLANGUAGE_ADDR = il2cpp_resolve_icall(c"UnityEngine.Application::get_systemLanguage()".as_ptr());
    }
}
