use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GETSKILLNAME_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetSkillName, GETSKILLNAME_ADDR, *mut Il2CppString, skillId: i32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterDataUtil);
    
    unsafe {
        GETSKILLNAME_ADDR = get_method_addr(MasterDataUtil, c"GetSkillName", 1);
    }
}
