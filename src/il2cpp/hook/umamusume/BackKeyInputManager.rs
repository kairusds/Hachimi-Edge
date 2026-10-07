use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
    windows::free_camera
};
use std::sync::atomic::{AtomicU8, Ordering};

static BACK_KEY_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(1);
static BACK_MOUSE_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(2);

#[inline(always)]
fn preserve_hook_identity(identity: &AtomicU8) {
    std::hint::black_box(identity.load(Ordering::Relaxed));
}

type IsTriggeredBackKeyFn = extern "C" fn() -> bool;
extern "C" fn IsTriggeredBackKey() -> bool {
    preserve_hook_identity(&BACK_KEY_TRIGGER_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(IsTriggeredBackKey, IsTriggeredBackKeyFn)()
    }
}

type BackMouseTriggeredFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn get_IsRightMouseButtonPressedForBack(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&BACK_MOUSE_TRIGGER_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(get_IsRightMouseButtonPressedForBack, BackMouseTriggeredFn)(this)
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", BackKeyInputManager);

    let is_triggered_back_key_addr = get_method_addr(BackKeyInputManager, c"IsTriggeredBackKey", 0);
    new_hook!(is_triggered_back_key_addr, IsTriggeredBackKey);

    let is_right_mouse_button_pressed_for_back_addr = get_method_addr(
        BackKeyInputManager,
        c"get_IsRightMouseButtonPressedForBack",
        0,
    );
    new_hook!(
        is_right_mouse_button_pressed_for_back_addr,
        get_IsRightMouseButtonPressedForBack
    );
}
