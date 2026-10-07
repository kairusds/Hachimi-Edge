use crate::{
    windows::free_camera,
    il2cpp::{
        symbols::{get_class, get_method_addr, SingletonLike},
        types::*,
    },
};

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

static mut UPDATE_INPUT_CONTROLS_ADDR: usize = 0;
impl_addr_wrapper_fn!(UpdateInputControls, UPDATE_INPUT_CONTROLS_ADDR, (), this: *mut Il2CppObject);

static mut CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR: usize = 0;
impl_addr_wrapper_fn!(CreateRenderTextureFromScreen, CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR, (), this: *mut Il2CppObject);

type CheckGamepadInputFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
extern "C" fn CheckGamepadInput(this: *mut Il2CppObject) -> bool {
    if free_camera::is_game_input_capture_active() {
        false
    } else {
        get_orig_fn!(CheckGamepadInput, CheckGamepadInputFn)(this)
    }
}

pub fn refresh_after_window_resize() {
    let this = instance();
    if this.is_null() {
        return;
    }
    CreateRenderTextureFromScreen(this);
    UpdateInputControls(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    let class = get_class(umamusume, c"Gallop", c"WindowsGamepadControl")
        .or_else(|_| get_class(umamusume, c"Gallop", c"SteamGamepadControl"));
    let Ok(class) = class else {
        warn!("WindowsGamepadControl class not found");
        return;
    };

    unsafe {
        CLASS = class;
        UPDATE_INPUT_CONTROLS_ADDR = get_method_addr(class, c"UpdateInputControls", 0);
        CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR = get_method_addr(class, c"CreateRenderTextureFromScreen", 0);
    }

    let check_gamepad_input_addr = get_method_addr(class, c"CheckGamepadInput", 0);
    new_hook!(check_gamepad_input_addr, CheckGamepadInput);
}
