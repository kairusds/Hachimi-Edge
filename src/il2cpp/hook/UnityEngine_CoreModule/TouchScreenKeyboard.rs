use crate::{il2cpp::{symbols::{get_method_addr, get_method_overload_addr}, types::*}};
use super::TouchScreenKeyboardType;

static mut OPEN_ADDR: usize = 0;
impl_addr_wrapper_fn!(
    Open, 
    OPEN_ADDR, 
    *mut Il2CppObject, 
    text: *mut Il2CppString,
    keyboardType: TouchScreenKeyboardType::KeyboardType,
    autocorrection: bool,
    multiline: bool,
    secure: bool
);

static mut GET_TEXT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_text, GET_TEXT_ADDR, *mut Il2CppString, this: *mut Il2CppObject);

#[repr(i32)]
#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Visible,
    Done,
    Canceled,
    LostFocus
}

static mut GET_STATUS_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_status, GET_STATUS_ADDR, Status, this: *mut Il2CppObject);

static mut SET_ACTIVE_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_active, SET_ACTIVE_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut GET_SELECTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_selection, GET_SELECTION_ADDR, RangeInt, this: *mut Il2CppObject);

static mut SET_SELECTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_selection, SET_SELECTION_ADDR, (), this: *mut Il2CppObject, value: RangeInt);

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, TouchScreenKeyboard);

    unsafe {
        OPEN_ADDR = get_method_overload_addr(
            TouchScreenKeyboard, 
            "Open", 
            &[
                Il2CppTypeEnum_IL2CPP_TYPE_STRING,   // String text
                Il2CppTypeEnum_IL2CPP_TYPE_VALUETYPE, // TouchScreenKeyboardType (Enum)
                Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN,  // Boolean autocorrection
                Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN,  // Boolean multiline
                Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN,  // Boolean secure
            ]
        );
        GET_TEXT_ADDR = get_method_addr(TouchScreenKeyboard, c"get_text", 0);
        GET_STATUS_ADDR = get_method_addr(TouchScreenKeyboard, c"get_status", 0);
        SET_ACTIVE_ADDR = get_method_addr(TouchScreenKeyboard, c"set_active", 1);
        GET_SELECTION_ADDR = get_method_addr(TouchScreenKeyboard, c"get_selection", 0);
        SET_SELECTION_ADDR = get_method_addr(TouchScreenKeyboard, c"set_selection", 1);
    }
}
