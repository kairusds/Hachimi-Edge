use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
    core::free_camera,
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

type InputButtonFn = extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> bool;
type InputAxisFn = extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> f32;
type InputVector2Fn =
    extern "C" fn(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> Vector2_t;
type InputTriggeredFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
type StaticInputTriggeredFn = extern "C" fn() -> bool;
type BackKeyTriggeredFn = extern "C" fn() -> bool;
type BackMouseTriggeredFn = extern "C" fn(this: *mut Il2CppObject) -> bool;

static GET_BUTTON_HOOK_ID: AtomicU8 = AtomicU8::new(1);
static GET_BUTTON_DOWN_HOOK_ID: AtomicU8 = AtomicU8::new(2);
static GET_BUTTON_UP_HOOK_ID: AtomicU8 = AtomicU8::new(3);
static GET_AXIS_HOOK_ID: AtomicU8 = AtomicU8::new(4);
static GET_VECTOR2_HOOK_ID: AtomicU8 = AtomicU8::new(5);
static KEYBOARD_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(6);
static GAMEPAD_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(7);
static ANY_KEY_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(8);
static BACK_KEY_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(9);
static BACK_MOUSE_TRIGGER_HOOK_ID: AtomicU8 = AtomicU8::new(10);

#[inline(always)]
fn preserve_hook_identity(identity: &AtomicU8) {
    std::hint::black_box(identity.load(Ordering::Relaxed));
}

fn should_block_game_input() -> bool {
    free_camera::is_game_input_capture_active()
}

macro_rules! block_input_button {
    ($hook:ident, $identity:ident) => {
        extern "C" fn $hook(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> bool {
            preserve_hook_identity(&$identity);
            if should_block_game_input() {
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

extern "C" fn GetAxis(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> f32 {
    preserve_hook_identity(&GET_AXIS_HOOK_ID);
    if should_block_game_input() {
        0.0
    } else {
        get_orig_fn!(GetAxis, InputAxisFn)(this, action_name)
    }
}

extern "C" fn GetVector2(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> Vector2_t {
    preserve_hook_identity(&GET_VECTOR2_HOOK_ID);
    if should_block_game_input() {
        Vector2_t::default()
    } else {
        get_orig_fn!(GetVector2, InputVector2Fn)(this, action_name)
    }
}

extern "C" fn IsActionKeyTriggeredInKeyboard(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&KEYBOARD_TRIGGER_HOOK_ID);
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsActionKeyTriggeredInKeyboard, InputTriggeredFn)(this)
    }
}

extern "C" fn IsActionButtonTriggeredInGamepad(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&GAMEPAD_TRIGGER_HOOK_ID);
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsActionButtonTriggeredInGamepad, InputTriggeredFn)(this)
    }
}

extern "C" fn get_IsAnyKeyTriggeredInKeyboard() -> bool {
    preserve_hook_identity(&ANY_KEY_TRIGGER_HOOK_ID);
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(get_IsAnyKeyTriggeredInKeyboard, StaticInputTriggeredFn)()
    }
}

extern "C" fn IsTriggeredBackKey() -> bool {
    preserve_hook_identity(&BACK_KEY_TRIGGER_HOOK_ID);
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsTriggeredBackKey, BackKeyTriggeredFn)()
    }
}

extern "C" fn get_IsRightMouseButtonPressedForBack(this: *mut Il2CppObject) -> bool {
    preserve_hook_identity(&BACK_MOUSE_TRIGGER_HOOK_ID);
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(get_IsRightMouseButtonPressedForBack, BackMouseTriggeredFn)(this)
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(input_system_manager) = get_class(umamusume, c"Gallop", c"InputSystemManager") {
        let get_button_addr = get_method_addr(input_system_manager, c"GetButton", 1);
        let get_button_down_addr = get_method_addr(input_system_manager, c"GetButtonDown", 1);
        let get_button_up_addr = get_method_addr(input_system_manager, c"GetButtonUp", 1);
        let get_axis_addr = get_method_addr(input_system_manager, c"GetAxis", 1);
        let get_vector2_addr = get_method_addr(input_system_manager, c"GetVector2", 1);
        let is_action_key_triggered_addr =
            get_method_addr(input_system_manager, c"IsActionKeyTriggeredInKeyboard", 0);
        let is_action_button_triggered_addr =
            get_method_addr(input_system_manager, c"IsActionButtonTriggeredInGamepad", 0);
        let is_any_key_triggered_addr =
            get_method_addr(input_system_manager, c"get_IsAnyKeyTriggeredInKeyboard", 0);

        new_hook!(get_button_addr, GetButton);
        new_hook!(get_button_down_addr, GetButtonDown);
        new_hook!(get_button_up_addr, GetButtonUp);
        new_hook!(get_axis_addr, GetAxis);
        new_hook!(get_vector2_addr, GetVector2);
        new_hook!(is_action_key_triggered_addr, IsActionKeyTriggeredInKeyboard);
        new_hook!(
            is_action_button_triggered_addr,
            IsActionButtonTriggeredInGamepad
        );
        new_hook!(is_any_key_triggered_addr, get_IsAnyKeyTriggeredInKeyboard);
    } else {
        warn!("InputSystemManager class not found");
    }

    if let Ok(back_key_input_manager) = get_class(umamusume, c"Gallop", c"BackKeyInputManager") {
        let is_triggered_back_key_addr =
            get_method_addr(back_key_input_manager, c"IsTriggeredBackKey", 0);
        let is_right_mouse_button_pressed_for_back_addr = get_method_addr(
            back_key_input_manager,
            c"get_IsRightMouseButtonPressedForBack",
            0,
        );
        new_hook!(is_triggered_back_key_addr, IsTriggeredBackKey);
        new_hook!(
            is_right_mouse_button_pressed_for_back_addr,
            get_IsRightMouseButtonPressedForBack
        );
    }
}
