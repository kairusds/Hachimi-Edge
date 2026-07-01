use std::ptr::null_mut;

use crate::il2cpp::{
    symbols::{get_class, get_method_addr, SingletonLike},
    types::*,
};

static mut CLASS: *mut Il2CppClass = null_mut();
static mut UPDATE_INPUT_CONTROLS_ADDR: usize = 0;
static mut CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR: usize = 0;

pub fn update_input_controls() {
    let class = unsafe { CLASS };
    if class.is_null() || unsafe { UPDATE_INPUT_CONTROLS_ADDR } == 0 {
        return;
    }

    let Some(singleton) = SingletonLike::new(class) else {
        return;
    };
    let instance = singleton.instance();
    if instance.is_null() {
        return;
    }

    let update: extern "C" fn(*mut Il2CppObject) = unsafe { std::mem::transmute(UPDATE_INPUT_CONTROLS_ADDR) };
    update(instance);
}

pub fn refresh_after_window_resize() {
    let class = unsafe { CLASS };
    if class.is_null() {
        return;
    }

    let Some(singleton) = SingletonLike::new(class) else {
        return;
    };
    let instance = singleton.instance();
    if instance.is_null() {
        return;
    }

    if unsafe { CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR } != 0 {
        let create_render_texture: extern "C" fn(*mut Il2CppObject) = unsafe {
            std::mem::transmute(CREATE_RENDER_TEXTURE_FROM_SCREEN_ADDR)
        };
        create_render_texture(instance);
    }

    update_input_controls();
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
}
