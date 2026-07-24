use crate::{
    core::free_camera::{self, CameraScene},
    il2cpp::{api::il2cpp_resolve_icall, types::*}
};

type CameraGetFloatFn = extern "C" fn(this: *mut Il2CppObject) -> f32;
type CameraSetFloatFn = extern "C" fn(this: *mut Il2CppObject, value: f32);

static mut SET_FIELD_OF_VIEW_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_fieldOfView, SET_FIELD_OF_VIEW_ADDR, (), this: *mut Il2CppObject, value: f32);

fn should_override_near_clip() -> bool {
    free_camera::is_scene_enabled(CameraScene::Live) ||
        free_camera::is_scene_enabled(CameraScene::Race)
}

extern "C" fn Camera_get_fieldOfView(this: *mut Il2CppObject) -> f32 {
    let scene = free_camera::scene();
    if let Some(fov) = free_camera::fov_for_scene(scene) {
        return fov;
    }
    get_orig_fn!(Camera_get_fieldOfView, CameraGetFloatFn)(this)
}

extern "C" fn Camera_set_nearClipPlane(this: *mut Il2CppObject, mut value: f32) {
    if should_override_near_clip() {
        value = 0.001;
    }
    get_orig_fn!(Camera_set_nearClipPlane, CameraSetFloatFn)(this, value);
}

extern "C" fn Camera_get_nearClipPlane(this: *mut Il2CppObject) -> f32 {
    if should_override_near_clip() {
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

pub fn init(_UnityEngine_CoreModule: *const Il2CppImage) {
    let get_fieldOfView_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_fieldOfView()".as_ptr());
    unsafe {
        SET_FIELD_OF_VIEW_ADDR =
            il2cpp_resolve_icall(c"UnityEngine.Camera::set_fieldOfView(System.Single)".as_ptr());
    }
    let set_nearClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::set_nearClipPlane(System.Single)".as_ptr());
    let get_nearClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_nearClipPlane()".as_ptr());
    let set_farClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::set_farClipPlane(System.Single)".as_ptr());
    let get_farClipPlane_addr =
        il2cpp_resolve_icall(c"UnityEngine.Camera::get_farClipPlane()".as_ptr());

    new_hook!(get_fieldOfView_addr, Camera_get_fieldOfView);
    new_hook!(set_nearClipPlane_addr, Camera_set_nearClipPlane);
    new_hook!(get_nearClipPlane_addr, Camera_get_nearClipPlane);
    new_hook!(set_farClipPlane_addr, Camera_set_farClipPlane);
    new_hook!(get_farClipPlane_addr, Camera_get_farClipPlane);
}
