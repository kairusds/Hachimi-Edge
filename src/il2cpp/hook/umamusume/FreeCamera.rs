#![allow(non_snake_case)]

use std::{
    collections::{HashMap, HashSet},
    ptr::null_mut,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use once_cell::sync::Lazy;

use crate::{
    core::{Hachimi, free_camera::{self, CameraScene, FreeCameraMode}},
    il2cpp::{
        api::il2cpp_resolve_icall,
        ext::{Il2CppObjectExt, Il2CppStringExt, StringExt},
        hook::UnityEngine_CoreModule::{Component, GameObject, Object, Transform},
        symbols::{
            IEnumerable, get_class, get_field_from_name, get_field_value, get_method_addr,
            set_field_value,
        },
        types::*,
    },
};

static UPDATE_RACE_CAMERA: AtomicBool = AtomicBool::new(false);
static LIVE_TIMELINE_CONTROL: AtomicUsize = AtomicUsize::new(0);
static mut POST_EFFECT_DOF_CLASS: *mut Il2CppClass = null_mut();
static mut POST_EFFECT_DOF_IS_ENABLE_FIELD: *mut FieldInfo = null_mut();

static RACE_INFO_GATE_NO: Lazy<Mutex<HashMap<usize, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static LIVE_DISABLED_HEADS: Lazy<Mutex<HashMap<i32, HashSet<usize>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static RACE_DISABLED_HEADS: Lazy<Mutex<HashMap<i32, HashSet<usize>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static mut LIVE_GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR: usize = 0;
static mut LIVE_GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR: usize = 0;
static mut LIVE_GET_HEAD_TRANSFORM_ADDR: usize = 0;
static mut GET_OWNER_OBJECT_ADDR: usize = 0;
static mut RACE_VIEW_GET_MODEL_CONTROLLER_ADDR: usize = 0;
static mut RACE_GET_PREFAB_ATTACH_TRANSFORM_ADDR: usize = 0;
static mut HORSE_DATA_GET_GATE_NO_ADDR: usize = 0;
static mut HORSERACE_POSITION_FIELD: *mut FieldInfo = null_mut();
static mut HORSERACE_ROTATION_ON_LANE_FIELD: *mut FieldInfo = null_mut();

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);
type LiveCameraPosFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    sheet_index: i32,
    is_use_camera_motion: bool,
);
type LiveCameraLookAtFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    out_look_at: *mut Vector3_t,
);
type GetCameraPosFn = extern "C" fn(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t;
type GetCameraPos2Fn = extern "C" fn(
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: i32,
) -> *mut Vector3_t;
type GetCharacterWorldPosFn = extern "C" fn(
    ret: *mut Vector3_t,
    timeline_control: *mut Il2CppObject,
    pos_flag: i32,
    chara_parts: i32,
    chara_pos: *mut Vector3_t,
    offset: *mut Vector3_t,
) -> *mut Vector3_t;
type LiveVoidFrameFn = extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32);
type LiveVoidFrameTimeFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
);
type LiveBoolFrameFn = extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) -> bool;
type LiveBoolFrameTimeFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
) -> bool;
type SetupRadialBlurInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
);
type DofUpdateInfoDelegateInvokeFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
);
type CameraGetFloatFn = extern "C" fn(this: *mut Il2CppObject) -> f32;
type CameraSetFloatFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
type TransformSetVectorFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Vector3_t);
type TransformLookAtFn = extern "C" fn(
    this: *mut Il2CppObject,
    world_position: *mut Vector3_t,
    world_up: *mut Vector3_t,
);
type TransformSetQuaternionFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Quaternion_t);
type RaceChangeCameraModeFn = extern "C" fn(this: *mut Il2CppObject, mode: i32, is_skip: bool);
type RacePlayEventCameraFn = extern "C" fn(
    this: *mut Il2CppObject,
    p1: i32,
    p2: i32,
    p3: i32,
    p4: bool,
    p5: bool,
) -> bool;
type RaceUpdateCameraDistanceBlendRateFn = extern "C" fn(
    this: *mut Il2CppObject,
    p1: *mut Il2CppObject,
    p2: *mut Il2CppObject,
    p3: *mut Il2CppObject,
);
type HorseRaceInfoReplayCtorFn = extern "C" fn(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
);
type GetRunMotionSpeedFn = extern "C" fn(this: *mut Il2CppObject) -> f32;

extern "C" fn GameSystem_Update(this: *mut Il2CppObject) {
    free_camera::tick();
    get_orig_fn!(GameSystem_Update, NoArgsFn)(this);
}

extern "C" fn AlterUpdate_CameraPos(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    sheet_index: i32,
    is_use_camera_motion: bool,
) {
    let frame = if free_camera::is_scene_enabled(CameraScene::Live) { 0 } else { current_frame };
    get_orig_fn!(AlterUpdate_CameraPos, LiveCameraPosFn)(
        this,
        sheet,
        frame,
        current_time,
        sheet_index,
        is_use_camera_motion,
    );
}

