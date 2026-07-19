use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_class, get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
    },
};

use super::{Director, PostEffectUpdateInfo_DOF};

static LIVE_TIMELINE_CONTROL: AtomicUsize = AtomicUsize::new(0);
static mut POST_FILM_KEYS_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut POST_FILM_2_KEYS_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut POST_FILM_3_KEYS_FIELD: *mut FieldInfo = std::ptr::null_mut();
static mut CLEAR_POST_FILM_KEYS_ADDR: usize = 0;

fn clear_live_screen_effects(sheet: *mut Il2CppObject) {
    if sheet.is_null() || !free_camera::should_remove_live_screen_effects() {
        return;
    }

    let clear_addr = unsafe { CLEAR_POST_FILM_KEYS_ADDR };
    if clear_addr == 0 {
        return;
    }
    let clear: extern "C" fn(this: *mut Il2CppObject) =
        unsafe { std::mem::transmute(clear_addr) };

    for field in unsafe {
        [
            POST_FILM_KEYS_FIELD,
            POST_FILM_2_KEYS_FIELD,
            POST_FILM_3_KEYS_FIELD,
        ]
    } {
        if !field.is_null() {
            let keys = get_field_object_value::<Il2CppObject>(sheet, field);
            if !keys.is_null() {
                clear(keys);
            }
        }
    }
}

pub(crate) fn set_current(this: *mut Il2CppObject) {
    if !this.is_null() {
        LIVE_TIMELINE_CONTROL.store(this as usize, Ordering::Relaxed);
    }
}

fn clear_current() {
    LIVE_TIMELINE_CONTROL.store(0, Ordering::Relaxed);
}

fn should_remove_live_camera_effects() -> bool {
    free_camera::set_live_active();
    free_camera::should_remove_camera_effects()
}

fn should_override_live_camera() -> bool {
    free_camera::set_live_active();
    free_camera::is_scene_enabled(CameraScene::Live)
}

fn apply_current_live_character_options() {
    let director = Director::instance();
    if !director.is_null() {
        Director::apply_live_character_options(director);
    }
}

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
type LiveVoidFrameFn =
    extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32);
type LiveBoolFrameFn =
    extern "C" fn(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) -> bool;
type LiveVoidFrameTimeFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
);
type LiveFormationOffsetFn = extern "C" fn(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    character_object_list: *mut Il2CppObject,
    change_visibility: bool,
);
type SetupDOFUpdateInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
    camera_look_at: Vector3_t,
);
type SetupRadialBlurInfoFn = extern "C" fn(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
);

extern "C" fn AlterUpdate_CameraPos(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    current_time: f32,
    sheet_index: i32,
    mut is_use_camera_motion: bool,
) {
    free_camera::set_live_active();
    clear_live_screen_effects(sheet);
    let free_camera_active = free_camera::is_scene_enabled(CameraScene::Live);
    let frame = if free_camera_active {
        is_use_camera_motion = false;
        0
    } else {
        current_frame
    };
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
    free_camera::set_live_active();
    clear_live_screen_effects(sheet);
    get_orig_fn!(AlterUpdate_CameraLookAt, LiveCameraLookAtFn)(
        this,
        sheet,
        current_frame,
        current_time,
        out_look_at,
    );

    set_current(this);
    if free_camera::is_scene_enabled(CameraScene::Live) && !out_look_at.is_null() {
        unsafe {
            *out_look_at = free_camera::camera_look_at();
        }
    }
}

extern "C" fn LiveTimelineControl_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::set_live_active();
    free_camera::tick();
    get_orig_fn!(LiveTimelineControl_AlterLateUpdate, NoArgsFn)(this);
    apply_current_live_character_options();
    let director = Director::instance();
    if !director.is_null() {
        Director::enforce_live_free_camera_output(director);
    }
}

extern "C" fn LiveTimelineControl_OnDestroy(this: *mut Il2CppObject) {
    Director::restore_live_disabled_heads(0, true);
    clear_current();
    free_camera::end_scene(CameraScene::Live);
    get_orig_fn!(LiveTimelineControl_OnDestroy, NoArgsFn)(this);
}

extern "C" fn AlterUpdate_RadialBlur(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_remove_live_camera_effects() {
        return;
    }
    get_orig_fn!(AlterUpdate_RadialBlur, LiveVoidFrameFn)(this, sheet, current_frame);
}

extern "C" fn SetupDOFUpdateInfo(
    this: *mut Il2CppObject,
    update_info: *mut PostEffectUpdateInfo_DOF::PostEffectUpdateInfoDOF,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
    camera_look_at: Vector3_t,
) {
    get_orig_fn!(SetupDOFUpdateInfo, SetupDOFUpdateInfoFn)(
        this,
        update_info,
        cur_data,
        next_data,
        current_frame,
        camera_look_at,
    );

    if should_remove_live_camera_effects() {
        PostEffectUpdateInfo_DOF::disable(update_info);
    }
}

