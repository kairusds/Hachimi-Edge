use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

def_method_wrapper_fn!(get_assetBundle, GET_ASSET_BUNDLE_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

pub fn init(_UnityEngine_AssetBundleModule: *const Il2CppImage) {
    unsafe {
        GET_ASSET_BUNDLE_ADDR = il2cpp_resolve_icall(c"UnityEngine.AssetBundleCreateRequest::get_assetBundle()".as_ptr());
    }
}
