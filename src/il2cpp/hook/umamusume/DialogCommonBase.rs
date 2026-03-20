use crate::{
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

// protected RectTransform get_ContentsRoot() { }
static mut GET_CONTENTSROOT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_ContentsRoot, GET_CONTENTSROOT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogCommonBase);

    unsafe {
        GET_CONTENTSROOT_ADDR = get_method_addr(DialogCommonBase, c"get_ContentsRoot", 0);
    }
}