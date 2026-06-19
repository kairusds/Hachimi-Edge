use crate::il2cpp::{
    symbols::{get_method_addr, get_type_object_for_class},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut GET_ISPLAYING_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsPlaying, GET_ISPLAYING_ADDR, bool, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsCharaMessageBase);

    unsafe {
        CLASS = PartsCharaMessageBase;
        TYPE_OBJECT = get_type_object_for_class(PartsCharaMessageBase);
        GET_ISPLAYING_ADDR = get_method_addr(PartsCharaMessageBase, c"get_IsPlaying", 0);
    }
}
