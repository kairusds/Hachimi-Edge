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

fn should_block_game_input() -> bool {
    free_camera::is_enabled()
}

macro_rules! block_input_button {
    ($hook:ident) => {
        extern "C" fn $hook(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> bool {
            if should_block_game_input() {
                false
            } else {
                get_orig_fn!($hook, InputButtonFn)(this, action_name)
            }
        }
    };
}

block_input_button!(GetButton);
block_input_button!(GetButtonDown);
block_input_button!(GetButtonUp);

extern "C" fn GetAxis(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> f32 {
    if should_block_game_input() {
        0.0
    } else {
        get_orig_fn!(GetAxis, InputAxisFn)(this, action_name)
    }
}

extern "C" fn GetVector2(this: *mut Il2CppObject, action_name: *mut Il2CppString) -> Vector2_t {
    if should_block_game_input() {
        Vector2_t::default()
    } else {
        get_orig_fn!(GetVector2, InputVector2Fn)(this, action_name)
    }
}

extern "C" fn IsActionKeyTriggeredInKeyboard(this: *mut Il2CppObject) -> bool {
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsActionKeyTriggeredInKeyboard, InputTriggeredFn)(this)
    }
}

extern "C" fn IsActionButtonTriggeredInGamepad(this: *mut Il2CppObject) -> bool {
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsActionButtonTriggeredInGamepad, InputTriggeredFn)(this)
    }
}

extern "C" fn get_IsAnyKeyTriggeredInKeyboard() -> bool {
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(get_IsAnyKeyTriggeredInKeyboard, StaticInputTriggeredFn)()
    }
}

extern "C" fn IsTriggeredBackKey() -> bool {
    if should_block_game_input() {
        false
    } else {
        get_orig_fn!(IsTriggeredBackKey, BackKeyTriggeredFn)()
    }
}

extern "C" fn get_IsRightMouseButtonPressedForBack(this: *mut Il2CppObject) -> bool {
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