extern "C" fn AlterUpdate_CameraLookAt(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    out_look_at: *mut Vector3_t,
) {
    get_orig_fn!(AlterUpdate_CameraLookAt, LiveCameraLookAtFn)(
        this,
        sheet,
        current_frame,
        current_time,
        out_look_at,
    );

    free_camera::set_live_active();
    if !this.is_null() {
        LIVE_TIMELINE_CONTROL.store(this as usize, Ordering::Relaxed);
    }
    if free_camera::is_scene_enabled(CameraScene::Live) && !out_look_at.is_null() {
        unsafe { *out_look_at = free_camera::camera_look_at(); }
    }
}

extern "C" fn GetCameraPos(
    ret: *mut Vector3_t,
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    if free_camera::is_scene_enabled(CameraScene::Live) &&
        free_camera::mode() == FreeCameraMode::SelfieStick
    {
        let field = get_field_from_name(unsafe { (*this).klass() }, c"setType");
        if !field.is_null() {
            set_field_value(this, field, &1i32);
        }
    }

    let result = get_orig_fn!(GetCameraPos, GetCameraPosFn)(ret, this, timeline_control);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe { *result = free_camera::camera_pos(); }
    }
    result
}

extern "C" fn GetCameraPos2(
    this: *mut Il2CppObject,
    timeline_control: *mut Il2CppObject,
    set_type: i32,
) -> *mut Vector3_t {
    free_camera::set_live_active();

    let result = get_orig_fn!(GetCameraPos2, GetCameraPos2Fn)(this, timeline_control, set_type);
    if free_camera::is_scene_enabled(CameraScene::Live) && !result.is_null() {
        unsafe { *result = free_camera::camera_pos(); }
    }
    result
}

extern "C" fn GetCharacterWorldPos(
    ret: *mut Vector3_t,
    timeline_control: *mut Il2CppObject,
    mut pos_flag: i32,
    mut chara_parts: i32,
    chara_pos: *mut Vector3_t,
    offset: *mut Vector3_t,
) -> *mut Vector3_t {
    free_camera::set_live_active();
    if !timeline_control.is_null() {
        LIVE_TIMELINE_CONTROL.store(timeline_control as usize, Ordering::Relaxed);
    }

    let is_selfie_stick = free_camera::is_scene_enabled(CameraScene::Live) &&
        free_camera::mode() == FreeCameraMode::SelfieStick;
    let is_head_selfie = free_camera::is_live_head_selfie();

    if is_selfie_stick {
        pos_flag = free_camera::live_position_flag();
        chara_parts = free_camera::live_part();
        unsafe {
            if !chara_pos.is_null() {
                *chara_pos = Vector3_t::default();
            }
            if !offset.is_null() {
                *offset = Vector3_t::default();
            }
        }
    }

    let result = get_orig_fn!(GetCharacterWorldPos, GetCharacterWorldPosFn)(
        ret,
        timeline_control,
        pos_flag,
        chara_parts,
        chara_pos,
        offset,
    );

    if is_selfie_stick && !result.is_null() {
        if is_head_selfie {
            free_camera::update_live_head_part_target(unsafe { *result });
        }
        else {
            free_camera::update_live_follow_position_target(unsafe { *result });
        }
    }
    result
}

extern "C" fn Director_AlterUpdate(this: *mut Il2CppObject) {
    free_camera::begin_live_director_update();
    get_orig_fn!(Director_AlterUpdate, NoArgsFn)(this);
    free_camera::set_live_active();

    let first_person = free_camera::is_live_first_person();
    let selfie_stick = free_camera::is_live_selfie_stick();
    let head_selfie = selfie_stick && free_camera::is_live_head_selfie();
    if !first_person && !selfie_stick {
        restore_disabled_heads(&LIVE_DISABLED_HEADS, 0, true);
        return;
    }

    let mut index = free_camera::live_character_position_index();
    let mut chara_object = live_get_character_object(this, index);
    if chara_object.is_null() && index > 0 {
        for fallback in (0..index).rev() {
            chara_object = live_get_character_object(this, fallback);
            if !chara_object.is_null() {
                index = fallback;
                break;
            }
        }
    }
    if chara_object.is_null() {
        restore_disabled_heads(&LIVE_DISABLED_HEADS, index, true);
        return;
    }

    let model_array = live_get_model_controller_array(chara_object);
    let model_controller = first_enumerable_item(model_array);
    if model_controller.is_null() {
        restore_disabled_heads(&LIVE_DISABLED_HEADS, index, true);
        return;
    }

    let head_transform = live_get_head_transform(model_controller);
    if head_transform.is_null() {
        restore_disabled_heads(&LIVE_DISABLED_HEADS, index, true);
        return;
    }

    let mut pos = Vector3_t::default();
    let mut rot = Quaternion_t::default();
    Transform::get_position_Injected(head_transform, &mut pos);
    Transform::get_rotation_Injected(head_transform, &mut rot);
    let mut forward = Vector3_t::default();
    Transform::get_forward(&mut forward, head_transform);

    let mut root_pos = pos;
    let owner = get_owner_object(model_controller);
    if !owner.is_null() {
        let owner_transform = GameObject::get_transform(owner);
        if !owner_transform.is_null() {
            Transform::get_position_Injected(owner_transform, &mut root_pos);
        }
    }

    if first_person {
        free_camera::update_first_person(CameraScene::Live, pos, rot, Some(forward));
        hide_head_parts(&LIVE_DISABLED_HEADS, model_controller, index);
        restore_disabled_heads(&LIVE_DISABLED_HEADS, index, false);
    }
    else if head_selfie {
        free_camera::update_live_head_follow(pos, rot, Some(forward));
        restore_disabled_heads(&LIVE_DISABLED_HEADS, 0, true);
    }
    else {
        free_camera::update_live_director_follow_target(pos, root_pos, rot, Some(forward));
        restore_disabled_heads(&LIVE_DISABLED_HEADS, 0, true);
    }
}

