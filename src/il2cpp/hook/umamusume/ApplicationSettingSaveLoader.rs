use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{symbols::get_method_addr, types::*}
};

static mut GET_ISTRYRACEDYNAMICCAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsTryRaceDynamicCamera, GET_ISTRYRACEDYNAMICCAMERA_ADDR, bool, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, ApplicationSettingSaveLoader);

    unsafe {
        GET_ISTRYRACEDYNAMICCAMERA_ADDR = get_method_addr(ApplicationSettingSaveLoader, c"get_IsTryRaceDynamicCamera", 0);
    }
}