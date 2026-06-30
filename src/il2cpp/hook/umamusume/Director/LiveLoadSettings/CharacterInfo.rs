use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

static mut GET_CHARAID_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CharaId, GET_CHARAID_ADDR, i32, this: *mut Il2CppObject);

static mut GET_MOBID_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_MobId, GET_MOBID_ADDR, i32, this: *mut Il2CppObject);

pub fn init(LiveLoadSettings: *mut Il2CppClass) {
    find_nested_class_or_return!(LiveLoadSettings, CharacterInfo);

    unsafe {
        GET_CHARAID_ADDR = get_method_addr(CharacterInfo, c"get_CharaId", 0);
        GET_MOBID_ADDR = get_method_addr(CharacterInfo, c"get_MobId", 0);
    }
}
