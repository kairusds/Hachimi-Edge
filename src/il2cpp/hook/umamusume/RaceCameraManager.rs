use crate::{
    windows::free_camera::{self, CameraScene},
    il2cpp::{
        hook::UnityEngine_CoreModule::Transform,
        symbols::{get_method_addr, get_method_overload_addr},
        types::*,
    },
};

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn RaceCameraManager_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::set_race_active();
    free_camera::tick();

    let active = free_camera::is_scene_enabled(CameraScene::Race);
    Transform::set_update_race_camera(active);
    get_orig_fn!(RaceCameraManager_AlterLateUpdate, NoArgsFn)(this);
    Transform::set_update_race_camera(false);
}

type RaceChangeCameraModeFn = extern "C" fn(this: *mut Il2CppObject, mode: i32, is_skip: bool);
extern "C" fn RaceCameraManager_ChangeCameraMode(this: *mut Il2CppObject, mode: i32, is_skip: bool) {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(RaceCameraManager_ChangeCameraMode, RaceChangeCameraModeFn)(this, mode, is_skip);
}

// public bool PlayEventCamera(int targetHorseIndex, int[] rivalHorseIndexArray, int cameraId, bool isForceInPlaying = False, bool isForceUnPlayableArea = False) { }
type RacePlayEventCameraFn = extern "C" fn(
    this: *mut Il2CppObject,
    targetHorseIndex: i32,
    rivalHorseIndexArray: *mut Il2CppArray,
    cameraId: i32,
    isForceInPlaying: bool,
    isForceUnPlayableArea: bool,
) -> bool;
extern "C" fn RaceCameraManager_PlayEventCamera(
    this: *mut Il2CppObject,
    targetHorseIndex: i32,
    rivalHorseIndexArray: *mut Il2CppArray,
    cameraId: i32,
    isForceInPlaying: bool,
    isForceUnPlayableArea: bool,
) -> bool {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return false;
    }
    get_orig_fn!(RaceCameraManager_PlayEventCamera, RacePlayEventCameraFn)(this, targetHorseIndex, rivalHorseIndexArray, cameraId, isForceInPlaying, isForceUnPlayableArea)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", RaceCameraManager);

    let RaceCameraManager_AlterLateUpdate_addr = get_method_addr(RaceCameraManager, c"AlterLateUpdate", 0);
    new_hook!(RaceCameraManager_AlterLateUpdate_addr, RaceCameraManager_AlterLateUpdate);

    let RaceCameraManager_ChangeCameraMode_addr = get_method_addr(RaceCameraManager, c"ChangeCameraMode", 2);
    new_hook!(RaceCameraManager_ChangeCameraMode_addr, RaceCameraManager_ChangeCameraMode);

    let RaceCameraManager_PlayEventCamera_addr = get_method_overload_addr(
        RaceCameraManager,
        "PlayEventCamera",
        &[
            Il2CppTypeEnum_IL2CPP_TYPE_I4, // int targetHorseIndex
            Il2CppTypeEnum_IL2CPP_TYPE_SZARRAY, // int[] rivalHorseIndexArray
            Il2CppTypeEnum_IL2CPP_TYPE_I4, // int cameraId
            Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN, // bool isForceInPlaying
            Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN // bool isForceUnPlayableArea
        ]
    );
    new_hook!(RaceCameraManager_PlayEventCamera_addr, RaceCameraManager_PlayEventCamera);
}
