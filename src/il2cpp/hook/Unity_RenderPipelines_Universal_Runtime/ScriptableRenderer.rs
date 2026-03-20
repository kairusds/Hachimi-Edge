use crate::{
    core::Hachimi,
    il2cpp::{
        symbols::get_method_addr, types::*,
        hook::umamusume::GraphicSettings::MsaaQuality
    }
};

// el magico
type GetRenderTextureDescriptorFn = extern "C" fn(cameraData: *mut isize,  renderPass: *mut Il2CppObject, targetRT: *mut RenderTextureDescriptor);
extern "C" fn GetRenderTextureDescriptor(cameraData: *mut isize, renderPass: *mut Il2CppObject, targetRT: *mut RenderTextureDescriptor) {
    get_orig_fn!(GetRenderTextureDescriptor, GetRenderTextureDescriptorFn)(cameraData, renderPass, targetRT);

    let msaa = Hachimi::instance().config.load().msaa;
    if msaa != MsaaQuality::Disabled {
        unsafe {
            (*targetRT).msaaSamples = msaa as i32;
        }
    }
}

pub fn init(Unity_RenderPipelines_Universal_Runtime: *const Il2CppImage) {
    get_class_or_return!(Unity_RenderPipelines_Universal_Runtime, "UnityEngine.Rendering.Universal", ScriptableRenderer);

    let GetRenderTextureDescriptor_addr = get_method_addr(ScriptableRenderer, c"GetRenderTextureDescriptor", 3);
    new_hook!(GetRenderTextureDescriptor_addr, GetRenderTextureDescriptor);
}

