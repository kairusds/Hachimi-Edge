use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{
        symbols::{get_class, get_method_addr},
        types::*,
    },
};

use super::Director;

static LIVE_TIMELINE_CONTROL: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn set_current(this: *mut Il2CppObject) {
    if !this.is_null() {
        LIVE_TIMELINE_CONTROL.store(this as usize, Ordering::Relaxed);
    }
}

fn clear_current() {
    LIVE_TIMELINE_CONTROL.store(0, Ordering::Relaxed);
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
    set_current(this);
    if free_camera::is_scene_enabled(CameraScene::Live) && !out_look_at.is_null() {
        unsafe { *out_look_at = free_camera::camera_look_at(); }
    }
}

extern "C" fn LiveTimelineControl_AlterLateUpdate(this: *mut Il2CppObject) {
    free_camera::tick();
    get_orig_fn!(LiveTimelineControl_AlterLateUpdate, NoArgsFn)(this);
}

extern "C" fn LiveTimelineControl_OnDestroy(this: *mut Il2CppObject) {
    Director::restore_live_disabled_heads(0, true);
    clear_current();
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

pub fn init(umamusume: *const Il2CppImage) {
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
}
