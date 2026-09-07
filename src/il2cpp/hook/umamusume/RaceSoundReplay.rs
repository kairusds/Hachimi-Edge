use crate::il2cpp::{
    api::il2cpp_class_is_assignable_from,
    ext::Il2CppObjectExt,
    symbols::{get_field_from_name, get_method_addr},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn is_replay_sound(obj: *mut Il2CppObject) -> bool {
    if class().is_null() || obj.is_null() {
        return false;
    }

    let obj_class = unsafe { (*obj).klass() };
    !obj_class.is_null() && il2cpp_class_is_assignable_from(class(), obj_class)
}

def_field_object_accessors!(get_BGMController, set_BGMController, BGM_CONTROLLER_FIELD, Il2CppObject);

def_method_wrapper_fn!(GetBGMVolume, GET_BGM_VOLUME_ADDR, f32, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceSoundReplay);

    unsafe {
        CLASS = RaceSoundReplay;
        BGM_CONTROLLER_FIELD = get_field_from_name(RaceSoundReplay, c"_BGMController");
        GET_BGM_VOLUME_ADDR = get_method_addr(RaceSoundReplay, c"GetBGMVolume", 0);
    }
}
