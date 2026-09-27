use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

def_method_wrapper_fn!(get_isDone, GET_IS_DONE_ADDR, bool, this: *mut Il2CppObject);

pub fn init(_UnityEngine_AssetBundleModule: *const Il2CppImage) {
    unsafe {
        GET_IS_DONE_ADDR = il2cpp_resolve_icall(c"UnityEngine.AsyncOperation::get_isDone()".as_ptr());
    }
}
