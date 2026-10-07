use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut GET_BOOT_PROGRESS_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetBootProgress, GET_BOOT_PROGRESS_ADDR, f32,);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MainGameInitializer);

    unsafe {
        GET_BOOT_PROGRESS_ADDR = get_method_addr(MainGameInitializer, c"GetBootProgress", 0);
    }
}
