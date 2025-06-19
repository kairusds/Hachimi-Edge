use crate::il2cpp::{symbols::get_method_addr, types::*};

static mut LOADIMAGE_ADDR: usize = 0;
pub fn LoadImage(this_tex: *mut Il2CppObject, data: *mut Il2CppArray, mark_non_readable: bool) -> bool {
    let addr = unsafe { LOADIMAGE_ADDR };
    // not available in CN
    if addr == 0 {
        return false;
    }

    let orig_fn: extern "C" fn(
        this_tex: *mut Il2CppObject, data: *mut Il2CppArray, mark_non_readable: bool
    ) -> bool = unsafe { std::mem::transmute(addr) };
    orig_fn(this_tex, data, mark_non_readable)
}

pub fn init(UnityEngine_ImageConversionModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_ImageConversionModule, UnityEngine, ImageConversion);

    unsafe {
        LOADIMAGE_ADDR = get_method_addr(ImageConversion, c"LoadImage", 3);
    }
}