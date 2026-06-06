use crate::il2cpp::{
    api::{il2cpp_class_get_type, il2cpp_type_get_object},
    symbols::get_method_addr,
    types::*,
};

#[repr(i32)]
pub enum Axis {
    Horizontal = 0,
    Vertical = 1,
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut SET_SIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetSizeWithCurrentAnchors, SET_SIZE_ADDR, (), this: *mut Il2CppObject, axis: Axis, size: f32);

static mut GET_RECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rect, GET_RECT_ADDR, Rect_t, this: *mut Il2CppObject);

static mut GET_OFFSET_MIN_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_offsetMin, GET_OFFSET_MIN_ADDR, Vector2_t, this: *mut Il2CppObject);

static mut SET_OFFSET_MIN_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_offsetMin, SET_OFFSET_MIN_ADDR, (), this: *mut Il2CppObject, value: Vector2_t);

static mut GET_OFFSET_MAX_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_offsetMax, GET_OFFSET_MAX_ADDR, Vector2_t, this: *mut Il2CppObject);

static mut SET_OFFSET_MAX_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_offsetMax, SET_OFFSET_MAX_ADDR, (), this: *mut Il2CppObject, value: Vector2_t);

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, RectTransform);
    unsafe {
        CLASS = RectTransform;
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(RectTransform));
        SET_SIZE_ADDR = get_method_addr(RectTransform, c"SetSizeWithCurrentAnchors", 2);

        GET_RECT_ADDR = get_method_addr(RectTransform, c"get_rect", 0);
        GET_OFFSET_MIN_ADDR = get_method_addr(RectTransform, c"get_offsetMin", 0);
        SET_OFFSET_MIN_ADDR = get_method_addr(RectTransform, c"set_offsetMin", 1);
        GET_OFFSET_MAX_ADDR = get_method_addr(RectTransform, c"get_offsetMax", 0);
        SET_OFFSET_MAX_ADDR = get_method_addr(RectTransform, c"set_offsetMax", 1);
    }
}
