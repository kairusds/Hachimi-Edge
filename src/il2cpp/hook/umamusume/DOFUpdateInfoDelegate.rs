use crate::{
    core::free_camera,
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::PostEffectUpdateInfo_DOF;

type DofUpdateInfoDelegateInvokeFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
);

extern "C" fn DOFUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
) {
    if free_camera::should_remove_camera_effects() {
        PostEffectUpdateInfo_DOF::disable_if_dof(update_info);
    }

    get_orig_fn!(DOFUpdateInfoDelegate_Invoke, DofUpdateInfoDelegateInvokeFn)(this, update_info);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(dof_update_info_delegate) = get_class(umamusume, c"Gallop.Live.Cutt", c"DOFUpdateInfoDelegate") {
        let DOFUpdateInfoDelegate_Invoke_addr = get_method_addr(dof_update_info_delegate, c"Invoke", 1);
        new_hook!(DOFUpdateInfoDelegate_Invoke_addr, DOFUpdateInfoDelegate_Invoke);
    }
}
