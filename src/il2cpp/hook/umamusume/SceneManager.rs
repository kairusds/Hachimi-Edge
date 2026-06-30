use std::sync::atomic::{self, AtomicBool};

use crate::{
    core::{Hachimi, game::Region},
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr, SingletonLike},
        types::*
    }
};

static SPLASH_SHOWN: AtomicBool = AtomicBool::new(false);
pub fn is_splash_shown() -> bool {
    SPLASH_SHOWN.load(atomic::Ordering::Acquire)
}

static HOME_INIT: AtomicBool = AtomicBool::new(false);
pub fn is_home_init() -> bool {
    HOME_INIT.load(atomic::Ordering::Acquire)
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

def_field_object_accessors!(get_PhotoCheckObject, set_PhotoCheckObject, PHOTOCHECKOBJECT_FIELD, *mut Il2CppObject);
def_field_object_accessors!(get_PhotoLibraryObject, set_PhotoLibraryObject, PHOTOLIBRARYOBJECT_FIELD, *mut Il2CppObject);

static mut GETCURRENTVIEWID_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetCurrentViewId, GETCURRENTVIEWID_ADDR, i32, this: *mut Il2CppObject);

static mut GETCURRENTVIEWCONTROLLER_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetCurrentViewController, GETCURRENTVIEWCONTROLLER_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

fn ChangeViewCommon(next_view_id: i32) {
    if next_view_id == 1 { // ViewId.Splash
        SPLASH_SHOWN.store(true, atomic::Ordering::Release);
    }
    if next_view_id == 100 && !HOME_INIT.swap(true, atomic::Ordering::AcqRel) { // ViewId.Home
        #[cfg(target_os = "windows")]
        {
            use crate::windows::{smtc, wnd_hook::get_target_hwnd};
            if Hachimi::instance().config.load().windows.enable_smtc {
                smtc::init(get_target_hwnd());
            }
        }
        info!("HOME_INIT: {}", is_home_init());
    }
}

type ChangeViewJpfn = extern "C" fn(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool, is_fast_destroy: bool, fade_in_duration: f32
);
extern "C" fn ChangeViewJp(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool, is_fast_destroy: bool, fade_in_duration: f32
) {
    get_orig_fn!(ChangeViewJp, ChangeViewJpfn)(
        this, next_view_id, view_info, callback_on_change_view_cancel,
        callback_on_change_view_accept, force_change, is_fast_destroy,
        fade_in_duration
    );
    ChangeViewCommon(next_view_id);
}

type ChangeViewOtherfn = extern "C" fn(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool
);
extern "C" fn ChangeViewOther(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool
) {
    get_orig_fn!(ChangeViewOther, ChangeViewOtherfn)(
        this, next_view_id, view_info, callback_on_change_view_cancel,
        callback_on_change_view_accept, force_change
    );
    ChangeViewCommon(next_view_id);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, SceneManager);

    unsafe {
        CLASS = SceneManager;
        GETCURRENTVIEWID_ADDR = get_method_addr(SceneManager, c"GetCurrentViewId", 0);
        PHOTOCHECKOBJECT_FIELD = get_field_from_name(SceneManager, c"PhotoCheckObject");
        PHOTOLIBRARYOBJECT_FIELD = get_field_from_name(SceneManager, c"PhotoLibraryObject");

        let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
        loop {
            let method = crate::il2cpp::api::il2cpp_class_get_methods(SceneManager, &mut iter);
            if method.is_null() { break; }
            let name = std::ffi::CStr::from_ptr((*method).name).to_string_lossy();
            if name == "GetCurrentViewController" && (*method).is_generic() == 0 {
                GETCURRENTVIEWCONTROLLER_ADDR = (*method).methodPointer;
                break;
            }
        }

        if GETCURRENTVIEWCONTROLLER_ADDR == 0 {
            error!("Failed to find non-generic GetCurrentViewController on SceneManager");
        }
    }

    if Hachimi::instance().game.region == Region::Japan {
        let ChangeView_addr = get_method_addr(SceneManager, c"ChangeView", 7);
        new_hook!(ChangeView_addr, ChangeViewJp);
    }
    else {
        let ChangeView_addr = get_method_addr(SceneManager, c"ChangeView", 5);
        new_hook!(ChangeView_addr, ChangeViewOther);
    }
}
