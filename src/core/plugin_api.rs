use std::ffi::{c_char, c_void, CStr};

use once_cell::sync::OnceCell;
use egui::Align;

use crate::{core::{gui, Hachimi, Interceptor}, il2cpp::{self, types::{il2cpp_array_size_t, FieldInfo, Il2CppArray, Il2CppClass, Il2CppImage, Il2CppObject, Il2CppThread, Il2CppTypeEnum, MethodInfo}}};

const VERSION: i32 = 2;

static PLUGIN_VTABLE: OnceCell<Vtable> = OnceCell::new();

pub type HachimiInitFn = extern "C" fn(vtable: *const Vtable, version: i32) -> InitResult;
pub type GuiMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type GuiMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type GuiUiCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum InitResult {
    Error,
    Ok
}

impl InitResult {
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Ok => true,
            _ => false
        }
    }
}

unsafe extern "C" fn hachimi_instance() -> *const Hachimi {
    Hachimi::instance().as_ref()
}

unsafe extern "C" fn hachimi_get_interceptor(this: *const Hachimi) -> *const Interceptor {
    &(*this).interceptor
}

unsafe extern "C" fn interceptor_hook(
    this: *const Interceptor, orig_addr: *mut c_void, hook_addr: *mut c_void
) -> *mut c_void {
    (*this).hook(orig_addr as _, hook_addr as _)
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0) as _
}

unsafe extern "C" fn interceptor_hook_vtable(
    this: *const Interceptor, vtable: *mut *mut c_void, vtable_index: usize, hook_addr: *mut c_void
) -> *mut c_void {
    (*this).hook_vtable(vtable as _, vtable_index as _, hook_addr as _)
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0) as _
}

