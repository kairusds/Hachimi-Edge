use widestring::Utf16Str;

use crate::{
    core::{Hachimi, ext::Utf16StringExt},
    il2cpp::{
        api::il2cpp_resolve_icall, ext::Il2CppObjectExt, hook::{
            Plugins::AnimateToUnity::AnRoot, UnityEngine_AssetBundleModule::AssetBundle,
            umamusume::{CameraData::{self, ShadowResolution}, FlashActionPlayer}
        }, symbols::{Array, get_method_addr}, types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GETCOMPONENTINCHILDREN_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    GetComponentInChildren, GETCOMPONENTINCHILDREN_ADDR,
    *mut Il2CppObject,
    this: *mut Il2CppObject, type_: *mut Il2CppObject, include_inactive: bool
);

static mut GETCOMPONENTSINTERNAL_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    GetComponentsInternal, GETCOMPONENTSINTERNAL_ADDR,
    Array<*mut Il2CppObject>,
    this: *mut Il2CppObject, type_: *mut Il2CppObject, use_search_type_as_array_return_type: bool,
    recursive: bool, include_inactive: bool, reverse: bool, /* Nullable */ result_list: *mut Il2CppObject
);

// Optimized out in assembly
pub fn GetComponentsInChildren(
    this: *mut Il2CppObject, type_: *mut Il2CppObject, include_inactive: bool
) -> Array<*mut Il2CppObject> {
    GetComponentsInternal(this, type_, true, true, include_inactive, false, 0 as _)
}

static mut SETACTIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetActive, SETACTIVE_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut GET_ACTIVESELF_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_activeSelf, GET_ACTIVESELF_ADDR, bool, this: *mut Il2CppObject);

// hook::UnityEngine_AssetBundleModule::AssetBundle
// Generic GameObject handler for prefabs. Used for ui flash and combined ui flash
pub fn on_LoadAsset(bundle: *mut Il2CppObject, this: *mut Il2CppObject, name: &Utf16Str) {
    if !name.starts_with(AssetBundle::ASSET_PATH_PREFIX) {
        return;
    }
    let path = &name[AssetBundle::ASSET_PATH_PREFIX.len()..];

    if path.starts_with("uianimation/flash/") {
        let root = GetComponentInChildren(this, AnRoot::type_object(), false);
        if !root.is_null() {
            AnRoot::on_LoadAsset(bundle, root, name);
        }
    }
    else if path.starts_with("uianimation/flashcombine/") {
        let player = GetComponentInChildren(this, FlashActionPlayer::type_object(), false);
        if !player.is_null() {
            FlashActionPlayer::on_LoadAsset(bundle, player, name);
        }
    }
}

fn customize(component: *mut Il2CppObject) {
    let shadow_resolution = Hachimi::instance().config.load().shadow_resolution;
    if shadow_resolution != ShadowResolution::Default {
        match unsafe { (*component).klass() } {
            // graphics quality - shadow resolution
            CameraData if CameraData == CameraData::class() => {
                CameraData::set_IsOverrideShadowResolution(component, true);
                CameraData::set_OverrideShadowResolution(component, shadow_resolution);
            }
            _ => return
        }
    }
}

type Internal_AddComponentWithTypeFn = extern "C" fn(this: *mut Il2CppObject, componentType: *mut Il2CppType) -> *mut Il2CppObject;
extern "C" fn Internal_AddComponentWithType(this: *mut Il2CppObject, componentType: *mut Il2CppType) -> *mut Il2CppObject {
    let component = get_orig_fn!(Internal_AddComponentWithType, Internal_AddComponentWithTypeFn)(this, componentType);
    if !component.is_null() {
        customize(component);
    }
    component
}

#[repr(C)]
struct FastPath {
    component: *mut Il2CppObject,
    oneFurtherThanResultValue: usize,
}

type TryGetComponentFastPathFn = extern "C" fn(this: *mut Il2CppObject, type_: *mut Il2CppType, oneFurtherThanResultValue: usize);
extern "C" fn TryGetComponentFastPath(this: *mut Il2CppObject, type_: *mut Il2CppType, oneFurtherThanResultValue: usize) {
    get_orig_fn!(TryGetComponentFastPath, TryGetComponentFastPathFn)(this, type_, oneFurtherThanResultValue);
    let fastPath = (oneFurtherThanResultValue - std::mem::size_of::<*mut Il2CppObject>())
        as *mut FastPath;
    let component = unsafe { (*fastPath).component };
    if !component.is_null() {
        customize(component);
    }
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, GameObject);

    let Internal_AddComponentWithType_addr = il2cpp_resolve_icall(
        c"UnityEngine.GameObject::Internal_AddComponentWithType(System.Type)".as_ptr()
    );
    let TryGetComponentFastPath_addr = il2cpp_resolve_icall(
        c"UnityEngine.GameObject::TryGetComponentFastPath(System.Type,System.IntPtr)".as_ptr()
    );

    unsafe {
        CLASS = GameObject;
        GETCOMPONENTINCHILDREN_ADDR = get_method_addr(GameObject, c"GetComponentInChildren", 2);
        GETCOMPONENTSINTERNAL_ADDR = il2cpp_resolve_icall(
            c"UnityEngine.GameObject::GetComponentsInternal(System.Type,System.Boolean,System.Boolean,\
            System.Boolean,System.Boolean,System.Object)".as_ptr()
        );
        SETACTIVE_ADDR = il2cpp_resolve_icall(c"UnityEngine.GameObject::SetActive(System.Boolean)".as_ptr());
        GET_ACTIVESELF_ADDR = il2cpp_resolve_icall(c"UnityEngine.GameObject::get_activeSelf()".as_ptr());
    }

    new_hook!(Internal_AddComponentWithType_addr, Internal_AddComponentWithType);
    new_hook!(TryGetComponentFastPath_addr, TryGetComponentFastPath);
}