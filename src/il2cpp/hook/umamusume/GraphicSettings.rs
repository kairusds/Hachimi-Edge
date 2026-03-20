use serde::{Deserialize, Serialize};

use crate::{core::Hachimi, il2cpp::{symbols::{get_method_addr}, types::*}};

use super::{LowResolutionCamera, SingleModeStartResultCharaViewer};

#[cfg(target_os = "windows")]
static mut CLASS: *mut Il2CppClass = 0 as _;
#[cfg(target_os = "windows")]
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

#[cfg(target_os = "windows")]
pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = crate::il2cpp::symbols::SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

type GetVirtualResolutionFn = extern "C" fn(this: *mut Il2CppObject) -> Vector2Int_t;
extern "C" fn GetVirtualResolution(this: *mut Il2CppObject) -> Vector2Int_t {
    let mut res = get_orig_fn!(GetVirtualResolution, GetVirtualResolutionFn)(this);
    let mult = Hachimi::instance().config.load().virtual_res_mult;
    if mult != 1.0 {
        res *= mult;
    }
    res
}

type GetVirtualResolution3DFn = extern "C" fn(this: *mut Il2CppObject, is_forced_wide_aspect: bool) -> Vector2Int_t;
extern "C" fn GetVirtualResolution3D(this: *mut Il2CppObject, is_forced_wide_aspect: bool) -> Vector2Int_t {
    let mut res = get_orig_fn!(GetVirtualResolution3D, GetVirtualResolution3DFn)(this, is_forced_wide_aspect);
    let mult = Hachimi::instance().config.load().virtual_res_mult;
    if mult != 1.0 &&
        !SingleModeStartResultCharaViewer::setting_up_image_effect() &&
        !LowResolutionCamera::creating_render_texture()
    {
        res *= mult;
    }
    res
}

type GetVirtualResolutionWidth3DFn = extern "C" fn(this: *mut Il2CppObject) -> i32;
extern "C" fn GetVirtualResolutionWidth3D(this: *mut Il2CppObject) -> i32 {
    let mut width = get_orig_fn!(GetVirtualResolutionWidth3D, GetVirtualResolutionWidth3DFn)(this);
    let mult = Hachimi::instance().config.load().virtual_res_mult;
    if mult != 1.0 {
        width = (width as f32 * mult) as i32;
    }
    width
}

#[cfg(target_os = "windows")]
static mut UPDATE3DRENDERTEXTURE_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(Update3DRenderTexture, UPDATE3DRENDERTEXTURE_ADDR, (), this: *mut Il2CppObject);

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum GraphicsQuality {
    #[default] Default = -1,
    Toon1280 = 0,
    Toon1280x2,
    Toon1280x4,
    ToonFull,
    Max
}

// UnityEngine.Rendering.Universal 
#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum MsaaQuality {
    #[default] Disabled = 1,
    _2x = 2,
    _4x = 4,
    _8x = 8
}

type get_IsMSAAFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
pub extern "C" fn get_IsMSAA(this: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().msaa != MsaaQuality::Disabled {
        return true;
    }
    get_orig_fn!(get_IsMSAA, get_IsMSAAFn)(this)
}

type set_ResolutionScaleFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
extern "C" fn set_ResolutionScale(this: *mut Il2CppObject, value: f32) {
    let render_scale = Hachimi::instance().config.load().render_scale;
    let target_value = if render_scale != 1.0 { render_scale } else { value };
    get_orig_fn!(set_ResolutionScale, set_ResolutionScaleFn)(this, target_value);
}

type set_ResolutionScale2DFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
pub extern "C" fn set_ResolutionScale2D(this: *mut Il2CppObject, value: f32) {
    let render_scale = Hachimi::instance().config.load().render_scale;
    let target_value = if render_scale != 1.0 { render_scale } else { value };
    get_orig_fn!(set_ResolutionScale2D, set_ResolutionScale2DFn)(this, target_value);
}

type Get3DAntiAliasingLevelFn = extern "C" fn(this: *mut Il2CppObject, allowMSAA: bool) -> i32;
extern "C" fn Get3DAntiAliasingLevel(this: *mut Il2CppObject, allowMSAA: bool) -> i32 {
    let msaa = Hachimi::instance().config.load().msaa;
    if allowMSAA && msaa != MsaaQuality::Disabled {
        return msaa as i32;
    }
    get_orig_fn!(Get3DAntiAliasingLevel, Get3DAntiAliasingLevelFn)(this, allowMSAA)
}

type ApplyGraphicsQualityFn = extern "C" fn(this: *mut Il2CppObject, quality: GraphicsQuality, force: bool);
extern "C" fn ApplyGraphicsQuality(this: *mut Il2CppObject, quality: GraphicsQuality, force: bool) {
    let custom_quality = Hachimi::instance().config.load().graphics_quality;
    if custom_quality != GraphicsQuality::Default {
        return get_orig_fn!(ApplyGraphicsQuality, ApplyGraphicsQualityFn)(this, custom_quality, true);
    }

    get_orig_fn!(ApplyGraphicsQuality, ApplyGraphicsQualityFn)(this, quality, force);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GraphicSettings);

    let GetVirtualResolution3D_addr = get_method_addr(GraphicSettings, c"GetVirtualResolution3D", 1);
    let GetVirtualResolution_addr = get_method_addr(GraphicSettings, c"GetVirtualResolution", 0);
    let GetVirtualResolutionWidth3D_addr = get_method_addr(GraphicSettings, c"GetVirtualResolutionWidth3D", 0);
    let ApplyGraphicsQuality_addr = get_method_addr(GraphicSettings, c"ApplyGraphicsQuality", 2);

    let get_IsMSAA_addr = get_method_addr(GraphicSettings, c"get_IsMSAA", 0);
    let SetResolutionScale_addr = get_method_addr(GraphicSettings, c"set_ResolutionScale", 1);
    let SetResolutionScale2D_addr = get_method_addr(GraphicSettings, c"set_ResolutionScale2D", 1);
    let Get3DAntiAliasingLevel_addr = get_method_addr(GraphicSettings, c"Get3DAntiAliasingLevel", 1);

    new_hook!(GetVirtualResolution3D_addr, GetVirtualResolution3D);
    new_hook!(GetVirtualResolution_addr, GetVirtualResolution);
    new_hook!(GetVirtualResolutionWidth3D_addr, GetVirtualResolutionWidth3D);
    new_hook!(ApplyGraphicsQuality_addr, ApplyGraphicsQuality);

    new_hook!(get_IsMSAA_addr, get_IsMSAA);
    new_hook!(SetResolutionScale_addr, set_ResolutionScale);
    new_hook!(SetResolutionScale2D_addr, set_ResolutionScale2D);
    new_hook!(Get3DAntiAliasingLevel_addr, Get3DAntiAliasingLevel);

    #[cfg(target_os = "windows")]
    unsafe {
        CLASS = GraphicSettings;
        UPDATE3DRENDERTEXTURE_ADDR = get_method_addr(GraphicSettings, c"Update3DRenderTexture", 0);
    }
}