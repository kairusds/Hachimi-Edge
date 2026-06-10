use std::{ffi::{c_char, c_void, CStr, CString}, sync::atomic::AtomicI32};

use once_cell::sync::OnceCell;
use egui::Align;

use crate::{core::{Hachimi, Interceptor, gui}, il2cpp::{self, types::{FieldInfo, Il2CppArray, Il2CppClass, Il2CppImage, Il2CppMethodPointer, Il2CppObject, Il2CppString, Il2CppThread, Il2CppTypeEnum, MethodInfo, il2cpp_array_size_t}}};

const VERSION: i32 = 3;

static PLUGIN_VTABLE: OnceCell<Vtable> = OnceCell::new();
static DATA_DIR_CSTR: once_cell::sync::OnceCell<CString> = once_cell::sync::OnceCell::new();
static DATA_PATH_CSTR: once_cell::sync::OnceCell<CString> = once_cell::sync::OnceCell::new();

pub type HachimiGetApiFn = extern "C" fn(name: *const c_char) -> *mut c_void;
pub type HachimiInitFn = extern "C" fn(vtable: *const Vtable, version: i32) -> InitResult;
pub type HachimiInitV3Fn = extern "C" fn(get_api: HachimiGetApiFn, version: i32) -> InitResult;
pub type GuiMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type GuiMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type GuiUiCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type GameInitializedCallback = unsafe extern "C" fn(userdata: *mut c_void);
pub type PresentCallback = unsafe extern "C" fn(swapchain: *mut c_void, userdata: *mut c_void);
pub type GuiWindowCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

static NEXT_PLUGIN_WINDOW_ID: AtomicI32 = AtomicI32::new(0);

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

unsafe extern "C" fn il2cpp_resolve_icall(name: *const c_char) -> Il2CppMethodPointer {
    il2cpp::api::il2cpp_resolve_icall(name)
}

unsafe extern "C" fn il2cpp_class_get_methods(klass: *mut Il2CppClass, iter: *mut *mut c_void) -> *const MethodInfo {
    il2cpp::api::il2cpp_class_get_methods(klass, iter)
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

unsafe extern "C" fn il2cpp_object_new(klass: *const Il2CppClass) -> *mut Il2CppObject{
    return il2cpp::api::il2cpp_object_new(klass);
}

unsafe extern "C" fn il2cpp_runtime_object_init(object: *mut Il2CppObject) {
    il2cpp::api::il2cpp_runtime_object_init(object);
}

unsafe extern "C" fn il2cpp_string_new(text: *const c_char) -> *mut Il2CppString {
    il2cpp::api::il2cpp_string_new(text)
}

unsafe extern "C" fn il2cpp_string_chars(s: *mut Il2CppString) -> *mut u16 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    il2cpp::api::il2cpp_string_chars(s)
}

