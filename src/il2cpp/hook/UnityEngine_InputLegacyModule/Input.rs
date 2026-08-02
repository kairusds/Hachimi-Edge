use crate::{
    il2cpp::{symbols::get_method_addr, types::*}
};

static mut GET_MOUSE_POSITION_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_mousePosition, GET_MOUSE_POSITION_ADDR, Vector3_t,);

static mut SET_IME_COMPOSITION_MODE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_imeCompositionMode, SET_IME_COMPOSITION_MODE_ADDR, (), value: i32);

static mut SET_COMPOSITION_CURSOR_POS_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_compositionCursorPos, SET_COMPOSITION_CURSOR_POS_ADDR, (), value: Vector2_t);

pub fn init(image: *const crate::il2cpp::types::Il2CppImage) {
    get_class_or_return!(image, UnityEngine, Input);

    unsafe {
        GET_MOUSE_POSITION_ADDR = get_method_addr(Input, c"get_mousePosition", 0);
        SET_IME_COMPOSITION_MODE_ADDR = get_method_addr(Input, c"set_imeCompositionMode", 1);
        SET_COMPOSITION_CURSOR_POS_ADDR = get_method_addr(Input, c"set_compositionCursorPos", 1);
    }
}
