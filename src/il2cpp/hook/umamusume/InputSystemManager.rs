use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    windows::free_camera,
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

static GET_BUTTON_HOOK_ID: AtomicU8 = AtomicU8::new(1);
static GET_BUTTON_DOWN_HOOK_ID: AtomicU8 = AtomicU8::new(2);
static GET_BUTTON_UP_HOOK_ID: AtomicU8 = AtomicU8::new(3);
static GET_AXIS_HOOK_ID: AtomicU8 = AtomicU8::new(4);
static GET_VECTOR2_HOOK_ID: AtomicU8 = AtomicU8::new(5);
static KEYBOARD_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(6);
static GAMEPAD_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(7);
static ANY_KEY_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(8);

#[inline(always)]
fn preserve_hook_identity(identity: &AtomicU8) {
    std::hint::black_box(identity.load(Ordering::Relaxed));
}

type InputButtonFn = extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> bool;
macro_rules! block_input_button {
    ($hook:ident, $identity:ident) => {
        extern "C" fn $hook(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> bool {
            preserve_hook_identity(&$identity);
            if free_camera::is_game_input_capture_active() {
                false
            } else {
                get_orig_fn!($hook, InputButtonFn)(this, action_name)
            }
        }
    };
}

block_input_button!(GetButton, GET_BUTTON_HOOK_ID);
block_input_button!(GetButtonDown, GET_BUTTON_DOWN_HOOK_ID);
block_input_button!(GetButtonUp, GET_BUTTON_UP_HOOK_ID);

type GetAxisFn = extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> f32;
extern "C" fn GetAxis(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> f32 {
    preserve_hook_identity(&GET_AXIS_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        0.0
    } else {
        get_orig_fn!(GetAxis, GetAxisFn)(this, action_name)
    }
}

type GetVector2Fn = extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> Vector2_t;
extern "C" fn GetVector2(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> Vector2_t {
    preserve_hook_identity(&GET_VECTOR2_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        Vector2_t::default()
    } else {
        get_orig_fn!(GetVector2, GetVector2Fn)(this, action_name)
    }
}

type IsActionKeyTriggeredInKeyboardFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn IsActionKeyTriggeredInKeyboard(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&KEYBOARD_TRIGGER_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(IsActionKeyTriggeredInKeyboard, IsActionKeyTriggeredInKeyboardFn)(this)
    }
}

type IsActionButtonTriggeredInGamepadFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn IsActionButtonTriggeredInGamepad(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&GAMEPAD_TRIGGER_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(IsActionButtonTriggeredInGamepad, IsActionButtonTriggeredInGamepadFn)(this)
    }
}

type get_IsAnyKeyTriggeredInKeyboardFn = extern "C" fn() -> bool;
extern "C" fn get_IsAnyKeyTriggeredInKeyboard() -> bool {
    preserve_hook_identity(&ANY_KEY_TRIGGER_HOOK_ID);
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(get_IsAnyKeyTriggeredInKeyboard, get_IsAnyKeyTriggeredInKeyboardFn)()
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", InputSystemManager);

    let get_button_addr = get_method_addr(InputSystemManager, c"GetButton", 1);
    new_hook!(get_button_addr, GetButton);

    let get_button_down_addr = get_method_addr(InputSystemManager, c"GetButtonDown", 1);
    new_hook!(get_button_down_addr, GetButtonDown);

    let get_button_up_addr = get_method_addr(InputSystemManager, c"GetButtonUp", 1);
    new_hook!(get_button_up_addr, GetButtonUp);

    let get_axis_addr = get_method_addr(InputSystemManager, c"GetAxis", 1);
    new_hook!(get_axis_addr, GetAxis);

    let get_vector2_addr = get_method_addr(InputSystemManager, c"GetVector2", 1);
    new_hook!(get_vector2_addr, GetVector2);

    let is_action_key_triggered_addr =
        get_method_addr(InputSystemManager, c"IsActionKeyTriggeredInKeyboard", 0);
    new_hook!(is_action_key_triggered_addr, IsActionKeyTriggeredInKeyboard);

    let is_action_button_triggered_addr =
        get_method_addr(InputSystemManager, c"IsActionButtonTriggeredInGamepad", 0);
    new_hook!(
        is_action_button_triggered_addr,
        IsActionButtonTriggeredInGamepad
    );

    let is_any_key_triggered_addr =
        get_method_addr(InputSystemManager, c"get_IsAnyKeyTriggeredInKeyboard", 0);
    new_hook!(is_any_key_triggered_addr, get_IsAnyKeyTriggeredInKeyboard);
}