unsafe extern "C" fn il2cpp_string_length(s: *mut Il2CppString) -> i32 {
    if s.is_null() {
        return 0;
    }
    il2cpp::api::il2cpp_string_length(s)
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

unsafe extern "C" fn hachimi_register_on_game_initialized(
    callback: Option<GameInitializedCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(callback) = callback else {
        return false;
    };
    let hachimi = Hachimi::instance();
    let mut callbacks = hachimi.plugin_init_callbacks.lock().unwrap();
    callbacks.push((callback as usize, userdata as usize));
    true
}

#[cfg(target_os = "windows")]
unsafe extern "C" fn hachimi_register_present_callback(
    callback: Option<PresentCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(callback) = callback else {
        return false;
    };
    let hachimi = Hachimi::instance();
    let mut callbacks = hachimi.present_callbacks.lock().unwrap();
    callbacks.push((callback as usize, userdata as usize));
    true
}

#[cfg(not(target_os = "windows"))]
unsafe extern "C" fn hachimi_register_present_callback(
    _callback: Option<PresentCallback>,
    _userdata: *mut c_void
) -> bool {
    false
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

    let id = ui.make_persistent_id(buffer as usize);
    let mut value = ui.memory(|mem| {
        mem.data.get_temp::<String>(id)
    }).unwrap_or_else(|| String::from_utf8_lossy(&bytes[..end]).into_owned());
    let original_value = value.clone();

    let response = ui.add(
        egui::TextEdit::singleline(&mut value)
            .id(id)
            .desired_width(80.0)
    );
    #[cfg(target_os = "android")]
    gui::handle_android_keyboard(&response, &mut value);

    if response.gained_focus() {
        response.scroll_to_me(Some(Align::Center));
    }

    ui.memory_mut(|mem| mem.data.insert_temp(id, value.clone()));

    let changed = value != original_value;
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

unsafe extern "C" fn gui_ui_combo_menu(
    ui: *mut c_void,
    id: *const c_char,
    selected_index: *mut i32,
    items: *const *const c_char,
    item_count: usize,
    search_term: *mut c_char,
    search_term_len: usize,
) -> bool {
    let Some(ui) = ui_from_ptr(ui) else { return false; };
    if selected_index.is_null() || items.is_null() || item_count == 0 {
        return false;
    }

    let id_str = cstr_or_empty(id);
    let button_id = ui.make_persistent_id(id_str);
    let popup_id = button_id.with("popup");

    let scale = gui::get_scale(ui.ctx());
    let fixed_width = 145.0 * scale;
    let row_height = 24.0 * scale;
    let padding = ui.spacing().button_padding;

    let current_idx = *selected_index as usize;
    let selected_text = if current_idx < item_count {
        let item_ptr = *items.add(current_idx);
        if !item_ptr.is_null() {
            CStr::from_ptr(item_ptr).to_str().unwrap_or("Unknown")
        } else {
            "Unknown"
        }
    } else {
        "Unknown"
    };

    let (rect, _) = ui.allocate_exact_size(egui::vec2(fixed_width, row_height), egui::Sense::hover());
    let button_res = ui.interact(rect, button_id, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let visuals = if is_open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&button_res)
        };

        ui.painter().rect(
            rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::epaint::StrokeKind::Inside
        );

        let icon_size = 12.0 * scale;
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - padding.x - icon_size / 2.0, rect.center().y),
            egui::vec2(icon_size, icon_size)
        );
        gui::Gui::down_triangle_icon(ui.painter(), icon_rect, visuals);

        let galley = ui.painter().layout_no_wrap(
            selected_text.to_owned(),
            egui::TextStyle::Button.resolve(ui.style()),
            visuals.text_color()
        );

        let text_pos = egui::pos2(
            rect.left() + padding.x,
            rect.center().y - galley.size().y / 2.0
        );
        ui.painter().galley(text_pos, galley, visuals.text_color());
    }

    let mut changed = false;
    let mut search = String::new();
    if !search_term.is_null() && search_term_len > 0 {
        let bytes = std::slice::from_raw_parts(search_term as *const u8, search_term_len);
        if let Some(end) = bytes.iter().position(|b| *b == 0) {
            search = String::from_utf8_lossy(&bytes[..end]).into_owned();
        }
    }

    egui::Popup::menu(&button_res)
    .id(popup_id)
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .show(|ui| {
        ui.set_width(fixed_width);
        ui.set_max_width(fixed_width);

        ui.horizontal(|ui| {
            let res = ui.add_sized(
                [ui.available_width() - 30.0 * scale, row_height],
                egui::TextEdit::singleline(&mut search).hint_text("Search...")
            );
            #[cfg(target_os = "android")]
            gui::handle_android_keyboard(&res, &mut search);

            if ui.button("X").clicked() {
                search.clear();
                res.surrender_focus();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
        .max_height(250.0 * scale)
        .hscroll(false)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                for i in 0..item_count {
                    let item_ptr = *items.add(i);
                    if item_ptr.is_null() { continue; }
                    let label = CStr::from_ptr(item_ptr).to_str().unwrap_or("");
                    if !search.is_empty() && !label.to_lowercase().contains(&search.to_lowercase()) {
                        continue;
                    }

                    let is_selected = current_idx == i;
                    if ui.add(egui::Button::selectable(is_selected, label)).clicked() {
                        *selected_index = i as i32;
                        changed = true;
                        egui::Popup::close_id(ui.ctx(), popup_id);
                        search.clear();
                    }
                }
            });
        });
    });

    if !search_term.is_null() && search_term_len > 0 {
        let bytes = std::slice::from_raw_parts_mut(search_term as *mut u8, search_term_len);
        let search_bytes = search.as_bytes();
        let len = search_bytes.len().min(search_term_len.saturating_sub(1));
        bytes[..len].copy_from_slice(&search_bytes[..len]);
        if len < search_term_len {
            bytes[len] = 0;
        }
        if len + 1 < search_term_len {
            bytes[len + 1..].fill(0);
        }
    }

    changed
}

