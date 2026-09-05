use crate::il2cpp::{symbols::get_method_addr, types::*};

def_method_wrapper_fn!(get_Size, GET_SIZE_ADDR, Vector2_t, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_OffsetPos, GET_OFFSETPOS_ADDR, Vector2_t, this: *mut Il2CppObject);

pub fn init(LandscapeUIManager: *mut Il2CppClass) {
    find_nested_class_or_return!(LandscapeUIManager, ScreenInfo);

    unsafe {
        GET_SIZE_ADDR = get_method_addr(ScreenInfo, c"get_Size", 0);
        GET_OFFSETPOS_ADDR = get_method_addr(ScreenInfo, c"get_OffsetPos", 0);
    }
}