extern "C" fn LiveTimelineControl_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::tick();
    get_orig_fn!(LiveTimelineControl_AlterLateUpdate, NoArgsFn)(this);
}

extern "C" fn LiveTimelineControl_OnDestroy(this: *mut Il2CppObject) {
    restore_disabled_heads(&LIVE_DISABLED_HEADS, 0, true);
    LIVE_TIMELINE_CONTROL.store(0, Ordering::Relaxed);
    free_camera::end_scene(CameraScene::Live);
    get_orig_fn!(LiveTimelineControl_OnDestroy, NoArgsFn)(this);
}

extern "C" fn AlterUpdate_RadialBlur(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
    if free_camera::should_remove_camera_effects() {
        return;
    }
    get_orig_fn!(AlterUpdate_RadialBlur, LiveVoidFrameFn)(this, sheet, current_frame);
}

extern "C" fn SetupRadialBlurInfo(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
) {
    if free_camera::should_remove_camera_effects() {
        return;
    }
    get_orig_fn!(SetupRadialBlurInfo, SetupRadialBlurInfoFn)(
        this,
        update_info,
        cur_data,
        next_data,
        current_frame,
    );
}

extern "C" fn DOFUpdateInfoDelegate_Invoke(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
) {
    if free_camera::should_remove_camera_effects() && !update_info.is_null() {
        unsafe {
            if !POST_EFFECT_DOF_CLASS.is_null() &&
                !POST_EFFECT_DOF_IS_ENABLE_FIELD.is_null() &&
                (*update_info).klass() == POST_EFFECT_DOF_CLASS
            {
                set_field_value(update_info, POST_EFFECT_DOF_IS_ENABLE_FIELD, &false);
            }
        }
    }

    get_orig_fn!(DOFUpdateInfoDelegate_Invoke, DofUpdateInfoDelegateInvokeFn)(this, update_info);
}

macro_rules! live_skip_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            if free_camera::should_remove_camera_effects() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    }
}

macro_rules! live_skip_void_frame_time {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(
            this: *mut Il2CppObject,
            sheet: *mut Il2CppObject,
            current_frame: i32,
            current_time: f32,
        ) {
            if free_camera::should_remove_camera_effects() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame, current_time);
        }
    }
}

live_skip_void_frame_time!(AlterUpdate_MultiCameraPosition, LiveVoidFrameTimeFn);
live_skip_void_frame_time!(AlterUpdate_MultiCameraLookAt, LiveVoidFrameTimeFn);
live_skip_void_frame!(AlterUpdate_MultiCameraRadialBlur, LiveVoidFrameFn);
live_skip_void_frame_time!(AlterUpdate_EyeCameraPosition, LiveVoidFrameTimeFn);
live_skip_void_frame!(AlterUpdate_PostEffect_BloomDiffusion, LiveVoidFrameFn);
live_skip_void_frame!(AlterUpdate_TiltShift, LiveVoidFrameFn);
live_skip_void_frame!(AlterUpdate_CameraLayer, LiveVoidFrameFn);
live_skip_void_frame!(AlterUpdate_CameraSwitcher, LiveVoidFrameFn);
live_skip_void_frame_time!(AlterUpdate_MonitorCameraLookAt, LiveVoidFrameTimeFn);
live_skip_void_frame_time!(AlterUpdate_EyeCameraLookAt, LiveVoidFrameTimeFn);

extern "C" fn AlterUpdate_CameraFov(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) -> bool {
    if free_camera::should_remove_camera_effects() {
        return true;
    }
    get_orig_fn!(AlterUpdate_CameraFov, LiveBoolFrameFn)(this, sheet, current_frame)
}

extern "C" fn AlterUpdate_CameraRoll(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) -> bool {
    if free_camera::should_remove_camera_effects() {
        return true;
    }
    get_orig_fn!(AlterUpdate_CameraRoll, LiveBoolFrameFn)(this, sheet, current_frame)
}

extern "C" fn AlterUpdate_MultiCamera(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
) -> bool {
    if free_camera::should_remove_camera_effects() {
        return true;
    }
    get_orig_fn!(AlterUpdate_MultiCamera, LiveBoolFrameTimeFn)(this, sheet, current_frame, current_time)
}

extern "C" fn Camera_get_fieldOfView(this: *mut Il2CppObject) -> f32 {
    let scene = free_camera::scene();
    if let Some(fov) = free_camera::fov_for_scene(scene) {
        return fov;
    }
    get_orig_fn!(Camera_get_fieldOfView, CameraGetFloatFn)(this)
}

extern "C" fn Camera_set_nearClipPlane(this: *mut Il2CppObject, mut value: f32) {
    if free_camera::is_live_first_person() ||
        free_camera::is_live_selfie_stick() ||
        free_camera::is_scene_enabled(CameraScene::Race)
    {
        value = 0.001;
    }
    get_orig_fn!(Camera_set_nearClipPlane, CameraSetFloatFn)(this, value);
}