unsafe extern "C" fn gui_register_menu_item_icon(
    label: *const c_char,
    icon_uri: *const c_char,
    icon_ptr: *const u8,
    icon_len: usize
) -> bool {
    if label.is_null() || icon_ptr.is_null() || icon_len == 0 {
        return false;
    }
    let Ok(label) = CStr::from_ptr(label).to_str() else {
        return false;
    };
    let uri = if icon_uri.is_null() {
        format!("bytes://plugin-icon/{}.png", label)
    }
    else {
        let Ok(uri) = CStr::from_ptr(icon_uri).to_str() else {
            return false;
        };
        uri.to_owned()
    };
    let bytes = std::slice::from_raw_parts(icon_ptr, icon_len);
    gui::register_plugin_menu_icon(label.to_owned(), uri, bytes.to_vec())
}

unsafe extern "C" fn gui_register_menu_section_with_icon(
    title: *const c_char,
    icon_uri: *const c_char,
    icon_ptr: *const u8,
    icon_len: usize,
    callback: Option<GuiMenuSectionCallback>,
    userdata: *mut c_void
) -> bool {
    let Some(callback) = callback else {
        return false;
    };
    if title.is_null() || icon_ptr.is_null() || icon_len == 0 {
        return false;
    }
    let Ok(title) = CStr::from_ptr(title).to_str() else {
        return false;
    };
    let uri = if icon_uri.is_null() {
        format!("bytes://plugin-section/{}.png", title)
    }
    else {
        let Ok(uri) = CStr::from_ptr(icon_uri).to_str() else {
            return false;
        };
        uri.to_owned()
    };
    let bytes = std::slice::from_raw_parts(icon_ptr, icon_len);
    gui::register_plugin_menu_section_with_icon(
        title.to_owned(),
        uri,
        bytes.to_vec(),
        callback,
        userdata
    )
}

