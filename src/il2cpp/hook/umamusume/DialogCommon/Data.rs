use std::ptr::null_mut;

use crate::il2cpp::{api::{il2cpp_class_from_il2cpp_type, il2cpp_object_new, il2cpp_runtime_object_init}, hook::umamusume::DialogCommon::FormType, symbols::{get_field_from_name, get_method_addr}, types::*};
static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn new() -> *mut Il2CppObject {
    let object = il2cpp_object_new(class());
    il2cpp_runtime_object_init(object);
    object
}

static mut SETSIMPLEONEBUTTONMESSAGE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetSimpleOneButtonMessage, SETSIMPLEONEBUTTONMESSAGE_ADDR, *mut Il2CppObject,
    this: *mut Il2CppObject, header_text: *mut Il2CppString,
    message: *mut Il2CppString, on_click_center_button: *mut Il2CppDelegate,
    close_text_id: i32, dialog_form_type: FormType
);

def_method_wrapper_fn!(SetSimpleThreeButtonMessage, SETSIMPLETHREEBUTTONMESSAGE_ADDR, *mut Il2CppObject,
    this: *mut Il2CppObject, header_text: *mut Il2CppString,
    message: *mut Il2CppString, on_right: *mut Il2CppDelegate, right_text_id: i32,
    on_center: *mut Il2CppDelegate, center_text_id: i32,
    on_left: *mut Il2CppDelegate, left_text_id: i32, dialog_form_type: FormType
);

def_field_object_accessors!(set set_LeftButtonText, LEFTBUTTONTEXT_FIELD, Il2CppString);
def_field_object_accessors!(set set_CenterButtonText, CENTERBUTTONTEXT_FIELD, Il2CppString);
def_field_object_accessors!(set set_RightButtonText, RIGHTBUTTONTEXT_FIELD, Il2CppString);

static mut CENTERBUTTONCALLBACK_FIELD: *mut FieldInfo = null_mut();
pub fn button_callback_class() -> *mut Il2CppClass {
    unsafe { il2cpp_class_from_il2cpp_type((*CENTERBUTTONCALLBACK_FIELD).type_) }
}

pub fn init(DialogCommon: *mut Il2CppClass) {
    find_nested_class_or_return!(DialogCommon, Data);

    unsafe {
        CLASS = Data;
        SETSIMPLEONEBUTTONMESSAGE_ADDR = get_method_addr(Data, c"SetSimpleOneButtonMessage", 5);
        SETSIMPLETHREEBUTTONMESSAGE_ADDR = get_method_addr(Data, c"SetSimpleThreeButtonMessage", 9);

        LEFTBUTTONTEXT_FIELD = get_field_from_name(Data, c"LeftButtonText");
        CENTERBUTTONTEXT_FIELD = get_field_from_name(Data, c"CenterButtonText");
        RIGHTBUTTONTEXT_FIELD = get_field_from_name(Data, c"RightButtonText");
        CENTERBUTTONCALLBACK_FIELD = get_field_from_name(Data, c"CenterButtonCallBack");
    }
}