extern "C" fn Camera_get_nearClipPlane(this: *mut Il2CppObject) -> f32 {
    if free_camera::is_live_first_person() ||
        free_camera::is_live_selfie_stick() ||
        free_camera::is_scene_enabled(CameraScene::Race)
    {
        return 0.001;
    }
    get_orig_fn!(Camera_get_nearClipPlane, CameraGetFloatFn)(this)
}

extern "C" fn Camera_set_farClipPlane(this: *mut Il2CppObject, mut value: f32) {
    if free_camera::is_scene_enabled(CameraScene::Live) || free_camera::is_scene_enabled(CameraScene::Race) {
        value = 2500.0;
    }
    get_orig_fn!(Camera_set_farClipPlane, CameraSetFloatFn)(this, value);
}

extern "C" fn Camera_get_farClipPlane(this: *mut Il2CppObject) -> f32 {
    if free_camera::is_scene_enabled(CameraScene::Live) || free_camera::is_scene_enabled(CameraScene::Race) {
        return 2500.0;
    }
    get_orig_fn!(Camera_get_farClipPlane, CameraGetFloatFn)(this)
}

extern "C" fn Transform_set_position_Injected(this: *mut Il2CppObject, value: *mut Vector3_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) &&
        free_camera::is_scene_enabled(CameraScene::Race) &&
        !value.is_null()
    {
        unsafe { *value = free_camera::race_camera_pos(*value); }
    }
    get_orig_fn!(Transform_set_position_Injected, TransformSetVectorFn)(this, value);
}

extern "C" fn Transform_set_localPosition_Injected(this: *mut Il2CppObject, value: *mut Vector3_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) &&
        free_camera::is_scene_enabled(CameraScene::Race) &&
        !value.is_null()
    {
        unsafe { *value = free_camera::race_camera_pos(*value); }
    }
    get_orig_fn!(Transform_set_localPosition_Injected, TransformSetVectorFn)(this, value);
}

extern "C" fn Transform_Internal_LookAt_Injected(
    this: *mut Il2CppObject,
    world_position: *mut Vector3_t,
    world_up: *mut Vector3_t,
) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) && free_camera::is_scene_enabled(CameraScene::Race) {
        if let Some(mut rot) = free_camera::camera_rotation() {
            get_orig_fn!(Transform_set_rotation_Injected, TransformSetQuaternionFn)(this, &mut rot);
            return;
        }

        if !world_position.is_null() {
            unsafe { *world_position = free_camera::camera_look_at(); }
        }
    }
    get_orig_fn!(Transform_Internal_LookAt_Injected, TransformLookAtFn)(this, world_position, world_up);
}

extern "C" fn Transform_set_rotation_Injected(this: *mut Il2CppObject, value: *mut Quaternion_t) {
    get_orig_fn!(Transform_set_rotation_Injected, TransformSetQuaternionFn)(this, value);
}

extern "C" fn Transform_set_localRotation_Injected(this: *mut Il2CppObject, value: *mut Quaternion_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) && free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(Transform_set_localRotation_Injected, TransformSetQuaternionFn)(this, value);
}

extern "C" fn RaceCameraManager_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::set_race_active();
    free_camera::tick();

    let active = free_camera::is_scene_enabled(CameraScene::Race);
    UPDATE_RACE_CAMERA.store(active, Ordering::Relaxed);
    get_orig_fn!(RaceCameraManager_AlterLateUpdate, NoArgsFn)(this);
    UPDATE_RACE_CAMERA.store(false, Ordering::Relaxed);
}

extern "C" fn RaceCameraManager_ChangeCameraMode(this: *mut Il2CppObject, mode: i32, is_skip: bool) {
    if free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(RaceCameraManager_ChangeCameraMode, RaceChangeCameraModeFn)(this, mode, is_skip);
}