unsafe extern "C" fn interceptor_get_trampoline_addr(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void {
    (*this).get_trampoline_addr(hook_addr as _) as _
}

unsafe extern "C" fn interceptor_unhook(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void {
    if let Some(handle) = (*this).unhook(hook_addr as _) {
        handle.orig_addr as _
    }
    else {
        0 as _
    }
}

unsafe extern "C" fn il2cpp_resolve_symbol(name: *const c_char) -> *mut c_void {
    let Ok(name) = CStr::from_ptr(name).to_str() else {
        return 0 as _;
    };
    il2cpp::symbols::dlsym(name) as _
}

unsafe extern "C" fn il2cpp_get_assembly_image(assembly_name: *const c_char) -> *const Il2CppImage {
    il2cpp::symbols::get_assembly_image(CStr::from_ptr(assembly_name))
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_class(
    image: *const Il2CppImage, namespace: *const c_char, class_name: *const c_char
) -> *mut Il2CppClass {
    il2cpp::symbols::get_class(image, CStr::from_ptr(namespace), CStr::from_ptr(class_name))
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_method(
    class: *mut Il2CppClass, name: *const c_char, args_count: i32
) -> *const MethodInfo {
    il2cpp::symbols::get_method(class, CStr::from_ptr(name), args_count)
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_method_overload(
    class: *mut Il2CppClass, name: *const c_char, params: *const Il2CppTypeEnum, param_count: usize
) -> *const MethodInfo {
    let name = CStr::from_ptr(name).to_string_lossy();
    let params = std::slice::from_raw_parts(params, param_count);
    il2cpp::symbols::get_method_overload(class, &name, params)
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_method_addr(
    class: *mut Il2CppClass, name: *const c_char, args_count: i32
) -> *mut c_void {
    il2cpp::symbols::get_method_addr(class, CStr::from_ptr(name), args_count) as _
}

unsafe extern "C" fn il2cpp_get_method_overload_addr(
    class: *mut Il2CppClass, name: *const c_char, params: *const Il2CppTypeEnum, param_count: usize
) -> *mut c_void {
    let name = CStr::from_ptr(name).to_string_lossy();
    let params = std::slice::from_raw_parts(params, param_count);
    il2cpp::symbols::get_method_overload_addr(class, &name, params) as _
}

unsafe extern "C" fn il2cpp_get_method_cached(
    class: *mut Il2CppClass, name: *const c_char, args_count: i32
) -> *const MethodInfo {
    il2cpp::symbols::get_method_cached(class, CStr::from_ptr(name), args_count)
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_method_addr_cached(
    class: *mut Il2CppClass, name: *const c_char, args_count: i32
) -> *mut c_void {
    il2cpp::symbols::get_method_addr_cached(class, CStr::from_ptr(name), args_count) as _
}

unsafe extern "C" fn il2cpp_find_nested_class(
    class: *mut Il2CppClass, name: *const c_char
) -> *mut Il2CppClass {
    il2cpp::symbols::find_nested_class(class, CStr::from_ptr(name))
        .inspect_err(|e| error!("{}", e))
        .unwrap_or(0 as _)
}

unsafe extern "C" fn il2cpp_get_field_from_name(
    class: *mut Il2CppClass, name: *const c_char
) -> *mut FieldInfo {
    il2cpp::api::il2cpp_class_get_field_from_name(class, name)
}

unsafe extern "C" fn il2cpp_get_field_value(
    obj: *mut Il2CppObject, field: *mut FieldInfo, out_value: *mut c_void
) {
    il2cpp::api::il2cpp_field_get_value(obj, field, out_value)
}

unsafe extern "C" fn il2cpp_set_field_value(
    obj: *mut Il2CppObject, field: *mut FieldInfo, value: *const c_void
) {
    il2cpp::api::il2cpp_field_set_value(obj, field, value as _)
}

unsafe extern "C" fn il2cpp_get_static_field_value(
    field: *mut FieldInfo, out_value: *mut c_void
) {
    il2cpp::api::il2cpp_field_static_get_value(field, out_value)
}

unsafe extern "C" fn il2cpp_set_static_field_value(
    field: *mut FieldInfo, value: *const c_void
) {
    il2cpp::api::il2cpp_field_static_set_value(field, value as _)
}

unsafe extern "C" fn il2cpp_unbox(obj: *mut Il2CppObject) -> *mut c_void {
    il2cpp::api::il2cpp_object_unbox(obj)
}

unsafe extern "C" fn il2cpp_get_main_thread() -> *mut Il2CppThread {
    il2cpp::symbols::Thread::main_thread().as_raw()
}

unsafe extern "C" fn il2cpp_get_attached_threads(out_size: *mut usize) -> *mut *mut Il2CppThread {
    il2cpp::api::il2cpp_thread_get_all_attached_threads(out_size)
}

unsafe extern "C" fn il2cpp_schedule_on_thread(thread: *mut Il2CppThread, callback: unsafe extern "C" fn()) {
    il2cpp::symbols::Thread::from_raw(thread).schedule(std::mem::transmute(callback));
}

unsafe extern "C" fn il2cpp_create_array(
    element_type: *mut Il2CppClass, length: il2cpp_array_size_t
) -> *mut Il2CppArray {
    il2cpp::api::il2cpp_array_new(element_type, length)
}

unsafe extern "C" fn il2cpp_get_singleton_like_instance(class: *mut Il2CppClass) -> *mut Il2CppObject {
    il2cpp::symbols::SingletonLike::new(class)
        .map(|s| s.instance())
        .unwrap_or(0 as _)
}

unsafe extern "C" fn log(level: i32, target: *const c_char, message: *const c_char) {
    let target = CStr::from_ptr(target).to_string_lossy();
    let message = CStr::from_ptr(message).to_string_lossy();
    let level = match level {
        1 => log::Level::Error,
        2 => log::Level::Warn,
        3 => log::Level::Info,
        4 => log::Level::Debug,
        5 => log::Level::Trace,

        _ => log::Level::Info
    };
    log!(target: &target, level, "{}", message);
}

unsafe extern "C" fn gui_register_menu_item(
    label: *const c_char,
    callback: Option<GuiMenuCallback>,
    userdata: *mut c_void
) -> bool {
    if label.is_null() {
        return false;
    }
    let Ok(label) = CStr::from_ptr(label).to_str() else {
        return false;
    };
    gui::register_plugin_menu_item(label.to_owned(), callback, userdata);
    true
}

unsafe extern "C" fn gui_register_menu_section(
    callback: Option<GuiMenuSectionCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(callback) = callback else {
        return false;
    };
    gui::register_plugin_menu_section(callback, userdata);
    true
}

unsafe extern "C" fn gui_show_notification(message: *const c_char) -> bool {
    if message.is_null() {
        return false;
    }
    let Ok(message) = CStr::from_ptr(message).to_str() else {
        return false;
    };
    gui::enqueue_plugin_notification(message.to_owned());
    true
}

unsafe fn ui_from_ptr<'a>(ui: *mut c_void) -> Option<&'a mut egui::Ui> {
    if ui.is_null() {
        return None;
    }
    Some(&mut *(ui as *mut egui::Ui))
}

unsafe fn cstr_or_empty(ptr: *const c_char) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

unsafe extern "C" fn gui_ui_heading(ui: *mut c_void, text: *const c_char) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.heading(cstr_or_empty(text));
    true
}

unsafe extern "C" fn gui_ui_label(ui: *mut c_void, text: *const c_char) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.label(cstr_or_empty(text));
    true
}

unsafe extern "C" fn gui_ui_small(ui: *mut c_void, text: *const c_char) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.small(cstr_or_empty(text));
    true
}

unsafe extern "C" fn gui_ui_separator(ui: *mut c_void) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.separator();
    true
}

unsafe extern "C" fn gui_ui_button(ui: *mut c_void, text: *const c_char) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.button(cstr_or_empty(text)).clicked()
}

unsafe extern "C" fn gui_ui_small_button(ui: *mut c_void, text: *const c_char) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.small_button(cstr_or_empty(text)).clicked()
}

unsafe extern "C" fn gui_ui_checkbox(
    ui: *mut c_void,
    text: *const c_char,
    value: *mut bool
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    if value.is_null() { return false; }
    let mut current = *value;
    let changed = ui.checkbox(&mut current, cstr_or_empty(text)).changed();
    if changed {
        *value = current;
    }
    changed
}

unsafe extern "C" fn gui_ui_text_edit_singleline(
    ui: *mut c_void,
    buffer: *mut c_char,
    buffer_len: usize
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    if buffer.is_null() || buffer_len == 0 { return false; }
    let bytes = std::slice::from_raw_parts_mut(buffer as *mut u8, buffer_len);
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(buffer_len);
    let mut value = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let response = ui.add(egui::TextEdit::singleline(&mut value).desired_width(80.0));
    if response.clicked() || response.gained_focus() {
        gui::request_ime();
    }
    if response.gained_focus() {
        response.scroll_to_me(Some(Align::Center));
    }
    let changed = response.changed();
    if changed {
        bytes.fill(0);
        let src = value.as_bytes();
        let copy_len = src.len().min(buffer_len.saturating_sub(1));
        bytes[..copy_len].copy_from_slice(&src[..copy_len]);
    }
    changed
}

unsafe extern "C" fn gui_ui_horizontal(
    ui: *mut c_void,
    callback: Option<GuiUiCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    let Some(callback) = callback else { return false; };
    ui.horizontal(|ui| {
        callback(ui as *mut _ as *mut c_void, userdata);
    });
    true
}

unsafe extern "C" fn gui_ui_grid(
    ui: *mut c_void,
    id: *const c_char,
    columns: usize,
    spacing_x: f32,
    spacing_y: f32,
    callback: Option<GuiUiCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    let Some(callback) = callback else { return false; };
    let id = cstr_or_empty(id);
    egui::Grid::new(id)
        .num_columns(columns)
        .spacing([spacing_x, spacing_y])
        .show(ui, |ui| {
            callback(ui as *mut _ as *mut c_void, userdata);
        });
    true
}

unsafe extern "C" fn gui_ui_end_row(ui: *mut c_void) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.end_row();
    true
}

