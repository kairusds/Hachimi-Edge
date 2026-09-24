use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{
    il2cpp::{
        hook::UnityEngine_CoreModule::{Behaviour, Component, Transform},
        symbols::get_method_addr,
        types::*,
    },
    windows::free_camera::{self, CameraScene},
};

type NoArgsFn = extern "C" fn(this: *mut Il2CppObject);
type GetCameraFn = extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject;

static HOME_DIRECTOR: AtomicPtr<Il2CppObject> = AtomicPtr::new(std::ptr::null_mut());
static HOME_CAMERA_SWITCHER: AtomicPtr<Il2CppObject> = AtomicPtr::new(std::ptr::null_mut());

static mut HOME_DIRECTOR_GET_CAMERA_ADDR: usize = 0;
static mut SWITCHER_GET_CAMERA_ADDR: usize = 0;
static mut SWITCHER_GET_CINEMA_BRAIN_ADDR: usize = 0;

static mut LAST_HOME_CAMERA_POS: Vector3_t = Vector3_t {
    x: 0.0,
    y: 1.2,
    z: -2.0,
};
static mut LAST_HOME_CAMERA_ROT: Quaternion_t = Quaternion_t {
    w: 1.0,
    x: 0.0,
    y: 0.0,
    z: 0.0,
};

fn get_home_camera() -> *mut Il2CppObject {
    let director = HOME_DIRECTOR.load(Ordering::Relaxed);
    if !director.is_null() && unsafe { HOME_DIRECTOR_GET_CAMERA_ADDR != 0 } {
        let func: GetCameraFn = unsafe { std::mem::transmute(HOME_DIRECTOR_GET_CAMERA_ADDR) };
        let cam = func(director);
        if !cam.is_null() {
            return cam;
        }
    }

    let switcher = HOME_CAMERA_SWITCHER.load(Ordering::Relaxed);
    if !switcher.is_null() && unsafe { SWITCHER_GET_CAMERA_ADDR != 0 } {
        let func: GetCameraFn = unsafe { std::mem::transmute(SWITCHER_GET_CAMERA_ADDR) };
        let cam = func(switcher);
        if !cam.is_null() {
            return cam;
        }
    }

    std::ptr::null_mut()
}

fn get_cinemachine_brain() -> *mut Il2CppObject {
    let switcher = HOME_CAMERA_SWITCHER.load(Ordering::Relaxed);
    if !switcher.is_null() {
        if unsafe { SWITCHER_GET_CINEMA_BRAIN_ADDR != 0 } {
            let func: GetCameraFn =
                unsafe { std::mem::transmute(SWITCHER_GET_CINEMA_BRAIN_ADDR) };
            let brain = func(switcher);
            if !brain.is_null() {
                return brain;
            }
        }
        let brain_ptr =
            unsafe { *(switcher.cast::<u8>().add(0x28) as *const *mut Il2CppObject) };
        if !brain_ptr.is_null() {
            return brain_ptr;
        }
    }

    let director = HOME_DIRECTOR.load(Ordering::Relaxed);
    if !director.is_null() {
        let switcher_ptr =
            unsafe { *(director.cast::<u8>().add(0x28) as *const *mut Il2CppObject) };
        if !switcher_ptr.is_null() {
            let brain_ptr =
                unsafe { *(switcher_ptr.cast::<u8>().add(0x28) as *const *mut Il2CppObject) };
            if !brain_ptr.is_null() {
                return brain_ptr;
            }
        }
    }

    std::ptr::null_mut()
}