extern "C" fn RaceCameraEventBase_get_CameraFov(this: *mut Il2CppObject) -> f32 {
    if let Some(fov) = free_camera::fov_for_scene(CameraScene::Race) {
        return fov;
    }
    get_orig_fn!(RaceCameraEventBase_get_CameraFov, CameraGetFloatFn)(this)
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

extern "C" fn RaceViewBase_LateUpdateView(this: *mut Il2CppObject) {
    let first_person = free_camera::is_race_first_person();
    let head_selfie = free_camera::is_race_head_selfie();
    if first_person || head_selfie {
        let index = free_camera::race_model_index();
        let model_controller = race_get_model_controller(this, index);
        if !model_controller.is_null() {
            let empty = "".to_il2cpp_string();
            let eye_left = race_get_prefab_attach_transform(model_controller, 0x7, empty);
            let eye_right = race_get_prefab_attach_transform(model_controller, 0x8, empty);
            if !eye_left.is_null() && !eye_right.is_null() {
                let mut pos_left = Vector3_t::default();
                let mut pos_right = Vector3_t::default();
                let mut rot_left = Quaternion_t::default();
                let mut rot_right = Quaternion_t::default();

                Transform::get_position_Injected(eye_left, &mut pos_left);
                Transform::get_position_Injected(eye_right, &mut pos_right);
                Transform::get_rotation_Injected(eye_left, &mut rot_left);
                Transform::get_rotation_Injected(eye_right, &mut rot_right);

                let pos = Vector3_t {
                    x: (pos_left.x + pos_right.x) * 0.5,
                    y: (pos_left.y + pos_right.y) * 0.5,
                    z: (pos_left.z + pos_right.z) * 0.5,
                };
                let rot = free_camera::slerp_quaternion(rot_left, rot_right, 0.5);
                if first_person {
                    free_camera::update_first_person(CameraScene::Race, pos, rot, None);
                    hide_head_parts(&RACE_DISABLED_HEADS, model_controller, index);
                    restore_disabled_heads(&RACE_DISABLED_HEADS, index, false);
                }
                else {
                    free_camera::update_race_head_follow(pos, rot);
                    restore_disabled_heads(&RACE_DISABLED_HEADS, 0, true);
                }
            }
        }
    }
    else {
        restore_disabled_heads(&RACE_DISABLED_HEADS, 0, true);
    }

    get_orig_fn!(RaceViewBase_LateUpdateView, NoArgsFn)(this);
}

extern "C" fn RaceEffectManager_OnDestroy(this: *mut Il2CppObject) {
    restore_disabled_heads(&RACE_DISABLED_HEADS, 0, true);
    RACE_INFO_GATE_NO.lock().unwrap().clear();
    free_camera::end_scene(CameraScene::Race);
    get_orig_fn!(RaceEffectManager_OnDestroy, NoArgsFn)(this);
}

extern "C" fn HorseRaceInfoReplay_ctor(
    this: *mut Il2CppObject,
    data: *mut Il2CppObject,
    reader: *mut Il2CppObject,
) {
    get_orig_fn!(HorseRaceInfoReplay_ctor, HorseRaceInfoReplayCtorFn)(this, data, reader);

    if data.is_null() || unsafe { HORSE_DATA_GET_GATE_NO_ADDR } == 0 {
        return;
    }

    let get_gate_no: extern "C" fn(*mut Il2CppObject) -> i32 =
        unsafe { std::mem::transmute(HORSE_DATA_GET_GATE_NO_ADDR) };
    let gate_no = get_gate_no(data) - 1;
    RACE_INFO_GATE_NO.lock().unwrap().insert(this as usize, gate_no);
}

extern "C" fn HorseRaceInfoReplay_get_RunMotionSpeed(this: *mut Il2CppObject) -> f32 {
    let result = get_orig_fn!(HorseRaceInfoReplay_get_RunMotionSpeed, GetRunMotionSpeedFn)(this);

    if !Hachimi::instance().config.load().free_camera.enabled {
        return result;
    }

    let gate_no = RACE_INFO_GATE_NO
        .lock()
        .unwrap()
        .get(&(this as usize))
        .copied()
        .unwrap_or(-1);
    if gate_no < 0 ||
        unsafe { HORSERACE_POSITION_FIELD.is_null() || HORSERACE_ROTATION_ON_LANE_FIELD.is_null() }
    {
        return result;
    }

    let pos: Vector3_t = get_field_value(this, unsafe { HORSERACE_POSITION_FIELD });
    let rot: Quaternion_t = get_field_value(this, unsafe { HORSERACE_ROTATION_ON_LANE_FIELD });
    free_camera::update_race_target(gate_no, pos, rot);
    result
}

fn live_get_character_object(director: *mut Il2CppObject, index: i32) -> *mut Il2CppObject {
    if unsafe { LIVE_GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject, i32) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(LIVE_GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR) };
    func(director, index)
}

fn live_get_model_controller_array(chara_object: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { LIVE_GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(LIVE_GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR) };
    func(chara_object)
}

fn live_get_head_transform(model_controller: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { LIVE_GET_HEAD_TRANSFORM_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(LIVE_GET_HEAD_TRANSFORM_ADDR) };
    func(model_controller)
}

fn get_owner_object(model_controller: *mut Il2CppObject) -> *mut Il2CppObject {
    if unsafe { GET_OWNER_OBJECT_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(GET_OWNER_OBJECT_ADDR) };
    func(model_controller)
}

fn race_get_model_controller(view: *mut Il2CppObject, index: i32) -> *mut Il2CppObject {
    if unsafe { RACE_VIEW_GET_MODEL_CONTROLLER_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject, i32) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(RACE_VIEW_GET_MODEL_CONTROLLER_ADDR) };
    func(view, index)
}

fn race_get_prefab_attach_transform(
    model_controller: *mut Il2CppObject,
    part: i32,
    name: *mut Il2CppString,
) -> *mut Il2CppObject {
    if unsafe { RACE_GET_PREFAB_ATTACH_TRANSFORM_ADDR } == 0 {
        return null_mut();
    }
    let func: extern "C" fn(*mut Il2CppObject, i32, *mut Il2CppString) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(RACE_GET_PREFAB_ATTACH_TRANSFORM_ADDR) };
    func(model_controller, part, name)
}

fn first_enumerable_item(value: *mut Il2CppObject) -> *mut Il2CppObject {
    let enumerable = IEnumerable::<*mut Il2CppObject>::from(value);
    let Some(enumerator) = enumerable.enumerator() else {
        return null_mut();
    };
    let Some(mut iter) = enumerator.iter() else {
        return null_mut();
    };
    iter.find(|item| !item.is_null()).unwrap_or(null_mut())
}

