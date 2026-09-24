use std::sync::Mutex;

use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use widestring::Utf16Str;

use crate::{core::{Hachimi, ext::Utf16StringExt, game::Region, hachimi::AssetMetadata}, il2cpp::{
    api::{il2cpp_array_new, il2cpp_resolve_icall}, ext::{Il2CppObjectExt, Il2CppStringExt}, hook::{
        mscorlib::Byte,
        umamusume::{StoryParamChangeEffect, StoryRaceTextAsset, StoryTimelineData, TextDotData, TextRubyData},
        Cute_UI_Assembly::AtlasReference,
        UnityEngine_CoreModule::{GameObject, Texture2D, Object}
    }, symbols::GCHandle, types::*
}};

pub const ASSET_PATH_PREFIX: &str = "assets/_gallopresources/bundle/resources/";

pub struct RequestInfo {
    pub name_handle: GCHandle,
    pub bundle: usize // *mut Il2CppObject (this)
}
impl RequestInfo {
    pub fn name(&self) -> *mut Il2CppString {
        self.name_handle.target() as _
    }
}
pub static REQUEST_INFOS: Lazy<Mutex<FnvHashMap<usize, RequestInfo>>> = Lazy::new(|| Mutex::default());

pub fn check_asset_bundle_name(this: *mut Il2CppObject, metadata: &AssetMetadata) -> bool {
    if let Some(meta_bundle_name) = &metadata.bundle_name {
        let name_ptr = Object::get_name(this);
        if Hachimi::instance().game.region == Region::Japan {
            if !name_ptr.is_null() {
                let logical_name = unsafe { (*name_ptr).as_utf16str().path_filename() };

                if let Some(real_hash) = crate::il2cpp::sql::MetaData::get_hash(&logical_name.to_string()) {
                    if real_hash == *meta_bundle_name {
                        return true;
                    } else {
                        warn!("Expected bundle {}, got {}", meta_bundle_name, real_hash);
                        return false;
                    }
                }
    
                return false;
            }
        } else {
            return true; // Unsolved for other regions for now
        }

        warn!("Failed to resolve bundle path for metadata check!");
    }

    true
}

type LoadAssetFn = extern "C" fn(this: *mut Il2CppObject, name: *mut Il2CppString, type_: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn LoadAsset_Internal(this: *mut Il2CppObject, name: *mut Il2CppString, type_: *mut Il2CppObject) -> *mut Il2CppObject {
    let asset = get_orig_fn!(LoadAsset_Internal, LoadAssetFn)(this, name, type_);
    on_LoadAsset(this, asset, name);
    asset
}

pub fn LoadAsset_Internal_orig(this: *mut Il2CppObject, name: *mut Il2CppString, type_: *mut Il2CppObject) -> *mut Il2CppObject {
    get_orig_fn!(LoadAsset_Internal, LoadAssetFn)(this, name, type_)
}

type LoadAssetAsyncFn = extern "C" fn(this: *mut Il2CppObject, name: *mut Il2CppString, type_: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn LoadAssetAsync_Internal(this: *mut Il2CppObject, name: *mut Il2CppString, type_: *mut Il2CppObject) -> *mut Il2CppObject {
    let request = get_orig_fn!(LoadAssetAsync_Internal, LoadAssetAsyncFn)(this, name, type_);
    let info = RequestInfo {
        name_handle: GCHandle::new(name as _, false), // is name even guaranteed to survive in memory..?
        bundle: this as usize
    };
    REQUEST_INFOS.lock().unwrap().insert(request as usize, info);
    request
}

type OnLoadAssetFn = fn(bundle: *mut Il2CppObject, asset: *mut Il2CppObject, name: &Utf16Str);
pub fn on_LoadAsset(bundle: *mut Il2CppObject, asset: *mut Il2CppObject, name: *mut Il2CppString) {
    let class = unsafe { (*asset).klass() };
    //debug!("{} {}", unsafe { std::ffi::CStr::from_ptr((*class).name).to_str().unwrap() }, unsafe { (*name).as_utf16str() });

    let handler: OnLoadAssetFn = if class == GameObject::class() {
        GameObject::on_LoadAsset
    }
    else if class == StoryTimelineData::class() {
        StoryTimelineData::on_LoadAsset
    }
    else if class == Texture2D::class() {
        Texture2D::on_LoadAsset
    }
    else if class == AtlasReference::class() {
        AtlasReference::on_LoadAsset
    }
    else if class == StoryRaceTextAsset::class() {
        StoryRaceTextAsset::on_LoadAsset
    }
    else if class == TextRubyData::class() {
        TextRubyData::on_LoadAsset
    }
    else if class == TextDotData::class() {
        TextDotData::on_LoadAsset
    }
    else if class == StoryParamChangeEffect::class() {
        StoryParamChangeEffect::on_LoadAsset
    }
    else {
        return;
    };

    handler(bundle, asset, unsafe { (*name).as_utf16str() });
}

type LoadFromFileInternalFn = extern "C" fn(path: *mut Il2CppString, crc: u32, offset: u64) -> *mut Il2CppObject;
extern "C" fn LoadFromFile_Internal(path: *mut Il2CppString, crc: u32, offset: u64) -> *mut Il2CppObject {
    get_orig_fn!(LoadFromFile_Internal, LoadFromFileInternalFn)(path, crc, offset)
}

pub fn LoadFromFile_Internal_orig(path: *mut Il2CppString, crc: u32, offset: u64) -> *mut Il2CppObject {
    LoadFromFile_Internal(path, crc, offset)
}

def_method_wrapper_fn!(LoadFromMemoryAsync_Internal, LOADFROMMEMORYASYNC_ADDR, *mut Il2CppObject, binary: *mut Il2CppObject, crc: u32);

pub fn load_from_memory_async(binary: &[u8]) -> (*mut Il2CppObject, *mut Il2CppObject) {
    let array = il2cpp_array_new(Byte::class(), binary.len());
    unsafe {
        std::ptr::copy_nonoverlapping(
            binary.as_ptr(),
            (array as *mut u8).offset(0x20),
            binary.len()
        );
    }
    let request = LoadFromMemoryAsync_Internal(array as *mut Il2CppObject, 0);
    (array as *mut Il2CppObject, request)
}

pub fn init(_UnityEngine_AssetBundleModule: *const Il2CppImage) {
    //get_class_or_return!(UnityEngine_AssetBundleModule, UnityEngine, AssetBundle);
    unsafe {
        LOADFROMMEMORYASYNC_ADDR = il2cpp_resolve_icall(
            c"UnityEngine.AssetBundle::LoadFromMemoryAsync_Internal(System.Byte[],System.UInt32)".as_ptr()
        );
    }

    let LoadAsset_Internal_addr = il2cpp_resolve_icall(
        c"UnityEngine.AssetBundle::LoadAsset_Internal(System.String,System.Type)".as_ptr()
    );
    let LoadAssetAsync_Internal_addr = il2cpp_resolve_icall(
        c"UnityEngine.AssetBundle::LoadAssetAsync_Internal(System.String,System.Type)".as_ptr()
    );
    let LoadFromFile_Internal_addr = il2cpp_resolve_icall(
        c"UnityEngine.AssetBundle::LoadFromFile_Internal(System.String,System.UInt32,System.UInt64)".as_ptr()
    );

    new_hook!(LoadAsset_Internal_addr, LoadAsset_Internal);
    new_hook!(LoadAssetAsync_Internal_addr, LoadAssetAsync_Internal);
    new_hook!(LoadFromFile_Internal_addr, LoadFromFile_Internal);
}