extern "C" fn SetupRadialBlurInfo(
    this: *mut Il2CppObject,
    update_info: *mut Il2CppObject,
    cur_data: *mut Il2CppObject,
    next_data: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_remove_live_camera_effects() {
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

macro_rules! live_skip_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            if should_remove_live_camera_effects() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_secondary_camera_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            let _guard = free_camera::begin_live_secondary_camera_update();
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_main_camera_void_frame {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(this: *mut Il2CppObject, sheet: *mut Il2CppObject, current_frame: i32) {
            if should_override_live_camera() {
                return;
            }
            get_orig_fn!($hook, $type)(this, sheet, current_frame);
        }
    };
}

macro_rules! live_secondary_camera_void_frame_time {
    ($hook:ident, $type:ty) => {
        extern "C" fn $hook(
            this: *mut Il2CppObject,
            sheet: *mut Il2CppObject,
            current_frame: i32,
            current_time: f32,
        ) {
            let _guard = free_camera::begin_live_secondary_camera_update();
            get_orig_fn!($hook, $type)(this, sheet, current_frame, current_time);
        }
    };
}

live_secondary_camera_void_frame_time!(AlterUpdate_MultiCameraPosition, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MultiCameraLookAt, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame!(AlterUpdate_MultiCameraRadialBlur, LiveVoidFrameFn);
live_secondary_camera_void_frame_time!(AlterUpdate_EyeCameraPosition, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MonitorCameraPosition, LiveVoidFrameTimeFn);
live_skip_void_frame!(AlterUpdate_PostEffect_BloomDiffusion, LiveVoidFrameFn);
live_skip_void_frame!(AlterUpdate_TiltShift, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraLayer, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraSwitcher, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_CameraMotion, LiveVoidFrameFn);
live_main_camera_void_frame!(AlterUpdate_HandShakeCamera, LiveVoidFrameFn);
live_secondary_camera_void_frame_time!(AlterUpdate_MonitorCameraLookAt, LiveVoidFrameTimeFn);
live_secondary_camera_void_frame_time!(AlterUpdate_EyeCameraLookAt, LiveVoidFrameTimeFn);

extern "C" fn AlterLateUpdate_CameraMotion(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) -> bool {
    if should_override_live_camera() {
        return false;
    }
    get_orig_fn!(AlterLateUpdate_CameraMotion, LiveBoolFrameFn)(this, sheet, current_frame)
}

extern "C" fn AlterUpdate_CameraFov(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_override_live_camera() {
        return;
    }
    get_orig_fn!(AlterUpdate_CameraFov, LiveVoidFrameFn)(this, sheet, current_frame);
}

extern "C" fn AlterUpdate_CameraRoll(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
) {
    if should_override_live_camera() {
        return;
    }
    get_orig_fn!(AlterUpdate_CameraRoll, LiveVoidFrameFn)(this, sheet, current_frame);
}

extern "C" fn AlterUpdate_FormationOffset(
    this: *mut Il2CppObject,
    sheet: *mut Il2CppObject,
    current_frame: i32,
    character_object_list: *mut Il2CppObject,
    mut change_visibility: bool,
) {
    free_camera::set_live_active();
    let disable_teleport = free_camera::should_disable_live_character_teleport();
    let frame = if disable_teleport { 0 } else { current_frame };
    if disable_teleport || free_camera::should_force_live_characters_visible() {
        change_visibility = false;
    }

    // Keep the formation-offset timeline at its initial pose when free camera
    // ignores the authored camera motion. This removes camera-directed teleports
    // without forcing character transform nodes to a shared local position.
    get_orig_fn!(AlterUpdate_FormationOffset, LiveFormationOffsetFn)(
        this,
        sheet,
        frame,
        character_object_list,
        change_visibility,
    );

    Director::apply_live_character_options_to_list(character_object_list);
    apply_current_live_character_options();
}

pub fn init(umamusume: *const Il2CppImage) {
    if let Ok(live_timeline_control) =
        get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineControl")
    {
        if let Ok(live_timeline_work_sheet) =
            get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineWorkSheet")
        {
            unsafe {
                POST_FILM_KEYS_FIELD =
                    get_field_from_name(live_timeline_work_sheet, c"postFilmKeys");
                POST_FILM_2_KEYS_FIELD =
                    get_field_from_name(live_timeline_work_sheet, c"postFilm2Keys");
                POST_FILM_3_KEYS_FIELD =
                    get_field_from_name(live_timeline_work_sheet, c"postFilm3Keys");
            }
        }

        if let Ok(post_film_keys) =
            get_class(umamusume, c"Gallop.Live.Cutt", c"LiveTimelineKeyPostFilmDataList")
        {
            unsafe {
                CLEAR_POST_FILM_KEYS_ADDR = get_method_addr(post_film_keys, c"Clear", 0);
            }
        }

        let AlterUpdate_CameraPos_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraPos", 5);
        let AlterUpdate_CameraLookAt_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraLookAt", 4);
        let LiveTimelineControl_AlterLateUpdate_addr =
            get_method_addr(live_timeline_control, c"AlterLateUpdate", 0);
        let LiveTimelineControl_OnDestroy_addr =
            get_method_addr(live_timeline_control, c"OnDestroy", 0);
        let AlterUpdate_RadialBlur_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_RadialBlur", 2);
        let SetupDOFUpdateInfo_addr =
            get_method_addr(live_timeline_control, c"SetupDOFUpdateInfo", 5);
        let SetupRadialBlurInfo_addr =
            get_method_addr(live_timeline_control, c"SetupRadialBlurInfo", 4);
        let AlterUpdate_MultiCameraRadialBlur_addr = get_method_addr(
            live_timeline_control,
            c"AlterUpdate_MultiCameraRadialBlur",
            2,
        );
        let AlterUpdate_EyeCameraPosition_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_EyeCameraPosition", 3);
        let AlterUpdate_MonitorCameraPosition_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_MonitorCameraPosition", 3);
        let AlterUpdate_PostEffect_BloomDiffusion_addr = get_method_addr(
            live_timeline_control,
            c"AlterUpdate_PostEffect_BloomDiffusion",
            2,
        );
        let AlterUpdate_TiltShift_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_TiltShift", 2);
        let AlterUpdate_CameraLayer_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraLayer", 2);
        let AlterUpdate_CameraFov_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraFov", 2);
        let AlterUpdate_CameraRoll_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraRoll", 2);
        let AlterUpdate_CameraMotion_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraMotion", 2);
        let AlterLateUpdate_CameraMotion_addr =
            get_method_addr(live_timeline_control, c"AlterLateUpdate_CameraMotion", 2);
        let AlterUpdate_HandShakeCamera_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_HandShakeCamera", 2);
        let AlterUpdate_CameraSwitcher_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_CameraSwitcher", 2);
        let AlterUpdate_MonitorCameraLookAt_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_MonitorCameraLookAt", 3);
        let AlterUpdate_EyeCameraLookAt_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_EyeCameraLookAt", 3);
        let AlterUpdate_MultiCameraPosition_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_MultiCameraPosition", 3);
        let AlterUpdate_MultiCameraLookAt_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_MultiCameraLookAt", 3);
        let AlterUpdate_FormationOffset_addr =
            get_method_addr(live_timeline_control, c"AlterUpdate_FormationOffset", 4);

        new_hook!(AlterUpdate_CameraPos_addr, AlterUpdate_CameraPos);
        new_hook!(AlterUpdate_CameraLookAt_addr, AlterUpdate_CameraLookAt);
        new_hook!(
            LiveTimelineControl_AlterLateUpdate_addr,
            LiveTimelineControl_AlterLateUpdate
        );
        new_hook!(
            LiveTimelineControl_OnDestroy_addr,
            LiveTimelineControl_OnDestroy
        );
        new_hook!(AlterUpdate_RadialBlur_addr, AlterUpdate_RadialBlur);
        new_hook!(SetupDOFUpdateInfo_addr, SetupDOFUpdateInfo);
        new_hook!(SetupRadialBlurInfo_addr, SetupRadialBlurInfo);
        new_hook!(
            AlterUpdate_MultiCameraRadialBlur_addr,
            AlterUpdate_MultiCameraRadialBlur
        );
        new_hook!(
            AlterUpdate_EyeCameraPosition_addr,
            AlterUpdate_EyeCameraPosition
        );
        new_hook!(
            AlterUpdate_MonitorCameraPosition_addr,
            AlterUpdate_MonitorCameraPosition
        );
        new_hook!(
            AlterUpdate_PostEffect_BloomDiffusion_addr,
            AlterUpdate_PostEffect_BloomDiffusion
        );
        new_hook!(AlterUpdate_TiltShift_addr, AlterUpdate_TiltShift);
        new_hook!(AlterUpdate_CameraLayer_addr, AlterUpdate_CameraLayer);
        new_hook!(AlterUpdate_CameraFov_addr, AlterUpdate_CameraFov);
        new_hook!(AlterUpdate_CameraRoll_addr, AlterUpdate_CameraRoll);
        new_hook!(AlterUpdate_CameraMotion_addr, AlterUpdate_CameraMotion);
        new_hook!(
            AlterLateUpdate_CameraMotion_addr,
            AlterLateUpdate_CameraMotion
        );
        new_hook!(
            AlterUpdate_HandShakeCamera_addr,
            AlterUpdate_HandShakeCamera
        );
        new_hook!(AlterUpdate_CameraSwitcher_addr, AlterUpdate_CameraSwitcher);
        new_hook!(
            AlterUpdate_MonitorCameraLookAt_addr,
            AlterUpdate_MonitorCameraLookAt
        );
        new_hook!(
            AlterUpdate_EyeCameraLookAt_addr,
            AlterUpdate_EyeCameraLookAt
        );
        new_hook!(
            AlterUpdate_MultiCameraPosition_addr,
            AlterUpdate_MultiCameraPosition
        );
        new_hook!(
            AlterUpdate_MultiCameraLookAt_addr,
            AlterUpdate_MultiCameraLookAt
        );
        new_hook!(
            AlterUpdate_FormationOffset_addr,
            AlterUpdate_FormationOffset
        );
    }
}
