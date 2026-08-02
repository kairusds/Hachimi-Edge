use crate::{
    windows::free_camera,
    il2cpp::{
        symbols::get_method_addr,
        types::*,
    },
};

use super::PostEffectUpdateInfo_DOF;

type DOFUpdateInfoDelegate_InvokeFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
);
extern "C" fn DOFUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
) {
    free_camera::set_live_active();
    if free_camera::should_remove_camera_effects() {
        PostEffectUpdateInfo_DOF::disable(update_info);
    }

    get_orig_fn!(DOFUpdateInfoDelegate_Invoke, DOFUpdateInfoDelegate_InvokeFn)(this, update_info);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live.Cutt", DOFUpdateInfoDelegate);

    let DOFUpdateInfoDelegate_Invoke_addr = get_method_addr(DOFUpdateInfoDelegate, c"Invoke", 1);
    new_hook!(DOFUpdateInfoDelegate_Invoke_addr, DOFUpdateInfoDelegate_Invoke);
}
