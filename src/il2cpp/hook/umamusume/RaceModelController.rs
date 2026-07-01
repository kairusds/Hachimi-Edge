use std::ptr::null_mut;

use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

static mut GET_PREFAB_ATTACH_TRANSFORM_ADDR: usize = 0;

pub fn GetPrefabAttachTransform(
    this: *mut Il2CppObject,
    part: i32,
    name: *mut Il2CppString,
) -> *mut Il2CppObject {
    if unsafe { GET_PREFAB_ATTACH_TRANSFORM_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject, i32, *mut Il2CppString) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_PREFAB_ATTACH_TRANSFORM_ADDR) };
    func(this, part, name)
}

type RaceUpdateCameraDistanceBlendRateFn = extern "C" fn(
    this: *mut Il2CppObject,
    p1: *mut Il2CppObject,
    p2: *mut Il2CppObject,
    p3: *mut Il2CppObject,
);

extern "C" fn RaceModelController_UpdateCameraDistanceBlendRate(
    this: *mut Il2CppObject,
    p1: *mut Il2CppObject,
    p2: *mut Il2CppObject,
    p3: *mut Il2CppObject,
) {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(RaceModelController_UpdateCameraDistanceBlendRate, RaceUpdateCameraDistanceBlendRateFn)(
        this,
        p1,
        p2,
        p3,
    );
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(race_model_controller) = get_class(umamusume, c"Gallop", c"RaceModelController") {
        unsafe {
            GET_PREFAB_ATTACH_TRANSFORM_ADDR =
                get_method_addr(race_model_controller, c"GetPrefabAttachTransform", 2);
        }
        let RaceModelController_UpdateCameraDistanceBlendRate_addr =
            get_method_addr(race_model_controller, c"UpdateCameraDistanceBlendRate", 3);
        new_hook!(
            RaceModelController_UpdateCameraDistanceBlendRate_addr,
            RaceModelController_UpdateCameraDistanceBlendRate
        );
    }
}
