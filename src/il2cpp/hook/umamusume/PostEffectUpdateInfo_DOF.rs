use crate::il2cpp::types::*;

#[repr(C)]
pub struct PostEffectUpdateInfoDOF {
    pub IsEnableDOF: bool,
}

pub fn disable(update_info: *mut PostEffectUpdateInfoDOF) {
    if let Some(update_info) = unsafe { update_info.as_mut() } {
        update_info.IsEnableDOF = false;
    }
}

pub fn init(_umamusume: *const Il2CppImage) {}
