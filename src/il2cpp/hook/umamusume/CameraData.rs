use std::ptr::null_mut;
use serde::{Serialize, Deserialize};
use crate::{core::{game::Region, Hachimi}, il2cpp::{symbols::get_method_addr, types::*}};

#[derive(Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(i32)]
pub enum ShadowResolution {
    #[default] Default,
    _256 = 0x100,
    _512 = 0x200,
    _1024 = 0x400,
    _2048 = 0x800,
    _4096 = 0x1000
}

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut SET_ISOVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_IsOverrideShadowResolution, SET_ISOVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut SET_OVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_OverrideShadowResolution, SET_OVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: ShadowResolution);

pub fn raise_shadow_distance_to_floor(value: f32, floor: f32) -> f32 {
    if floor > 0.0 && value > 0.0 && value < floor {
        floor
    } else {
        value
    }
}

type SetMaxShadowDistanceFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
extern "C" fn set_MaxShadowDistance(this: *mut Il2CppObject, value: f32) {
    let floor = Hachimi::instance().config.load().shadow_distance;
    let value = raise_shadow_distance_to_floor(value, floor);
    get_orig_fn!(set_MaxShadowDistance, SetMaxShadowDistanceFn)(this, value)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.RenderPipeline", CameraData);
    unsafe {
        CLASS = CameraData;
        SET_ISOVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_IsOverrideShadowResolution", 1);
        SET_OVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_OverrideShadowResolution", 1);
    }

    if Hachimi::instance().game.region == Region::Japan {
        let SetMaxShadowDistance_addr = get_method_addr(CameraData, c"set_MaxShadowDistance", 1);
        new_hook!(SetMaxShadowDistance_addr, set_MaxShadowDistance);
    }
}
