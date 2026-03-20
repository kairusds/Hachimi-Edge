use std::sync::atomic::{self, AtomicBool};

use crate::{core::{Hachimi, game::Region}, il2cpp::{symbols::get_method_addr, types::*}};

static SPLASH_SHOWN: AtomicBool = AtomicBool::new(false);
pub fn is_splash_shown() -> bool {
    SPLASH_SHOWN.load(atomic::Ordering::Acquire)
}

fn ChangeViewCommon(next_view_id: i32) {
    if next_view_id == 1 { // ViewId.Splash
        SPLASH_SHOWN.store(true, atomic::Ordering::Release);
    }
}

type ChangeViewJpfn = extern "C" fn(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool, is_fast_destroy: bool
);
extern "C" fn ChangeViewJp(
    this: *mut Il2CppObject, next_view_id: i32, view_info: *mut Il2CppObject,
    callback_on_change_view_cancel: *mut Il2CppObject, callback_on_change_view_accept: *mut Il2CppObject,
    force_change: bool, is_fast_destroy: bool
) {
    get_orig_fn!(ChangeViewJp, ChangeViewJpfn)(
        this, next_view_id, view_info, callback_on_change_view_cancel,
        callback_on_change_view_accept, force_change, is_fast_destroy
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

    if Hachimi::instance().game.region == Region::Japan {
        let ChangeView_addr = get_method_addr(SceneManager, c"ChangeView", 6);
        new_hook!(ChangeView_addr, ChangeViewJp);
    }
    else {
        let ChangeView_addr = get_method_addr(SceneManager, c"ChangeView", 5);
        new_hook!(ChangeView_addr, ChangeViewOther);
    }
}