fn hide_head_parts(
    store: &Lazy<Mutex<HashMap<i32, HashSet<usize>>>>,
    model_controller: *mut Il2CppObject,
    index: i32,
) {
    let owner = get_owner_object(model_controller);
    if owner.is_null() {
        return;
    }

    let transform = GameObject::get_transform(owner);
    if transform.is_null() {
        return;
    }

    let count = Transform::get_childCount(transform);
    for i in 0..count {
        let child = Transform::GetChild(transform, i);
        if child.is_null() {
            continue;
        }
        let game_object = Component::get_gameObject(child);
        if game_object.is_null() {
            continue;
        }
        let name = Object::get_name(game_object);
        if name.is_null() {
            continue;
        }
        let name = unsafe { (*name).as_utf16str().to_string() };
        if name == "M_Hair" || name == "M_Face" {
            store.lock().unwrap().entry(index).or_default().insert(game_object as usize);
            GameObject::SetActive(game_object, false);
        }
    }
}

fn restore_disabled_heads(
    store: &Lazy<Mutex<HashMap<i32, HashSet<usize>>>>,
    current_index: i32,
    force_all: bool,
) {
    let mut store = store.lock().unwrap();
    let mut restored = Vec::new();

    for (index, objects) in store.iter() {
        if *index == current_index && !force_all {
            continue;
        }

        for obj in objects {
            let obj = *obj as *mut Il2CppObject;
            if Object::IsNativeObjectAlive(obj) {
                GameObject::SetActive(obj, true);
            }
        }
        restored.push(*index);
    }

    for index in restored {
        store.remove(&index);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(game_system) = get_class(umamusume, c"Gallop", c"GameSystem") {
        let GameSystem_Update_addr = get_method_addr(game_system, c"Update", 0);
        new_hook!(GameSystem_Update_addr, GameSystem_Update);
    }

    if let Ok(live_timeline_control) = get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineControl") {
        let AlterUpdate_CameraPos_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraPos", 5);
        let AlterUpdate_CameraLookAt_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraLookAt", 4);
        let LiveTimelineControl_AlterLateUpdate_addr = get_method_addr(live_timeline_control, c"AlterLateUpdate", 0);
        let LiveTimelineControl_OnDestroy_addr = get_method_addr(live_timeline_control, c"OnDestroy", 0);
        let AlterUpdate_RadialBlur_addr = get_method_addr(live_timeline_control, c"AlterUpdate_RadialBlur", 2);
        let SetupRadialBlurInfo_addr = get_method_addr(live_timeline_control, c"SetupRadialBlurInfo", 4);
        let AlterUpdate_MultiCameraRadialBlur_addr = get_method_addr(live_timeline_control, c"AlterUpdate_MultiCameraRadialBlur", 2);
        let AlterUpdate_EyeCameraPosition_addr = get_method_addr(live_timeline_control, c"AlterUpdate_EyeCameraPosition", 3);
        let AlterUpdate_PostEffect_BloomDiffusion_addr = get_method_addr(live_timeline_control, c"AlterUpdate_PostEffect_BloomDiffusion", 2);
        let AlterUpdate_TiltShift_addr = get_method_addr(live_timeline_control, c"AlterUpdate_TiltShift", 2);
        let AlterUpdate_CameraLayer_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraLayer", 2);
        let AlterUpdate_CameraFov_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraFov", 2);
        let AlterUpdate_CameraRoll_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraRoll", 2);
        let AlterUpdate_MultiCamera_addr = get_method_addr(live_timeline_control, c"AlterUpdate_MultiCamera", 3);
        let AlterUpdate_CameraSwitcher_addr = get_method_addr(live_timeline_control, c"AlterUpdate_CameraSwitcher", 2);
        let AlterUpdate_MonitorCameraLookAt_addr = get_method_addr(live_timeline_control, c"AlterUpdate_MonitorCameraLookAt", 3);
        let AlterUpdate_EyeCameraLookAt_addr = get_method_addr(live_timeline_control, c"AlterUpdate_EyeCameraLookAt", 3);
        let AlterUpdate_MultiCameraPosition_addr = get_method_addr(live_timeline_control, c"AlterUpdate_MultiCameraPosition", 3);
        let AlterUpdate_MultiCameraLookAt_addr = get_method_addr(live_timeline_control, c"AlterUpdate_MultiCameraLookAt", 3);

        new_hook!(AlterUpdate_CameraPos_addr, AlterUpdate_CameraPos);
        new_hook!(AlterUpdate_CameraLookAt_addr, AlterUpdate_CameraLookAt);
        new_hook!(LiveTimelineControl_AlterLateUpdate_addr, LiveTimelineControl_AlterLateUpdate);
        new_hook!(LiveTimelineControl_OnDestroy_addr, LiveTimelineControl_OnDestroy);
        new_hook!(AlterUpdate_RadialBlur_addr, AlterUpdate_RadialBlur);
        new_hook!(SetupRadialBlurInfo_addr, SetupRadialBlurInfo);
        new_hook!(AlterUpdate_MultiCameraRadialBlur_addr, AlterUpdate_MultiCameraRadialBlur);
        new_hook!(AlterUpdate_EyeCameraPosition_addr, AlterUpdate_EyeCameraPosition);
        new_hook!(AlterUpdate_PostEffect_BloomDiffusion_addr, AlterUpdate_PostEffect_BloomDiffusion);
        new_hook!(AlterUpdate_TiltShift_addr, AlterUpdate_TiltShift);
        new_hook!(AlterUpdate_CameraLayer_addr, AlterUpdate_CameraLayer);
        new_hook!(AlterUpdate_CameraFov_addr, AlterUpdate_CameraFov);
        new_hook!(AlterUpdate_CameraRoll_addr, AlterUpdate_CameraRoll);
        new_hook!(AlterUpdate_MultiCamera_addr, AlterUpdate_MultiCamera);
        new_hook!(AlterUpdate_CameraSwitcher_addr, AlterUpdate_CameraSwitcher);
        new_hook!(AlterUpdate_MonitorCameraLookAt_addr, AlterUpdate_MonitorCameraLookAt);
        new_hook!(AlterUpdate_EyeCameraLookAt_addr, AlterUpdate_EyeCameraLookAt);
        new_hook!(AlterUpdate_MultiCameraPosition_addr, AlterUpdate_MultiCameraPosition);
        new_hook!(AlterUpdate_MultiCameraLookAt_addr, AlterUpdate_MultiCameraLookAt);
    }

    if let Ok(post_effect_dof) = get_class(umamusume, c"Gallop.Live.Cutt", c"PostEffectUpdateInfo_DOF") {
        unsafe {
            POST_EFFECT_DOF_CLASS = post_effect_dof;
            POST_EFFECT_DOF_IS_ENABLE_FIELD = get_field_from_name(post_effect_dof, c"IsEnableDOF");
        }
    }

    if let Ok(dof_update_info_delegate) = get_class(umamusume, c"Gallop.Live.Cutt", c"DOFUpdateInfoDelegate") {
        let DOFUpdateInfoDelegate_Invoke_addr = get_method_addr(dof_update_info_delegate, c"Invoke", 1);
        new_hook!(DOFUpdateInfoDelegate_Invoke_addr, DOFUpdateInfoDelegate_Invoke);
    }

    if let Ok(camera_pos_data) = get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineKeyCameraPositionData") {
        let GetCameraPos_addr = get_method_addr(camera_pos_data, c"GetValue", 1);
        let GetCameraPos2_addr = get_method_addr(camera_pos_data, c"GetValue", 2);
        new_hook!(GetCameraPos_addr, GetCameraPos);
        new_hook!(GetCameraPos2_addr, GetCameraPos2);
    }

    if let Ok(camera_lookat_data) = get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineKeyCameraLookAtData") {
        let GetCharacterWorldPos_addr = get_method_addr(camera_lookat_data, c"GetCharacterWorldPos", 5);
        new_hook!(GetCharacterWorldPos_addr, GetCharacterWorldPos);
    }

    if let Ok(director) = get_class(umamusume, c"Gallop.Live", c"Director") {
        unsafe {
            LIVE_GET_CHARACTER_OBJECT_FROM_POSITION_ID_ADDR =
                get_method_addr(director, c"GetCharacterObjectFromPositionId", 1);
        }
        let Director_AlterUpdate_addr = get_method_addr(director, c"AlterUpdate", 0);
        new_hook!(Director_AlterUpdate_addr, Director_AlterUpdate);
    }

    if let Ok(character_object) = get_class(umamusume, c"Gallop.Live", c"CharacterObject") {
        unsafe {
            LIVE_GET_LIVE_MODEL_CONTROLLER_ARRAY_ADDR =
                get_method_addr(character_object, c"get_LiveModelControllerArray", 0);
        }
    }

    if let Ok(live_model_controller) = get_class(umamusume, c"Gallop", c"LiveModelController") {
        unsafe {
            LIVE_GET_HEAD_TRANSFORM_ADDR =
                get_method_addr(live_model_controller, c"get_HeadTransform", 0);
        }
    }

    if let Ok(model_controller) = get_class(umamusume, c"Gallop", c"ModelController") {
        unsafe {
            GET_OWNER_OBJECT_ADDR = get_method_addr(model_controller, c"get_OwnerObject", 0);
        }
    }

    if let Ok(race_camera_manager) = get_class(umamusume, c"Gallop", c"RaceCameraManager") {
        let RaceCameraManager_AlterLateUpdate_addr = get_method_addr(race_camera_manager, c"AlterLateUpdate", 0);
        let RaceCameraManager_ChangeCameraMode_addr = get_method_addr(race_camera_manager, c"ChangeCameraMode", 2);
        let RaceCameraManager_PlayEventCamera_addr = get_method_addr(race_camera_manager, c"PlayEventCamera", 5);
        new_hook!(RaceCameraManager_AlterLateUpdate_addr, RaceCameraManager_AlterLateUpdate);
        new_hook!(RaceCameraManager_ChangeCameraMode_addr, RaceCameraManager_ChangeCameraMode);
        new_hook!(RaceCameraManager_PlayEventCamera_addr, RaceCameraManager_PlayEventCamera);
    }

    if let Ok(race_camera_event_base) = get_class(umamusume, c"Gallop", c"RaceCameraEventBase") {
        let RaceCameraEventBase_get_CameraFov_addr =
            get_method_addr(race_camera_event_base, c"get_CameraFov", 0);
        new_hook!(RaceCameraEventBase_get_CameraFov_addr, RaceCameraEventBase_get_CameraFov);
    }

    if let Ok(race_model_controller) = get_class(umamusume, c"Gallop", c"RaceModelController") {
        unsafe {
            RACE_GET_PREFAB_ATTACH_TRANSFORM_ADDR =
                get_method_addr(race_model_controller, c"GetPrefabAttachTransform", 2);
        }
        let RaceModelController_UpdateCameraDistanceBlendRate_addr =
            get_method_addr(race_model_controller, c"UpdateCameraDistanceBlendRate", 3);
        new_hook!(
            RaceModelController_UpdateCameraDistanceBlendRate_addr,
            RaceModelController_UpdateCameraDistanceBlendRate
        );
    }

    if let Ok(race_view_base) = get_class(umamusume, c"Gallop", c"RaceViewBase") {
        unsafe {
            RACE_VIEW_GET_MODEL_CONTROLLER_ADDR =
                get_method_addr(race_view_base, c"GetModelController", 1);
        }
        let RaceViewBase_LateUpdateView_addr = get_method_addr(race_view_base, c"LateUpdateView", 0);
        new_hook!(RaceViewBase_LateUpdateView_addr, RaceViewBase_LateUpdateView);
    }

    if let Ok(race_effect_manager) = get_class(umamusume, c"Gallop", c"RaceEffectManager") {
        let RaceEffectManager_OnDestroy_addr = get_method_addr(race_effect_manager, c"OnDestroy", 0);
        new_hook!(RaceEffectManager_OnDestroy_addr, RaceEffectManager_OnDestroy);
    }

    if let Ok(horse_data) = get_class(umamusume, c"Gallop", c"HorseData") {
        unsafe {
            HORSE_DATA_GET_GATE_NO_ADDR = get_method_addr(horse_data, c"get_GateNo", 0);
        }
    }

    if let Ok(horse_race_info_replay) = get_class(umamusume, c"Gallop", c"HorseRaceInfoReplay") {
        let HorseRaceInfoReplay_ctor_addr = get_method_addr(horse_race_info_replay, c".ctor", 2);
        let HorseRaceInfoReplay_get_RunMotionSpeed_addr =
            get_method_addr(horse_race_info_replay, c"get_RunMotionSpeed", 0);
        new_hook!(HorseRaceInfoReplay_ctor_addr, HorseRaceInfoReplay_ctor);
        new_hook!(HorseRaceInfoReplay_get_RunMotionSpeed_addr, HorseRaceInfoReplay_get_RunMotionSpeed);
    }

    if let Ok(horse_race_info) = get_class(umamusume, c"Gallop", c"HorseRaceInfo") {
        unsafe {
            HORSERACE_POSITION_FIELD = get_field_from_name(horse_race_info, c"_position");
            HORSERACE_ROTATION_ON_LANE_FIELD = get_field_from_name(horse_race_info, c"_rotationOnLane");
        }
    }

    let Camera_get_fieldOfView_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_fieldOfView()".as_ptr());
    let Camera_set_nearClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::set_nearClipPlane(System.Single)".as_ptr());
    let Camera_get_nearClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_nearClipPlane()".as_ptr());
    let Camera_set_farClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::set_farClipPlane(System.Single)".as_ptr());
    let Camera_get_farClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_farClipPlane()".as_ptr());
    let Transform_set_position_Injected_addr =
        il2cpp_resolve_icall(c"UnityEngine.Transform::set_position_Injected(UnityEngine.Vector3&)".as_ptr());
    let Transform_set_localPosition_Injected_addr =
        il2cpp_resolve_icall(c"UnityEngine.Transform::set_localPosition_Injected(UnityEngine.Vector3&)".as_ptr());
    let Transform_Internal_LookAt_Injected_addr =
        il2cpp_resolve_icall(c"UnityEngine.Transform::Internal_LookAt_Injected(UnityEngine.Vector3&,UnityEngine.Vector3&)".as_ptr());
    let Transform_set_rotation_Injected_addr =
        il2cpp_resolve_icall(c"UnityEngine.Transform::set_rotation_Injected(UnityEngine.Quaternion&)".as_ptr());
    let Transform_set_localRotation_Injected_addr =
        il2cpp_resolve_icall(c"UnityEngine.Transform::set_localRotation_Injected(UnityEngine.Quaternion&)".as_ptr());

    new_hook!(Camera_get_fieldOfView_addr, Camera_get_fieldOfView);
    new_hook!(Camera_set_nearClipPlane_addr, Camera_set_nearClipPlane);
    new_hook!(Camera_get_nearClipPlane_addr, Camera_get_nearClipPlane);
    new_hook!(Camera_set_farClipPlane_addr, Camera_set_farClipPlane);
    new_hook!(Camera_get_farClipPlane_addr, Camera_get_farClipPlane);
    new_hook!(Transform_set_position_Injected_addr, Transform_set_position_Injected);
    new_hook!(Transform_set_localPosition_Injected_addr, Transform_set_localPosition_Injected);
    new_hook!(Transform_Internal_LookAt_Injected_addr, Transform_Internal_LookAt_Injected);
    new_hook!(Transform_set_rotation_Injected_addr, Transform_set_rotation_Injected);
    new_hook!(Transform_set_localRotation_Injected_addr, Transform_set_localRotation_Injected);
}
