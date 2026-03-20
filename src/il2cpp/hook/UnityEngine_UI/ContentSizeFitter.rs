use crate::il2cpp::{api::{il2cpp_class_get_type, il2cpp_type_get_object}, symbols::get_method_addr, types::*};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut SET_VERTICALFIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_verticalFit, SET_VERTICALFIT_ADDR, (), this: *mut Il2CppObject, value: i32);

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.UI", ContentSizeFitter);
    
    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(ContentSizeFitter));
        SET_VERTICALFIT_ADDR = get_method_addr(ContentSizeFitter, c"set_verticalFit", 1);
    }
}
