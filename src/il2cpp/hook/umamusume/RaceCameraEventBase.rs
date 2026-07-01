use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

type CameraGetFloatFn = extern "C" fn(this: *mut Il2CppObject) -> f32;

extern "C" fn RaceCameraEventBase_get_CameraFov(this: *mut Il2CppObject) -> f32 {
    if let Some(fov) = free_camera::fov_for_scene(CameraScene::Race) {
        return fov;
    }
    get_orig_fn!(RaceCameraEventBase_get_CameraFov, CameraGetFloatFn)(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(race_camera_event_base) = get_class(umamusume, c"Gallop", c"RaceCameraEventBase") {
        let RaceCameraEventBase_get_CameraFov_addr =
            get_method_addr(race_camera_event_base, c"get_CameraFov", 0);
        new_hook!(RaceCameraEventBase_get_CameraFov_addr, RaceCameraEventBase_get_CameraFov);
    }
}
