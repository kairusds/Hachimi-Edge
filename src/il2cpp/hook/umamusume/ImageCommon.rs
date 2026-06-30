use crate::{
    il2cpp::{
        symbols::{get_method_addr, get_type_object_for_class},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut SETALPHA_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetAlpha, SETALPHA_ADDR, (), this: *mut Il2CppObject, alpha: f32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, ImageCommon);

    unsafe {
        CLASS = ImageCommon;
        TYPE_OBJECT = get_type_object_for_class(ImageCommon);
        SETALPHA_ADDR = get_method_addr(ImageCommon, c"SetAlpha", 1);
    }
}
