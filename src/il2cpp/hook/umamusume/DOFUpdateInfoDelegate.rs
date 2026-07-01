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
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
);
type MultiCameraDofUpdateInfoDelegateInvokeFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
    multi_camera_no: i32,
);

extern "C" fn DOFUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
) {
    free_camera::set_live_active();
    if free_camera::should_remove_camera_effects() {
        PostEffectUpdateInfo_DOF::disable(update_info);
    }

    get_orig_fn!(DOFUpdateInfoDelegate_Invoke, DofUpdateInfoDelegateInvokeFn)(this, update_info);
}

extern "C" fn MultiCameraDOFUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
    multi_camera_no: i32,
) {
    free_camera::set_live_active();
    if free_camera::should_remove_camera_effects() {
        PostEffectUpdateInfo_DOF::disable(update_info);
    }

    get_orig_fn!(
        MultiCameraDOFUpdateInfoDelegate_Invoke,
        MultiCameraDofUpdateInfoDelegateInvokeFn
    )(this, update_info, multi_camera_no);
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(dof_update_info_delegate) =
        get_class(umamusume, c"Gallop.Live.Cutt", c"DOFUpdateInfoDelegate")
    {
        let DOFUpdateInfoDelegate_Invoke_addr =
            get_method_addr(dof_update_info_delegate, c"Invoke", 1);
        new_hook!(
            DOFUpdateInfoDelegate_Invoke_addr,
            DOFUpdateInfoDelegate_Invoke
        );
    }

    if let Ok(multi_camera_dof_update_info_delegate) = get_class(
        umamusume,
        c"Gallop.Live.Cutt",
        c"MultiCameraDOFUpdateInfoDelegate",
    ) {
        let MultiCameraDOFUpdateInfoDelegate_Invoke_addr =
            get_method_addr(multi_camera_dof_update_info_delegate, c"Invoke", 2);
        new_hook!(
            MultiCameraDOFUpdateInfoDelegate_Invoke_addr,
            MultiCameraDOFUpdateInfoDelegate_Invoke
        );
    }
}