unsafe extern "C" fn gui_new_window_id() -> i32 {
    NEXT_PLUGIN_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

unsafe extern "C" fn gui_show_window(
    id: i32,
    title: *const c_char,
    contents_callback: Option<GuiWindowCallback>,
    bottom_callback: Option<GuiWindowCallback>,
    userdata: *mut c_void
) -> bool {
    if title.is_null() { return false; }
    let Ok(title_str) = CStr::from_ptr(title).to_str() else { return false; };

    gui::show_plugin_window(
        id,
        title_str.to_owned(),
        contents_callback,
        bottom_callback,
        userdata as usize,
    );
    true
}

unsafe extern "C" fn gui_close_window(id: i32) {
    gui::close_plugin_window(id);
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_load(dex_ptr: *const u8, dex_len: usize, class_name: *const c_char) -> u64 {
    crate::android::dex_bridge::dex_load(dex_ptr, dex_len, class_name)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_load(_dex_ptr: *const u8, _dex_len: usize, _class_name: *const c_char) -> u64 {
    0
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_unload(handle: u64) -> bool {
    crate::android::dex_bridge::dex_unload(handle)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_unload(_handle: u64) -> bool {
    false
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_call_static_noargs(handle: u64, method: *const c_char, sig: *const c_char) -> bool {
    let method = CStr::from_ptr(method);
    let sig = CStr::from_ptr(sig);
    crate::android::dex_bridge::call_static_noargs(handle, method, sig)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_call_static_noargs(_handle: u64, _method: *const c_char, _sig: *const c_char) -> bool {
    false
}

#[cfg(target_os = "android")]
unsafe extern "C" fn android_dex_call_static_string(handle: u64, method: *const c_char, sig: *const c_char, arg: *const c_char) -> bool {
    let method = CStr::from_ptr(method);
    let sig = CStr::from_ptr(sig);
    let arg = CStr::from_ptr(arg);
    crate::android::dex_bridge::call_static_string(handle, method, sig, arg)
}

#[cfg(not(target_os = "android"))]
unsafe extern "C" fn android_dex_call_static_string(_handle: u64, _method: *const c_char, _sig: *const c_char, _arg: *const c_char) -> bool {
    false
}

unsafe extern "C" fn gui_get_menu_width() -> f32 {
    gui::get_menu_width()
}

unsafe extern "C" fn gui_set_menu_width(width: f32) {
    gui::set_menu_width(width);
}

unsafe extern "C" fn hachimi_get_base_dir() -> *const c_char {
    let s = DATA_DIR_CSTR.get_or_init(|| {
        let path = &crate::core::Hachimi::instance().game.data_dir;
        CString::new(path.to_string_lossy().into_owned()).unwrap()
    });
    s.as_ptr()
}

unsafe extern "C" fn hachimi_get_data_path() -> *const c_char {
    let s = DATA_PATH_CSTR.get_or_init(|| {
        let path = crate::core::utils::get_data_path();
        CString::new(path).unwrap()
    });
    s.as_ptr()
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
    pub il2cpp_resolve_icall: unsafe extern "C" fn(name: *const c_char) -> Il2CppMethodPointer,
    pub il2cpp_class_get_methods: unsafe extern "C" fn(klass: *mut Il2CppClass, iter: *mut *mut c_void) -> *const MethodInfo,
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
    pub il2cpp_object_new: unsafe extern "C" fn(klass: *const Il2CppClass) -> *mut Il2CppObject,
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
    pub gui_register_menu_item_icon: unsafe extern "C" fn(
        label: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize
    ) -> bool,
    pub gui_register_menu_section_with_icon: unsafe extern "C" fn(
        title: *const c_char,
        icon_uri: *const c_char,
        icon_ptr: *const u8,
        icon_len: usize,
        callback: Option<GuiMenuSectionCallback>,
        userdata: *mut c_void
    ) -> bool,
    // Window management (version >= 3)
    pub gui_new_window_id: unsafe extern "C" fn() -> i32,
    pub gui_show_window: unsafe extern "C" fn(
        id: i32,
        title: *const c_char,
        contents_callback: Option<GuiWindowCallback>,
        bottom_callback: Option<GuiWindowCallback>,
        userdata: *mut c_void
    ) -> bool,
    pub gui_close_window: unsafe extern "C" fn(id: i32),

    pub android_dex_load: unsafe extern "C" fn(dex_ptr: *const u8, dex_len: usize, class_name: *const c_char) -> u64,
    pub android_dex_unload: unsafe extern "C" fn(handle: u64) -> bool,
    pub android_dex_call_static_noargs: unsafe extern "C" fn(handle: u64, method: *const c_char, sig: *const c_char) -> bool,
    pub android_dex_call_static_string: unsafe extern "C" fn(handle: u64, method: *const c_char, sig: *const c_char, arg: *const c_char) -> bool,

    pub il2cpp_runtime_object_init: unsafe extern "C" fn(object: *mut Il2CppObject),
    pub il2cpp_string_new: unsafe extern "C" fn(text: *const c_char) -> *mut Il2CppString,
    pub il2cpp_string_chars: unsafe extern "C" fn(s: *mut Il2CppString) -> *mut u16,
    pub il2cpp_string_length: unsafe extern "C" fn(s: *mut Il2CppString) -> i32,
    pub gui_ui_combo_menu: unsafe extern "C" fn(
        ui: *mut c_void,
        id: *const c_char,
        selected_index: *mut i32,
        items: *const *const c_char,
        item_count: usize,
        search_term: *mut c_char,
        search_term_len: usize,
    ) -> bool,
    pub hachimi_register_on_game_initialized: unsafe extern "C" fn(
        callback: Option<GameInitializedCallback>,
        userdata: *mut c_void,
    ) -> bool,
    pub hachimi_register_present_callback: unsafe extern "C" fn(
        callback: Option<PresentCallback>,
        userdata: *mut c_void,
    ) -> bool,
    pub gui_get_menu_width: unsafe extern "C" fn() -> f32,
    pub gui_set_menu_width: unsafe extern "C" fn(width: f32),
    pub hachimi_get_base_dir: unsafe extern "C" fn() -> *const c_char,
    pub hachimi_get_data_path: unsafe extern "C" fn() -> *const c_char,
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
        il2cpp_resolve_icall,
        il2cpp_class_get_methods,
        il2cpp_get_field_from_name,
        il2cpp_get_field_value,
        il2cpp_set_field_value,
        il2cpp_get_static_field_value,
        il2cpp_set_static_field_value,
        il2cpp_object_new,
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
        gui_register_menu_item_icon,
        gui_register_menu_section_with_icon,
        gui_new_window_id,
        gui_show_window,
        gui_close_window,
        android_dex_load,
        android_dex_unload,
        android_dex_call_static_noargs,
        android_dex_call_static_string,
        il2cpp_runtime_object_init,
        il2cpp_string_new,
        il2cpp_string_chars,
        il2cpp_string_length,
        gui_ui_combo_menu,
        hachimi_register_on_game_initialized,
        hachimi_register_present_callback,
        gui_get_menu_width,
        gui_set_menu_width,
        hachimi_get_base_dir,
        hachimi_get_data_path,
    };

    pub fn instantiate() -> Self {
        Self::VALUE.clone()
    }
}

pub extern "C" fn hachimi_get_api(name: *const c_char) -> *mut c_void {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(func_name) = (unsafe { CStr::from_ptr(name).to_str() }) else {
        return std::ptr::null_mut();
    };

    match func_name {
        "hachimi_instance" => hachimi_instance as *mut c_void,
        "hachimi_get_interceptor" => hachimi_get_interceptor as *mut c_void,
        "interceptor_hook" => interceptor_hook as *mut c_void,
        "interceptor_hook_vtable" => interceptor_hook_vtable as *mut c_void,
        "interceptor_get_trampoline_addr" => interceptor_get_trampoline_addr as *mut c_void,
        "interceptor_unhook" => interceptor_unhook as *mut c_void,
        "il2cpp_resolve_symbol" => il2cpp_resolve_symbol as *mut c_void,
        "il2cpp_get_assembly_image" => il2cpp_get_assembly_image as *mut c_void,
        "il2cpp_get_class" => il2cpp_get_class as *mut c_void,
        "il2cpp_get_method" => il2cpp_get_method as *mut c_void,
        "il2cpp_get_method_overload" => il2cpp_get_method_overload as *mut c_void,
        "il2cpp_get_method_addr" => il2cpp_get_method_addr as *mut c_void,
        "il2cpp_get_method_overload_addr" => il2cpp_get_method_overload_addr as *mut c_void,
        "il2cpp_get_method_cached" => il2cpp_get_method_cached as *mut c_void,
        "il2cpp_get_method_addr_cached" => il2cpp_get_method_addr_cached as *mut c_void,
        "il2cpp_find_nested_class" => il2cpp_find_nested_class as *mut c_void,
        "il2cpp_resolve_icall" => il2cpp_resolve_icall as *mut c_void,
        "il2cpp_class_get_methods" => il2cpp_class_get_methods as *mut c_void,
        "il2cpp_get_field_from_name" => il2cpp_get_field_from_name as *mut c_void,
        "il2cpp_get_field_value" => il2cpp_get_field_value as *mut c_void,
        "il2cpp_set_field_value" => il2cpp_set_field_value as *mut c_void,
        "il2cpp_get_static_field_value" => il2cpp_get_static_field_value as *mut c_void,
        "il2cpp_set_static_field_value" => il2cpp_set_static_field_value as *mut c_void,
        "il2cpp_object_new" => il2cpp_object_new as *mut c_void,
        "il2cpp_unbox" => il2cpp_unbox as *mut c_void,
        "il2cpp_get_main_thread" => il2cpp_get_main_thread as *mut c_void,
        "il2cpp_get_attached_threads" => il2cpp_get_attached_threads as *mut c_void,
        "il2cpp_schedule_on_thread" => il2cpp_schedule_on_thread as *mut c_void,
        "il2cpp_create_array" => il2cpp_create_array as *mut c_void,
        "il2cpp_get_singleton_like_instance" => il2cpp_get_singleton_like_instance as *mut c_void,
        "log" => log as *mut c_void,
        "gui_register_menu_item" => gui_register_menu_item as *mut c_void,
        "gui_register_menu_section" => gui_register_menu_section as *mut c_void,
        "gui_show_notification" => gui_show_notification as *mut c_void,
        "gui_ui_heading" => gui_ui_heading as *mut c_void,
        "gui_ui_label" => gui_ui_label as *mut c_void,
        "gui_ui_small" => gui_ui_small as *mut c_void,
        "gui_ui_separator" => gui_ui_separator as *mut c_void,
        "gui_ui_button" => gui_ui_button as *mut c_void,
        "gui_ui_small_button" => gui_ui_small_button as *mut c_void,
        "gui_ui_checkbox" => gui_ui_checkbox as *mut c_void,
        "gui_ui_text_edit_singleline" => gui_ui_text_edit_singleline as *mut c_void,
        "gui_ui_horizontal" => gui_ui_horizontal as *mut c_void,
        "gui_ui_grid" => gui_ui_grid as *mut c_void,
        "gui_ui_end_row" => gui_ui_end_row as *mut c_void,
        "gui_ui_colored_label" => gui_ui_colored_label as *mut c_void,
        "gui_register_menu_item_icon" => gui_register_menu_item_icon as *mut c_void,
        "gui_register_menu_section_with_icon" => gui_register_menu_section_with_icon as *mut c_void,
        "gui_new_window_id" => gui_new_window_id as *mut c_void,
        "gui_show_window" => gui_show_window as *mut c_void,
        "gui_close_window" => gui_close_window as *mut c_void,
        "android_dex_load" => android_dex_load as *mut c_void,
        "android_dex_unload" => android_dex_unload as *mut c_void,
        "android_dex_call_static_noargs" => android_dex_call_static_noargs as *mut c_void,
        "android_dex_call_static_string" => android_dex_call_static_string as *mut c_void,
        "il2cpp_runtime_object_init" => il2cpp_runtime_object_init as *mut c_void,
        "il2cpp_string_new" => il2cpp_string_new as *mut c_void,
        "il2cpp_string_chars" => il2cpp_string_chars as *mut c_void,
        "il2cpp_string_length" => il2cpp_string_length as *mut c_void,
        "gui_ui_combo_menu" => gui_ui_combo_menu as *mut c_void,
        "hachimi_register_on_game_initialized" => hachimi_register_on_game_initialized as *mut c_void,
        "hachimi_register_present_callback" => hachimi_register_present_callback as *mut c_void,
        "gui_get_menu_width" => gui_get_menu_width as *mut c_void,
        "gui_set_menu_width" => gui_set_menu_width as *mut c_void,
        "hachimi_get_base_dir" => hachimi_get_base_dir as *mut c_void,
        "hachimi_get_data_path" => hachimi_get_data_path as *mut c_void,
        _ => std::ptr::null_mut(),
    }
}

pub enum PluginInit {
    V2(HachimiInitFn),
    V3(HachimiInitV3Fn),
}

pub struct Plugin {
    pub name: String,
    pub init_fn: PluginInit
}

impl Plugin {
    pub fn init(&self) -> InitResult {
        match &self.init_fn {
            PluginInit::V2(init) => {
                let vtable = PLUGIN_VTABLE.get_or_init(Vtable::instantiate);
                init(vtable as *const Vtable, 2)
            },
            PluginInit::V3(init) => {
                init(hachimi_get_api, VERSION)
            }
        }
    }
}