unsafe extern "C" fn gui_ui_colored_label(
    ui: *mut c_void,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    text: *const c_char
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    ui.colored_label(egui::Color32::from_rgba_unmultiplied(r, g, b, a), cstr_or_empty(text));
    true
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vtable {
    pub hachimi_instance: unsafe extern "C" fn() -> *const Hachimi,
    pub hachimi_get_interceptor: unsafe extern "C" fn(this: *const Hachimi) -> *const Interceptor,

    pub interceptor_hook: unsafe extern "C" fn(
        this: *const Interceptor, orig_addr: *mut c_void, hook_addr: *mut c_void
    ) -> *mut c_void,
    pub interceptor_hook_vtable: unsafe extern "C" fn(
        this: *const Interceptor, vtable: *mut *mut c_void, vtable_index: usize, hook_addr: *mut c_void
    ) -> *mut c_void,
    pub interceptor_get_trampoline_addr: unsafe extern "C" fn(
        this: *const Interceptor, hook_addr: *mut c_void
    ) -> *mut c_void,
    pub interceptor_unhook: unsafe extern "C" fn(this: *const Interceptor, hook_addr: *mut c_void) -> *mut c_void,

    pub il2cpp_resolve_symbol: unsafe extern "C" fn(name: *const c_char) -> *mut c_void,
    pub il2cpp_get_assembly_image: unsafe extern "C" fn(assembly_name: *const c_char) -> *const Il2CppImage,
    pub il2cpp_get_class: unsafe extern "C" fn(
        image: *const Il2CppImage, namespace: *const c_char, class_name: *const c_char
    ) -> *mut Il2CppClass,
    pub il2cpp_get_method: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, args_count: i32
    ) -> *const MethodInfo,
    pub il2cpp_get_method_overload: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, params: *const Il2CppTypeEnum, param_count: usize
    ) -> *const MethodInfo,
    pub il2cpp_get_method_addr: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, args_count: i32
    ) -> *mut c_void,
    pub il2cpp_get_method_overload_addr: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, params: *const Il2CppTypeEnum, param_count: usize
    ) -> *mut c_void,
        pub il2cpp_get_method_cached: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, args_count: i32
    ) -> *const MethodInfo,
    pub il2cpp_get_method_addr_cached: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char, args_count: i32
    ) -> *mut c_void,
    pub il2cpp_find_nested_class: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char
    ) -> *mut Il2CppClass,
    pub il2cpp_get_field_from_name: unsafe extern "C" fn(
        class: *mut Il2CppClass, name: *const c_char
    ) -> *mut FieldInfo,
    pub il2cpp_get_field_value: unsafe extern "C" fn(
        obj: *mut Il2CppObject, field: *mut FieldInfo, out_value: *mut c_void
    ),
    pub il2cpp_set_field_value: unsafe extern "C" fn(
        obj: *mut Il2CppObject, field: *mut FieldInfo, value: *const c_void
    ),
    pub il2cpp_get_static_field_value: unsafe extern "C" fn(
        field: *mut FieldInfo, out_value: *mut c_void
    ),
    pub il2cpp_set_static_field_value: unsafe extern "C" fn(
        field: *mut FieldInfo, value: *const c_void
    ),
    pub il2cpp_unbox: unsafe extern "C" fn(obj: *mut Il2CppObject) -> *mut c_void,
    pub il2cpp_get_main_thread: unsafe extern "C" fn() -> *mut Il2CppThread,
    pub il2cpp_get_attached_threads: unsafe extern "C" fn(out_size: *mut usize) -> *mut *mut Il2CppThread,
    pub il2cpp_schedule_on_thread: unsafe extern "C" fn(thread: *mut Il2CppThread, callback: unsafe extern "C" fn()),
    pub il2cpp_create_array: unsafe extern "C" fn(
        element_type: *mut Il2CppClass, length: il2cpp_array_size_t
    ) -> *mut Il2CppArray,
    pub il2cpp_get_singleton_like_instance: unsafe extern "C" fn(class: *mut Il2CppClass) -> *mut Il2CppObject,

    pub log: unsafe extern "C" fn(level: i32, target: *const c_char, message: *const c_char),
    pub gui_register_menu_item: unsafe extern "C" fn(
        label: *const c_char,
        callback: Option<GuiMenuCallback>,
        userdata: *mut c_void
    ) -> bool,
    pub gui_register_menu_section: unsafe extern "C" fn(
        callback: Option<GuiMenuSectionCallback>,
        userdata: *mut c_void
    ) -> bool,
    pub gui_show_notification: unsafe extern "C" fn(message: *const c_char) -> bool,
    pub gui_ui_heading: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_label: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_small: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_separator: unsafe extern "C" fn(ui: *mut c_void) -> bool,
    pub gui_ui_button: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_small_button: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char) -> bool,
    pub gui_ui_checkbox: unsafe extern "C" fn(ui: *mut c_void, text: *const c_char, value: *mut bool) -> bool,
    pub gui_ui_text_edit_singleline: unsafe extern "C" fn(
        ui: *mut c_void,
        buffer: *mut c_char,
        buffer_len: usize
    ) -> bool,
    pub gui_ui_horizontal: unsafe extern "C" fn(
        ui: *mut c_void,
        callback: Option<GuiUiCallback>,
        userdata: *mut c_void
    ) -> bool,
    pub gui_ui_grid: unsafe extern "C" fn(
        ui: *mut c_void,
        id: *const c_char,
        columns: usize,
        spacing_x: f32,
        spacing_y: f32,
        callback: Option<GuiUiCallback>,
        userdata: *mut c_void
    ) -> bool,
    pub gui_ui_end_row: unsafe extern "C" fn(ui: *mut c_void) -> bool,
    pub gui_ui_colored_label: unsafe extern "C" fn(
        ui: *mut c_void,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        text: *const c_char
    ) -> bool,
}

