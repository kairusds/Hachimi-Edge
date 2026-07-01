use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        hook::UnityEngine_CoreModule::Transform,
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);
type RaceChangeCameraModeFn = extern "C" fn(this: *mut Il2CppObject, mode: i32, is_skip: bool);
type RacePlayEventCameraFn = extern "C" fn(
    this: *mut Il2CppObject,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: bool,
    p5: bool,
) -> bool;

extern "C" fn RaceCameraManager_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::set_race_active();
    free_camera::tick();

    let active = free_camera::is_scene_enabled(CameraScene::Race);
    Transform::set_update_race_camera(active);
    get_orig_fn!(RaceCameraManager_AlterLateUpdate, NoArgsFn)(this);
    Transform::set_update_race_camera(false);
}

extern "C" fn RaceCameraManager_ChangeCameraMode(this: *mut Il2CppObject, mode: i32, is_skip: bool) {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(RaceCameraManager_ChangeCameraMode, RaceChangeCameraModeFn)(this, mode, is_skip);
}

extern "C" fn RaceCameraManager_PlayEventCamera(
    this: *mut Il2CppObject,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: bool,
    p5: bool,
) -> bool {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return false;
    }
    get_orig_fn!(RaceCameraManager_PlayEventCamera, RacePlayEventCameraFn)(this, p1, p2, p3, p4, p5)
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(race_camera_manager) = get_class(umamusume, c"Gallop", c"RaceCameraManager") {
        let RaceCameraManager_AlterLateUpdate_addr = get_method_addr(race_camera_manager, c"AlterLateUpdate", 0);
        let RaceCameraManager_ChangeCameraMode_addr = get_method_addr(race_camera_manager, c"ChangeCameraMode", 2);
        let RaceCameraManager_PlayEventCamera_addr = get_method_addr(race_camera_manager, c"PlayEventCamera", 5);
        new_hook!(RaceCameraManager_AlterLateUpdate_addr, RaceCameraManager_AlterLateUpdate);
        new_hook!(RaceCameraManager_ChangeCameraMode_addr, RaceCameraManager_ChangeCameraMode);
        new_hook!(RaceCameraManager_PlayEventCamera_addr, RaceCameraManager_PlayEventCamera);
    }
}
