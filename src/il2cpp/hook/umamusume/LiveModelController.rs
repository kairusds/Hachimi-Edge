use crate::il2cpp::{
    symbols::get_method_addr,
    types::*,
};

static mut GET_HEAD_TRANSFORM_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_HeadTransform, GET_HEAD_TRANSFORM_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut SET_MESH_ACTIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetMeshActive, SET_MESH_ACTIVE_ADDR, (), this: *mut Il2CppObject, is_active: bool);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", LiveModelController);

    unsafe {
        GET_HEAD_TRANSFORM_ADDR = get_method_addr(LiveModelController, c"get_HeadTransform", 0);
        SET_MESH_ACTIVE_ADDR = get_method_addr(LiveModelController, c"SetMeshActive", 1);
    }
}
