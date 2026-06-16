use crate::{
    il2cpp::{api::il2cpp_resolve_icall, symbols::get_method_addr, types::*}
};

static mut GET_MOUSE_POSITION_INJECTED_ADDR: usize = 0;
pub fn mouse_position() -> Vector3_t {
    let mut value = Vector3_t { x: 0.0, y: 0.0, z: 0.0 };
    let get_mouse_position: extern "C" fn(value: *mut Vector3_t) = unsafe {
        std::mem::transmute(GET_MOUSE_POSITION_INJECTED_ADDR)
    };
    get_mouse_position(&mut value);
    value
}

static mut SET_IME_COMPOSITION_MODE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_imeCompositionMode, SET_IME_COMPOSITION_MODE_ADDR, (), value: i32);

static mut SET_COMPOSITION_CURSOR_POS_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_compositionCursorPos, SET_COMPOSITION_CURSOR_POS_ADDR, (), value: Vector2_t);

pub fn init(image: *const crate::il2cpp::types::Il2CppImage) {
    get_class_or_return!(image, UnityEngine, Input);

    unsafe {
        GET_MOUSE_POSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Input::get_mousePosition_Injected()".as_ptr());
        SET_IME_COMPOSITION_MODE_ADDR = get_method_addr(Input, c"set_imeCompositionMode", 1);
        SET_COMPOSITION_CURSOR_POS_ADDR = get_method_addr(Input, c"set_compositionCursorPos", 1);
    }
}
