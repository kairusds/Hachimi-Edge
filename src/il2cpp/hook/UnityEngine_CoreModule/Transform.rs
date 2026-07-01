#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    il2cpp::{
        api::{il2cpp_class_get_type, il2cpp_resolve_icall, il2cpp_type_get_object},
        symbols::get_method_addr,
        types::*
    }
};
#[cfg(target_os = "windows")]
use crate::core::free_camera::{self, CameraScene};

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

#[cfg(target_os = "windows")]
static UPDATE_RACE_CAMERA: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub fn set_update_race_camera(value: bool) {
    UPDATE_RACE_CAMERA.store(value, Ordering::Relaxed);
}

// public Transform get_parent() { }
static mut GET_PARENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_parent, GET_PARENT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

// public Int32 get_childCount() { }
static mut GET_CHILDCOUNT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_childCount, GET_CHILDCOUNT_ADDR, i32, this: *mut Il2CppObject);

// public Transform GetChild(Int32 index) { }
static mut GETCHILD_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetChild, GETCHILD_ADDR, *mut Il2CppObject, this: *mut Il2CppObject, index: i32);

// public Vector3 get_position() { }
static mut GET_POSITION_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_position, GET_POSITION_ADDR, Vector3_t, this: *mut Il2CppObject);

// public Void set_position(Vector3 value) { }
static mut SET_POSITION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_position, SET_POSITION_ADDR, (), this: *mut Il2CppObject, value: Vector3_t);

static mut GET_POSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_position_Injected, GET_POSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut SET_POSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_position_Injected, SET_POSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut GET_LOCALPOSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_localPosition_Injected, GET_LOCALPOSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut SET_LOCALPOSITION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_localPosition_Injected, SET_LOCALPOSITION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Vector3_t);

static mut GET_ROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_rotation_Injected, GET_ROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut GET_FORWARD_ADDR: usize = 0;
pub fn get_forward(ret: *mut Vector3_t, this: *mut Il2CppObject) -> *mut Vector3_t {
    if unsafe { GET_FORWARD_ADDR } == 0 {
        return ret;
    }
    let orig_fn: extern "C" fn(*mut Vector3_t, *mut Il2CppObject) -> *mut Vector3_t =
        unsafe { std::mem::transmute(GET_FORWARD_ADDR) };
    orig_fn(ret, this)
}

static mut SET_ROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_rotation_Injected, SET_ROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut GET_LOCALROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_localRotation_Injected, GET_LOCALROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut SET_LOCALROTATION_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_localRotation_Injected, SET_LOCALROTATION_INJECTED_ADDR, (), this: *mut Il2CppObject, value: *mut Quaternion_t);

static mut INTERNAL_LOOKAT_INJECTED_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    Internal_LookAt_Injected,
    INTERNAL_LOOKAT_INJECTED_ADDR,
    (),
    this: *mut Il2CppObject,
    world_position: *mut Vector3_t,
    world_up: *mut Vector3_t
);

#[cfg(target_os = "windows")]
type TransformSetVectorFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Vector3_t);
#[cfg(target_os = "windows")]
type TransformLookAtFn = extern "C" fn(
    this: *mut Il2CppObject,
    world_position: *mut Vector3_t,
    world_up: *mut Vector3_t,
);
#[cfg(target_os = "windows")]
type TransformSetQuaternionFn = extern "C" fn(this: *mut Il2CppObject, value: *mut Quaternion_t);

#[cfg(target_os = "windows")]
extern "C" fn Transform_set_position_Injected(this: *mut Il2CppObject, value: *mut Vector3_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) &&
        free_camera::is_scene_enabled(CameraScene::Race) &&
        !value.is_null()
    {
        unsafe { *value = free_camera::race_camera_pos(*value); }
    }
    get_orig_fn!(Transform_set_position_Injected, TransformSetVectorFn)(this, value);
}

#[cfg(target_os = "windows")]
extern "C" fn Transform_set_localPosition_Injected(this: *mut Il2CppObject, value: *mut Vector3_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) &&
        free_camera::is_scene_enabled(CameraScene::Race) &&
        !value.is_null()
    {
        unsafe { *value = free_camera::race_camera_pos(*value); }
    }
    get_orig_fn!(Transform_set_localPosition_Injected, TransformSetVectorFn)(this, value);
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
extern "C" fn Transform_set_rotation_Injected(this: *mut Il2CppObject, value: *mut Quaternion_t) {
    get_orig_fn!(Transform_set_rotation_Injected, TransformSetQuaternionFn)(this, value);
}

#[cfg(target_os = "windows")]
extern "C" fn Transform_set_localRotation_Injected(this: *mut Il2CppObject, value: *mut Quaternion_t) {
    if UPDATE_RACE_CAMERA.load(Ordering::Relaxed) && free_camera::is_scene_enabled(CameraScene::Race) {
        return;
    }
    get_orig_fn!(Transform_set_localRotation_Injected, TransformSetQuaternionFn)(this, value);
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Transform);

    unsafe {
        TYPE_OBJECT = il2cpp_type_get_object(il2cpp_class_get_type(Transform));
        GET_PARENT_ADDR = get_method_addr(Transform, c"get_parent", 0);
        GET_CHILDCOUNT_ADDR = get_method_addr(Transform, c"get_childCount", 0);
        GETCHILD_ADDR = get_method_addr(Transform, c"GetChild", 1);
        GET_POSITION_ADDR = get_method_addr(Transform, c"get_position", 0);
        SET_POSITION_ADDR = get_method_addr(Transform, c"set_position", 1);
        GET_POSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_position_Injected(UnityEngine.Vector3&)".as_ptr());
        SET_POSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_position_Injected(UnityEngine.Vector3&)".as_ptr());
        GET_LOCALPOSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_localPosition_Injected(UnityEngine.Vector3&)".as_ptr());
        SET_LOCALPOSITION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_localPosition_Injected(UnityEngine.Vector3&)".as_ptr());
        GET_ROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_rotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        GET_FORWARD_ADDR = get_method_addr(Transform, c"get_forward", 0);
        SET_ROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_rotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        GET_LOCALROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::get_localRotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        SET_LOCALROTATION_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::set_localRotation_Injected(UnityEngine.Quaternion&)".as_ptr());
        INTERNAL_LOOKAT_INJECTED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Transform::Internal_LookAt_Injected(UnityEngine.Vector3&,UnityEngine.Vector3&)".as_ptr());
    }

    #[cfg(target_os = "windows")]
    {
        let set_position_Injected_addr = unsafe { SET_POSITION_INJECTED_ADDR };
        let set_localPosition_Injected_addr = unsafe { SET_LOCALPOSITION_INJECTED_ADDR };
        let Internal_LookAt_Injected_addr = unsafe { INTERNAL_LOOKAT_INJECTED_ADDR };
        let set_rotation_Injected_addr = unsafe { SET_ROTATION_INJECTED_ADDR };
        let set_localRotation_Injected_addr = unsafe { SET_LOCALROTATION_INJECTED_ADDR };

        new_hook!(set_position_Injected_addr, Transform_set_position_Injected);
        new_hook!(set_localPosition_Injected_addr, Transform_set_localPosition_Injected);
        new_hook!(Internal_LookAt_Injected_addr, Transform_Internal_LookAt_Injected);
        new_hook!(set_rotation_Injected_addr, Transform_set_rotation_Injected);
        new_hook!(set_localRotation_Injected_addr, Transform_set_localRotation_Injected);
    }
}