pub fn apply_home_free_camera() {
    if !free_camera::is_scene_enabled(CameraScene::Home) {
        return;
    }

    let camera = get_home_camera();
    if camera.is_null() {
        return;
    }

    let camera_transform = Component::get_transform(camera);
    if camera_transform.is_null() {
        return;
    }

    let mut position = free_camera::camera_pos();
    Transform::set_position_Injected(camera_transform, &mut position);
    if let Some(mut rotation) = free_camera::camera_rotation() {
        Transform::set_rotation_Injected(camera_transform, &mut rotation);
    } else {
        let mut look_at = free_camera::camera_look_at();
        let mut world_up = Vector3_t {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        Transform::Internal_LookAt_Injected(camera_transform, &mut look_at, &mut world_up);
    }
}

fn on_home_frame() {
    let camera = get_home_camera();
    if !camera.is_null() {
        let camera_transform = Component::get_transform(camera);
        if !camera_transform.is_null() {
            if !free_camera::is_scene_enabled(CameraScene::Home) {
                let mut pos = Vector3_t::default();
                let mut rot = Quaternion_t::default();
                Transform::get_position_Injected(camera_transform, &mut pos);
                Transform::get_rotation_Injected(camera_transform, &mut rot);
                unsafe {
                    LAST_HOME_CAMERA_POS = pos;
                    LAST_HOME_CAMERA_ROT = rot;
                }

                let brain = get_cinemachine_brain();
                if !brain.is_null() && !Behaviour::get_enabled(brain) {
                    Behaviour::set_enabled(brain, true);
                }
            }
        }
    }

    unsafe {
        free_camera::set_home_active_with_transform(LAST_HOME_CAMERA_POS, LAST_HOME_CAMERA_ROT);
    }
    free_camera::tick();

    if free_camera::is_scene_enabled(CameraScene::Home) {
        let brain = get_cinemachine_brain();
        if !brain.is_null() && Behaviour::get_enabled(brain) {
            Behaviour::set_enabled(brain, false);
        }
        apply_home_free_camera();
    }
}

extern "C" fn HomeDirector_AlterUpdate(this: *mut Il2CppObject) {
    HOME_DIRECTOR.store(this, Ordering::Relaxed);
    let switcher_ptr = unsafe { *(this.cast::<u8>().add(0x28) as *const *mut Il2CppObject) };
    if !switcher_ptr.is_null() {
        HOME_CAMERA_SWITCHER.store(switcher_ptr, Ordering::Relaxed);
    }

    on_home_frame();
    get_orig_fn!(HomeDirector_AlterUpdate, NoArgsFn)(this);
    apply_home_free_camera();
}

extern "C" fn HomeDirector_OnDestroy(this: *mut Il2CppObject) {
    HOME_DIRECTOR.store(std::ptr::null_mut(), Ordering::Relaxed);
    HOME_CAMERA_SWITCHER.store(std::ptr::null_mut(), Ordering::Relaxed);
    free_camera::end_scene(CameraScene::Home);
    get_orig_fn!(HomeDirector_OnDestroy, NoArgsFn)(this);
}

extern "C" fn HomeCameraSwitcher_AlterUpdate(this: *mut Il2CppObject) {
    HOME_CAMERA_SWITCHER.store(this, Ordering::Relaxed);
    on_home_frame();
    get_orig_fn!(HomeCameraSwitcher_AlterUpdate, NoArgsFn)(this);
    apply_home_free_camera();
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HomeDirector);
    unsafe {
        HOME_DIRECTOR_GET_CAMERA_ADDR = get_method_addr(HomeDirector, c"GetCamera", 0);
    }
    let director_alter_update = get_method_addr(HomeDirector, c"AlterUpdate", 0);
    new_hook!(director_alter_update, HomeDirector_AlterUpdate);

    let director_on_destroy = get_method_addr(HomeDirector, c"OnDestroy", 0);
    new_hook!(director_on_destroy, HomeDirector_OnDestroy);

    get_class_or_return!(umamusume, Gallop, HomeCameraSwitcher);
    unsafe {
        SWITCHER_GET_CAMERA_ADDR = get_method_addr(HomeCameraSwitcher, c"get_Camera", 0);
        SWITCHER_GET_CINEMA_BRAIN_ADDR = get_method_addr(HomeCameraSwitcher, c"get_CinemaBrain", 0);
    }
    let switcher_alter_update = get_method_addr(HomeCameraSwitcher, c"AlterUpdate", 0);
    new_hook!(switcher_alter_update, HomeCameraSwitcher_AlterUpdate);
}
