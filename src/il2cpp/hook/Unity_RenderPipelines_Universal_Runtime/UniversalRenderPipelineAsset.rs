use std::ptr::null_mut;

use serde::{Deserialize, Serialize};

use crate::{
    core::Hachimi,
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr},
        types::*
    }
};

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum SoftShadowQuality {
    #[default]
    UsePipelineSettings = 0,
    Low = 1,
    Medium = 2,
    High = 3
}

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_method_wrapper_fn!(get_asset, GET_ASSET_ADDR, *mut Il2CppObject,);

def_field_value_accessors!(get_ShadowDistance, set_ShadowDistance, SHADOWDISTANCE_FIELD, f32);
def_field_value_accessors!(get_ShadowDepthBias, set_ShadowDepthBias, SHADOWDEPTHBIAS_FIELD, f32);
def_field_value_accessors!(get_ShadowNormalBias, set_ShadowNormalBias, SHADOWNORMALBIAS_FIELD, f32);
def_field_value_accessors!(get_SoftShadowsSupported, set_SoftShadowsSupported, SOFTSHADOWSSUPPORTED_FIELD, bool);
def_field_value_accessors!(get_SoftShadowQuality, set_SoftShadowQuality, SOFTSHADOWQUALITY_FIELD, SoftShadowQuality);
def_field_object_accessors!(get get_RendererDataList, RENDERERDATALIST_FIELD, Il2CppArray);

struct OriginalShadowSettings {
    soft_shadows_supported: bool,
    soft_shadow_quality: SoftShadowQuality,
    shadow_distance: f32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32
}
static mut ORIGINALS: Option<OriginalShadowSettings> = None;

pub fn apply_shadow_overrides() {
    let config = Hachimi::instance().config.load();
    let asset = get_asset();
    if asset.is_null() {
        return;
    }

    if unsafe { ORIGINALS.is_none() } {
        unsafe {
            ORIGINALS = Some(OriginalShadowSettings {
                soft_shadows_supported: get_SoftShadowsSupported(asset),
                soft_shadow_quality: get_SoftShadowQuality(asset),
                shadow_distance: get_ShadowDistance(asset),
                shadow_depth_bias: get_ShadowDepthBias(asset),
                shadow_normal_bias: get_ShadowNormalBias(asset)
            });
        }
    }
    let originals: &OriginalShadowSettings = unsafe { ORIGINALS.as_ref().unwrap() };

    if config.soft_shadows {
        set_SoftShadowsSupported(asset, true);
        if config.soft_shadow_quality != SoftShadowQuality::UsePipelineSettings {
            set_SoftShadowQuality(asset, config.soft_shadow_quality);
        }
    } else {
        set_SoftShadowsSupported(asset, originals.soft_shadows_supported);
        set_SoftShadowQuality(asset, originals.soft_shadow_quality);
    }

    if config.shadow_distance > 0.0 {
        if originals.shadow_distance < config.shadow_distance {
            set_ShadowDistance(asset, config.shadow_distance);
        }
    }

    match config.shadow_depth_bias {
        Some(bias) => set_ShadowDepthBias(asset, bias),
        None => set_ShadowDepthBias(asset, originals.shadow_depth_bias)
    }
    match config.shadow_normal_bias {
        Some(bias) => set_ShadowNormalBias(asset, bias),
        None => set_ShadowNormalBias(asset, originals.shadow_normal_bias)
    }
}

pub fn init(Unity_RenderPipelines_Universal_Runtime: *const Il2CppImage) {
    get_class_or_return!(Unity_RenderPipelines_Universal_Runtime, "UnityEngine.Rendering.Universal", UniversalRenderPipelineAsset);
    get_class_or_return!(Unity_RenderPipelines_Universal_Runtime, "UnityEngine.Rendering.Universal", UniversalRenderPipeline);

    unsafe {
        CLASS = UniversalRenderPipelineAsset;
        GET_ASSET_ADDR = get_method_addr(UniversalRenderPipeline, c"get_asset", 0);
        SHADOWDISTANCE_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_ShadowDistance");
        SHADOWDEPTHBIAS_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_ShadowDepthBias");
        SHADOWNORMALBIAS_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_ShadowNormalBias");
        SOFTSHADOWSSUPPORTED_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_SoftShadowsSupported");
        SOFTSHADOWQUALITY_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_SoftShadowQuality");
        RENDERERDATALIST_FIELD = get_field_from_name(UniversalRenderPipelineAsset, c"m_RendererDataList");
    }
}