impl Vtable {
    pub const VALUE: Self = Self {
        hachimi_instance,
        hachimi_get_interceptor,
        interceptor_hook,
        interceptor_hook_vtable,
        interceptor_get_trampoline_addr,
        interceptor_unhook,
        il2cpp_resolve_symbol,
        il2cpp_get_assembly_image,
        il2cpp_get_class,
        il2cpp_get_method,
        il2cpp_get_method_overload,
        il2cpp_get_method_addr,
        il2cpp_get_method_overload_addr,
        il2cpp_get_method_cached,
        il2cpp_get_method_addr_cached,
        il2cpp_find_nested_class,
        il2cpp_get_field_from_name,
        il2cpp_get_field_value,
        il2cpp_set_field_value,
        il2cpp_get_static_field_value,
        il2cpp_set_static_field_value,
        il2cpp_unbox,
        il2cpp_get_main_thread,
        il2cpp_get_attached_threads,
        il2cpp_schedule_on_thread,
        il2cpp_create_array,
        il2cpp_get_singleton_like_instance,
        log,
        gui_register_menu_item,
        gui_register_menu_section,
        gui_show_notification,
        gui_ui_heading,
        gui_ui_label,
        gui_ui_small,
        gui_ui_separator,
        gui_ui_button,
        gui_ui_small_button,
        gui_ui_checkbox,
        gui_ui_text_edit_singleline,
        gui_ui_horizontal,
        gui_ui_grid,
        gui_ui_end_row,
        gui_ui_colored_label,
    };

    pub fn instantiate() -> Self {
        Self::VALUE.clone()
    }
}

pub struct Plugin {
    pub name: String,
    pub init_fn: HachimiInitFn
}

impl Plugin {
    pub fn init(&self) -> InitResult {
        let vtable = PLUGIN_VTABLE.get_or_init(Vtable::instantiate);
        (self.init_fn)(vtable as *const Vtable, VERSION)
    }
}
