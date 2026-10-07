use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::Unity_RenderPipelines_Universal_Runtime::UniversalRenderPipelineAsset,
        symbols::get_method_addr,
        types::*
    }
};

type UpdateShadowSettingsFn = extern "C" fn(this: *mut Il2CppObject, rendering_data: *mut Il2CppObject);
extern "C" fn UpdateShadowSettings(this: *mut Il2CppObject, rendering_data: *mut Il2CppObject) {
    get_orig_fn!(UpdateShadowSettings, UpdateShadowSettingsFn)(this, rendering_data);
    UniversalRenderPipelineAsset::apply_shadow_overrides();
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, "Gallop.RenderPipeline", GallopRenderer);

    let UpdateShadowSettings_addr = get_method_addr(GallopRenderer, c"UpdateShadowSettings", 1);
    new_hook!(UpdateShadowSettings_addr, UpdateShadowSettings);
}
