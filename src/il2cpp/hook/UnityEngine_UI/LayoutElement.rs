use crate::il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, symbols::get_method_addr, types::*};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

// public virtual Void set_minHeight(Single value) { }
// public virtual Void set_flexibleHeight(Single value) { }
static mut SET_MINHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_minHeight, SET_MINHEIGHT_ADDR, (), this: *mut Il2CppObject, value: f32);

static mut SET_FLEXIBLEHEIGHT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_flexibleHeight, SET_FLEXIBLEHEIGHT_ADDR, (), this: *mut Il2CppObject, value: f32);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", LayoutElement);
    
    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(LayoutElement));
        SET_MINHEIGHT_ADDR = get_method_addr(LayoutElement, c"set_minHeight", 1);
        SET_FLEXIBLEHEIGHT_ADDR = get_method_addr(LayoutElement, c"set_flexibleHeight", 1);
    }
}
