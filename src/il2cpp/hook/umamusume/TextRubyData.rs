use widestring::Utf16Str;
use std::ffi::c_void;

use crate::{
    core::{Hachimi, hachimi::CustomRubyBlock},
    il2cpp::{
        api::{il2cpp_array_new, il2cpp_class_get_method_from_name, il2cpp_field_get_type, il2cpp_class_from_type, il2cpp_object_new, il2cpp_runtime_invoke, il2cpp_string_new_utf16},
        symbols::{get_field_from_name, set_field_object_value, set_field_value},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut RUBYBLOCKDATA_CLASS: *mut Il2CppClass = 0 as _;
static mut RUBYDATA_CLASS: *mut Il2CppClass = 0 as _;
static mut DATAARRAY_FIELD: *mut FieldInfo = 0 as _;

static mut START_INDEX_FIELD: *mut FieldInfo = 0 as _;
static mut END_INDEX_FIELD: *mut FieldInfo = 0 as _;
static mut CHAR_X_FIELD: *mut FieldInfo = 0 as _;
static mut CHAR_Y_FIELD: *mut FieldInfo = 0 as _;
static mut RUBY_TEXT_FIELD: *mut FieldInfo = 0 as _;
static mut BLOCK_INDEX_FIELD: *mut FieldInfo = 0 as _;
static mut RUBY_DATA_LIST_FIELD: *mut FieldInfo = 0 as _;

fn set_DataArray(this: *mut Il2CppObject, value: *mut Il2CppArray) {
    set_field_object_value(this, unsafe { DATAARRAY_FIELD }, value as *mut Il2CppObject);
}

unsafe fn inject_custom_ruby_blocks(this: *mut Il2CppObject, blocks: &[CustomRubyBlock]) {
    let list_class = il2cpp_class_from_type(il2cpp_field_get_type(RUBY_DATA_LIST_FIELD));
    let list_ctor = il2cpp_class_get_method_from_name(list_class, c".ctor".as_ptr(), 0);
    let list_add = il2cpp_class_get_method_from_name(list_class, c"Add".as_ptr(), 1);

    let block_ctor = il2cpp_class_get_method_from_name(RUBYBLOCKDATA_CLASS, c".ctor".as_ptr(), 0);
    let data_ctor = il2cpp_class_get_method_from_name(RUBYDATA_CLASS, c".ctor".as_ptr(), 0);

    let array = il2cpp_array_new(RUBYBLOCKDATA_CLASS, blocks.len());
    let array_elements_ptr = (array as *mut u8).offset(0x20) as *mut *mut Il2CppObject;

    let invalid_idx: i32 = -1;

    for (i, block) in blocks.iter().enumerate() {
        let list_obj = il2cpp_object_new(list_class);
        if !list_ctor.is_null() {
            il2cpp_runtime_invoke(list_ctor, list_obj as *mut c_void, std::ptr::null_mut(), std::ptr::null_mut());
        }

        for ruby in &block.rubies {
            let data_obj = il2cpp_object_new(RUBYDATA_CLASS);
            if !data_ctor.is_null() {
                il2cpp_runtime_invoke(data_ctor, data_obj as *mut c_void, std::ptr::null_mut(), std::ptr::null_mut());
            }

            set_field_value(data_obj, START_INDEX_FIELD, &invalid_idx);
            set_field_value(data_obj, END_INDEX_FIELD, &invalid_idx);
            set_field_value(data_obj, CHAR_X_FIELD, &ruby.char_x);
            set_field_value(data_obj, CHAR_Y_FIELD, &ruby.char_y);

            let utf16_ruby: Vec<u16> = ruby.ruby_text.encode_utf16().chain(std::iter::once(0)).collect();
            let il2cpp_str = il2cpp_string_new_utf16(utf16_ruby.as_ptr(), (utf16_ruby.len() - 1) as i32);
            set_field_object_value(data_obj, RUBY_TEXT_FIELD, il2cpp_str as *mut Il2CppObject);

            let mut args = [data_obj as *mut c_void];
            il2cpp_runtime_invoke(list_add, list_obj as *mut c_void, args.as_mut_ptr(), std::ptr::null_mut());
        }

        let block_obj = il2cpp_object_new(RUBYBLOCKDATA_CLASS);
        if !block_ctor.is_null() {
            il2cpp_runtime_invoke(block_ctor, block_obj as *mut c_void, std::ptr::null_mut(), std::ptr::null_mut());
        }

        set_field_value(block_obj, BLOCK_INDEX_FIELD, &block.block_index);
        set_field_object_value(block_obj, RUBY_DATA_LIST_FIELD, list_obj);

        *array_elements_ptr.offset(i as isize) = block_obj;
    }

    set_DataArray(this, array);
}

pub fn on_LoadAsset(_bundle: *mut Il2CppObject, this: *mut Il2CppObject, name: &Utf16Str) {
    let asset_name = name.to_string();
    let localized_data = Hachimi::instance().localized_data.load();

    if let Some(custom_blocks) = localized_data.load_custom_story_ruby(&asset_name) {
        unsafe { inject_custom_ruby_blocks(this, &custom_blocks); }
    }
    else if localized_data.config.remove_ruby {
        let empty_array = unsafe { il2cpp_array_new(RUBYBLOCKDATA_CLASS, 0) };
        set_DataArray(this, empty_array);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "", TextRubyData);

    unsafe {
        CLASS = TextRubyData;
        DATAARRAY_FIELD = get_field_from_name(TextRubyData, c"DataArray");
    }

    // Putting nested classes inside parent module due to lack of usage
    find_nested_class_or_return!(TextRubyData, RubyBlockData);
    find_nested_class_or_return!(TextRubyData, RubyData);

    unsafe {
        RUBYBLOCKDATA_CLASS = RubyBlockData;
        RUBYDATA_CLASS = RubyData;

        START_INDEX_FIELD = get_field_from_name(RUBYDATA_CLASS, c"StartIndex");
        END_INDEX_FIELD = get_field_from_name(RUBYDATA_CLASS, c"EndIndex");
        CHAR_X_FIELD = get_field_from_name(RUBYDATA_CLASS, c"CharX");
        CHAR_Y_FIELD = get_field_from_name(RUBYDATA_CLASS, c"CharY");
        RUBY_TEXT_FIELD = get_field_from_name(RUBYDATA_CLASS, c"RubyText");

        BLOCK_INDEX_FIELD = get_field_from_name(RUBYBLOCKDATA_CLASS, c"BlockIndex");
        RUBY_DATA_LIST_FIELD = get_field_from_name(RUBYBLOCKDATA_CLASS, c"RubyDataList");
    }
}