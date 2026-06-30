use crate::{
    il2cpp::{
        symbols::{get_method_addr, get_field_from_name, get_type_object_for_class},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut GET_GAMEOBJECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_gameObject, GET_GAMEOBJECT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

def_field_object_accessors!(get__Label, set__Label, _LABEL_FIELD, Il2CppObject);
def_field_object_accessors!(get_canvasGroup, set_canvasGroup, _CANVASGROUP_FIELD, Il2CppObject);
def_field_value_accessors!(get__displayTime, set__displayTime, _DISPLAYTIME_FIELD, f32);
def_field_value_accessors!(get__fadeOutTime, set__fadeOutTime, _FADEOUTTIME_FIELD, f32);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Notification);

    unsafe {
        CLASS = Notification;
        TYPE_OBJECT = get_type_object_for_class(Notification);
        GET_GAMEOBJECT_ADDR = get_method_addr(Notification, c"get_gameObject", 0);
        _LABEL_FIELD = get_field_from_name(Notification, c"_Label");
        _CANVASGROUP_FIELD = get_field_from_name(Notification, c"canvasGroup");
        _DISPLAYTIME_FIELD = get_field_from_name(Notification, c"_displayTime");
        _FADEOUTTIME_FIELD = get_field_from_name(Notification, c"_fadeOutTime");
    }
}
