use crate::il2cpp::{
    symbols::get_method_addr,
    types::*,
};

static mut GET_OWNER_OBJECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_OwnerObject, GET_OWNER_OBJECT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_VISIBLE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetVisible, SET_VISIBLE_ADDR, (), this: *mut Il2CppObject, visible: bool, force: bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", ModelController);

    unsafe {
        GET_OWNER_OBJECT_ADDR = get_method_addr(ModelController, c"get_OwnerObject", 0);
        SET_VISIBLE_ADDR = get_method_addr(ModelController, c"SetVisible", 2);
    }
}
