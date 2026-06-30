use std::{
    collections::{HashMap, HashSet},
    ptr::null_mut,
    sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::il2cpp::{
    ext::Il2CppStringExt,
    hook::{
        UnityEngine_CoreModule::{Component, GameObject, Object, Transform},
        umamusume::ModelController,
    },
    symbols::IEnumerable,
    types::*,
};

pub type DisabledHeadStore = Lazy<Mutex<HashMap<i32, HashSet<usize>>>>;

pub fn new_disabled_head_store() -> Mutex<HashMap<i32, HashSet<usize>>> {
    Mutex::new(HashMap::new())
}

pub fn first_enumerable_item(value: *mut Il2CppObject) -> *mut Il2CppObject {
    let enumerable = IEnumerable::<*mut Il2CppObject>::from(value);
    let Some(enumerator) = enumerable.enumerator() else {
        return null_mut();
    };
    let Some(mut iter) = enumerator.iter() else {
        return null_mut();
    };
    iter.find(|item| !item.is_null()).unwrap_or(null_mut())
}

pub fn hide_head_parts(
    store: &DisabledHeadStore,
    model_controller: *mut Il2CppObject,
    index: i32,
) {
    let owner = ModelController::get_OwnerObject(model_controller);
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

pub fn restore_disabled_heads(
    store: &DisabledHeadStore,
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
