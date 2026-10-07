use crate::il2cpp::{
    hook::UnityEngine_CoreModule::Shader,
    types::*
};

pub const SSAO_SHADER_NAME: &str = "Hidden/Universal Render Pipeline/ScreenSpaceAmbientOcclusion";

pub fn init(_Unity_RenderPipelines_Universal_Runtime: *const Il2CppImage) {
    // get_class_or_return!(Unity_RenderPipelines_Universal_Runtime, "UnityEngine.Rendering.Universal", ScreenSpaceAmbientOcclusion);

    let shader = Shader::find(SSAO_SHADER_NAME);
    if shader.is_null() {
        warn!("SSAO shader \"{}\" not found in the build", SSAO_SHADER_NAME);
    } else {
        info!("SSAO shader \"{}\" found in the build", SSAO_SHADER_NAME);
    }
}
