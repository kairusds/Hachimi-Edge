use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr::null_mut;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::il2cpp::api::*;
use crate::il2cpp::ext::Il2CppObjectExt;
use crate::il2cpp::symbols;
use crate::il2cpp::types::*;

struct CaptionState {
    handle: Option<symbols::GCHandle>,
    inited: bool,
    fade_id: u64,
    fade_start_time: Option<std::time::Instant>,
    display_time: f32,
    fade_out_time: f32,
}

impl CaptionState {
    fn notification(&self) -> *mut Il2CppObject {
        self.handle.as_ref().map_or(null_mut(), |h| h.target())
    }

    fn clear(&mut self) {
        self.handle = None;
        self.inited = false;
        self.fade_id = self.fade_id.wrapping_add(1);
    }

    fn set_notification(&mut self, obj: *mut Il2CppObject) {
        self.handle = None;
        self.fade_id = self.fade_id.wrapping_add(1);
        if !obj.is_null() {
            self.handle = Some(symbols::GCHandle::new(obj, false));
        }
    }
}

static STATE: Lazy<Mutex<CaptionState>> = Lazy::new(|| {
    Mutex::new(CaptionState {
        handle: None,
        inited: false,
        fade_id: 0,
        fade_start_time: None,
        display_time: 0.0,
        fade_out_time: 0.5,
    })
});

fn is_native_alive(obj: *mut Il2CppObject) -> bool {
    if obj.is_null() { return false; }
    crate::il2cpp::hook::UnityEngine_CoreModule::Object::IsNativeObjectAlive(obj)
}

fn invoke(method: *const MethodInfo, obj: *mut c_void, params: *mut *mut c_void) -> *mut Il2CppObject {
    if method.is_null() { return null_mut(); }
    let mut exc: *mut Il2CppException = null_mut();
    let r = il2cpp_runtime_invoke(method, obj, params, &mut exc);
    if !exc.is_null() { return null_mut(); }
    r
}

fn invoke_method(klass: *mut Il2CppClass, name: &std::ffi::CStr, argc: i32, obj: *mut c_void, params: *mut *mut c_void) -> *mut Il2CppObject {
    let m = il2cpp_class_get_method_from_name(klass, name.as_ptr(), argc);
    invoke(m, obj, params)
}

fn get_class(asm: &std::ffi::CStr, ns: &std::ffi::CStr, name: &std::ffi::CStr) -> *mut Il2CppClass {
    let image = match symbols::get_assembly_image(asm) {
        Ok(img) => img,
        Err(_) => return null_mut(),
    };
    match symbols::get_class(image, ns, name) {
        Ok(c) => c,
        Err(_) => null_mut(),
    }
}

fn get_runtime_type(asm: &std::ffi::CStr, ns: &std::ffi::CStr, name: &std::ffi::CStr) -> *mut Il2CppObject {
    let k = get_class(asm, ns, name);
    if k.is_null() { return null_mut(); }
    let t = il2cpp_class_get_type(k);
    if t.is_null() { return null_mut(); }
    il2cpp_type_get_object(t) as *mut Il2CppObject
}

fn parse_enum(enum_type: *mut Il2CppObject, value: &str) -> *mut Il2CppObject {
    if enum_type.is_null() || value.is_empty() { return null_mut(); }
    let enum_class = get_class(c"mscorlib.dll", c"System", c"Enum");
    if enum_class.is_null() { return null_mut(); }
    let c_val = match CString::new(value) { Ok(v) => v, Err(_) => return null_mut() };
    let val_str = il2cpp_string_new(c_val.as_ptr());
    let mut params: [*mut c_void; 2] = [enum_type as _, val_str as _];
    invoke_method(enum_class, c"Parse", 2, null_mut(), params.as_mut_ptr())
}

fn get_enum_int(e: *mut Il2CppObject) -> i32 {
    if e.is_null() { return 0; }
    let enum_class = get_class(c"mscorlib.dll", c"System", c"Enum");
    if enum_class.is_null() { return 0; }
    let mut params: [*mut c_void; 1] = [e as _];
    let r = invoke_method(enum_class, c"ToUInt64", 1, null_mut(), params.as_mut_ptr());
    if r.is_null() { return 0; }
    unsafe { *(il2cpp_object_unbox(r) as *mut u64) as i32 }
}

unsafe fn method_pointer(m: *const MethodInfo) -> usize {
    if m.is_null() { return 0; }
    *(m as *const usize)
}

#[cfg(target_os = "windows")]
fn seh_guard<F: FnMut()>(mut f: F) {
    if microseh::try_seh(|| f()).is_err() {
        warn!("[captions] SEH exception caught, resetting state");
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

#[cfg(not(target_os = "windows"))]
fn seh_guard<F: FnMut()>(mut f: F) { f(); }

fn init_impl() {
    let mut st = STATE.lock().unwrap();
    let notif = st.notification();
    if st.inited && !notif.is_null() && is_native_alive(notif) { return; }
    st.clear();

    let ui_mgr_class = get_class(c"umamusume.dll", c"Gallop", c"UIManager");
    if ui_mgr_class.is_null() { return; }
    let ui_mgr = invoke_method(ui_mgr_class, c"get_Instance", 0, null_mut(), null_mut());
    if ui_mgr.is_null() { return; }

    let mut canvas: *mut Il2CppObject = null_mut();
    for fname in [c"_noticeCanvas", c"_systemCanvas", c"_mainCanvas"] {
        let f = il2cpp_class_get_field_from_name(ui_mgr_class, fname.as_ptr());
        if !f.is_null() {
            il2cpp_field_get_value(ui_mgr, f, &mut canvas as *mut _ as _);
            if !canvas.is_null() { break; }
        }
    }
    if canvas.is_null() { return; }

    let transform = invoke_method(unsafe { (*canvas).klass() }, c"get_transform", 0, canvas as _, null_mut());
    if transform.is_null() { return; }

    let res_class = get_class(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"Resources");
    if res_class.is_null() { return; }
    let path = il2cpp_string_new(c"UI/Parts/Notification".as_ptr());
    let go_type = get_runtime_type(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"GameObject");
    if go_type.is_null() { return; }
    let mut load_params: [*mut c_void; 2] = [path as _, go_type as _];
    let prefab = invoke_method(res_class, c"Load", 2, null_mut(), load_params.as_mut_ptr());
    if prefab.is_null() { return; }

    type CloneFn = extern "C" fn(*mut Il2CppObject, *mut Il2CppObject, bool) -> *mut Il2CppObject;
    let clone_fn: CloneFn = unsafe {
        let ptr = il2cpp_resolve_icall(c"UnityEngine.Object::Internal_CloneSingleWithParent()".as_ptr());
        if ptr == 0 { return; }
        std::mem::transmute(ptr)
    };
    let inst = clone_fn(prefab, transform, false);
    if inst.is_null() { return; }

    let notif_type = get_runtime_type(c"umamusume.dll", c"Gallop", c"Notification");
    if notif_type.is_null() { return; }
    let mut inc_inactive: bool = true;
    let mut gc_params: [*mut c_void; 2] = [notif_type as _, &mut inc_inactive as *mut bool as _];
    let go_class = get_class(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"GameObject");
    if go_class.is_null() { return; }
    let new_notif = invoke_method(go_class, c"GetComponentInChildren", 2, inst as _, gc_params.as_mut_ptr());
    if new_notif.is_null() { return; }
    st.set_notification(new_notif);

    let go = invoke_method(unsafe { (*new_notif).klass() }, c"get_gameObject", 0, new_notif as _, null_mut());
    if !go.is_null() {
        let mut active: bool = false;
        let mut p: [*mut c_void; 1] = [&mut active as *mut bool as _];
        invoke_method(unsafe { (*go).klass() }, c"SetActive", 1, go as _, p.as_mut_ptr());
        st.inited = true;
    }
    if !st.inited { st.clear(); }
}

fn show_impl(text: &str, line_char_count: i32) {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) {
        drop(st);
        STATE.lock().unwrap().clear();
        return;
    }
    let nk = unsafe { (*notif).klass() };
    drop(st);

    let label_f = il2cpp_class_get_field_from_name(nk, c"_Label".as_ptr());
    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if label_f.is_null() || cg_f.is_null() { return; }

    let mut label: *mut Il2CppObject = null_mut();
    let mut cg: *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, label_f, &mut label as *mut _ as _);
    il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
    if label.is_null() || cg.is_null() { return; }

    let c_text = match CString::new(text) { Ok(v) => v, Err(_) => return };
    let mut il2_text = il2cpp_string_new(c_text.as_ptr()) as *mut Il2CppObject;

    let gu_class = get_class(c"umamusume.dll", c"Gallop", c"GallopUtil");
    if !gu_class.is_null() && line_char_count > 0 {
        let mut lcc = line_char_count;
        let mut p: [*mut c_void; 2] = [il2_text as _, &mut lcc as *mut i32 as _];
        let wrapped = invoke_method(gu_class, c"LineHeadWrap", 2, null_mut(), p.as_mut_ptr());
        if !wrapped.is_null() { il2_text = wrapped; }
    }

    let set_text_m = il2cpp_class_get_method_from_name(unsafe { (*label).klass() }, c"set_text".as_ptr(), 1);
    let set_text_fp = unsafe { method_pointer(set_text_m) };
    if set_text_fp != 0 {
        let set_text: extern "C" fn(*mut Il2CppObject, *mut Il2CppObject) = unsafe { std::mem::transmute(set_text_fp) };
        set_text(label, il2_text);
    }

    let set_alpha_m = il2cpp_class_get_method_from_name(unsafe { (*cg).klass() }, c"set_alpha".as_ptr(), 1);
    let set_alpha_fp = unsafe { method_pointer(set_alpha_m) };
    if set_alpha_fp != 0 {
        let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = unsafe { std::mem::transmute(set_alpha_fp) };
        set_alpha(cg, 1.0);
    }

    let go_m = il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0);
    let go_fp = unsafe { method_pointer(go_m) };
    if go_fp != 0 {
        let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(go_fp) };
        let go = get_go(notif);
        if !go.is_null() {
            let sa_m = il2cpp_class_get_method_from_name(unsafe { (*go).klass() }, c"SetActive".as_ptr(), 1);
            let sa_fp = unsafe { method_pointer(sa_m) };
            if sa_fp != 0 {
                let set_active: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(sa_fp) };
                set_active(go, true);
            }
        }
    }

    let mut display_time: f32 = 0.0;
    let mut fade_out: f32 = 0.5;
    let dt_f = il2cpp_class_get_field_from_name(nk, c"_displayTime".as_ptr());
    let fo_f = il2cpp_class_get_field_from_name(nk, c"_fadeOutTime".as_ptr());
    if !dt_f.is_null() { il2cpp_field_get_value(notif, dt_f, &mut display_time as *mut f32 as _); }
    if !fo_f.is_null() { il2cpp_field_get_value(notif, fo_f, &mut fade_out as *mut f32 as _); }

    {
        let mut st = STATE.lock().unwrap();
        st.fade_id = st.fade_id.wrapping_add(1);
        st.fade_start_time = Some(std::time::Instant::now());
        st.display_time = display_time;
        st.fade_out_time = fade_out;
    }
    crate::il2cpp::symbols::Thread::main_thread().schedule(fade_tick_global);
}

fn fade_tick_global() {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }

    let start_time = match st.fade_start_time {
        Some(t) => t,
        None => return,
    };

    let display_time = st.display_time;
    let fade_out = st.fade_out_time;
    let nk = unsafe { (*notif).klass() };
    drop(st);

    let elapsed = start_time.elapsed().as_secs_f32();
    let mut alpha = 1.0;
    let mut active = true;
    let mut done = false;

    if elapsed >= display_time + fade_out {
        alpha = 0.0;
        active = false;
        done = true;
    } else if elapsed >= display_time {
        let progress = (elapsed - display_time) / fade_out.max(0.001);
        alpha = 1.0 - progress.clamp(0.0, 1.0);
    }

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if !cg_f.is_null() {
        let mut cg: *mut Il2CppObject = null_mut();
        il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
        if !cg.is_null() {
            let set_alpha_m = il2cpp_class_get_method_from_name(unsafe { (*cg).klass() }, c"set_alpha".as_ptr(), 1);
            let set_alpha_fp = unsafe { method_pointer(set_alpha_m) };
            if set_alpha_fp != 0 {
                let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = unsafe { std::mem::transmute(set_alpha_fp) };
                set_alpha(cg, alpha);
            }
        }
    }

    if !active {
        let go_m = il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0);
        let go_fp = unsafe { method_pointer(go_m) };
        if go_fp != 0 {
            let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(go_fp) };
            let go = get_go(notif);
            if !go.is_null() {
                let sa_m = il2cpp_class_get_method_from_name(unsafe { (*go).klass() }, c"SetActive".as_ptr(), 1);
                let sa_fp = unsafe { method_pointer(sa_m) };
                if sa_fp != 0 {
                    let set_active: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(sa_fp) };
                    set_active(go, false);
                }
            }
        }
    }

    if !done {
        crate::il2cpp::symbols::Thread::main_thread().schedule(fade_tick_global);
    }
}

fn set_display_time_impl(time: f32) {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }
    let nk = unsafe { (*notif).klass() };
    drop(st);

    let f = il2cpp_class_get_field_from_name(nk, c"_displayTime".as_ptr());
    if !f.is_null() { il2cpp_field_set_value(notif, f, &time as *const f32 as _); }
}

fn set_format_impl(
    font_size: i32,
    font_color: &str,
    outline_size: &str,
    outline_color: &str,
    pos_x: f32,
    pos_y: f32,
    bg_alpha: f32,
) {
    let st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }
    let nk = unsafe { (*notif).klass() };
    drop(st);

    let label_f = il2cpp_class_get_field_from_name(nk, c"_Label".as_ptr());
    if label_f.is_null() { return; }
    let mut label: *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, label_f, &mut label as *mut _ as _);
    if label.is_null() { return; }
    let lk = unsafe { (*label).klass() };

    let mut fs = font_size;
    let mut sp: [*mut c_void; 1] = [&mut fs as *mut i32 as _];
    invoke_method(lk, c"set_fontSize", 1, label as _, sp.as_mut_ptr());
    invoke_method(lk, c"set_resizeTextMaxSize", 1, label as _, sp.as_mut_ptr());

    if !font_color.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"FontColorType"), font_color);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_FontColor", 1, label as _, p.as_mut_ptr());
        }
    }

    if !outline_size.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineSizeType"), outline_size);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_OutlineSize", 1, label as _, p.as_mut_ptr());
        }
        invoke_method(lk, c"UpdateOutline", 0, label as _, null_mut());
    }

    if !outline_color.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineColorType"), outline_color);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_OutlineColor", 1, label as _, p.as_mut_ptr());
        }
        invoke_method(lk, c"RebuildOutline", 0, label as _, null_mut());
    }

    let go = invoke_method(nk, c"get_gameObject", 0, notif as _, null_mut());
    if !go.is_null() {
        let img_type = get_runtime_type(c"umamusume.dll", c"Gallop", c"ImageCommon");
        if !img_type.is_null() {
            let mut inc: bool = true;
            let mut bgp: [*mut c_void; 2] = [img_type as _, &mut inc as *mut bool as _];
            let bg = invoke_method(unsafe { (*go).klass() }, c"GetComponentInChildren", 2, go as _, bgp.as_mut_ptr());
            if !bg.is_null() {
                let mut ba = bg_alpha;
                let mut p: [*mut c_void; 1] = [&mut ba as *mut f32 as _];
                invoke_method(unsafe { (*bg).klass() }, c"SetAlpha", 1, bg as _, p.as_mut_ptr());
            }
        }
    }

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if cg_f.is_null() { return; }
    let mut cg: *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
    if cg.is_null() || !is_native_alive(cg) { return; }

    let cg_tr = invoke_method(unsafe { (*cg).klass() }, c"get_transform", 0, cg as _, null_mut());
    if cg_tr.is_null() { return; }
    let tr_k = unsafe { (*cg_tr).klass() };

    let get_pos_m = il2cpp_class_get_method_from_name(tr_k, c"get_position".as_ptr(), 0);
    let set_pos_m = il2cpp_class_get_method_from_name(tr_k, c"set_position".as_ptr(), 1);
    if !get_pos_m.is_null() && !set_pos_m.is_null() {
        let pos_obj = invoke(get_pos_m, cg_tr as _, null_mut());
        if !pos_obj.is_null() {
            #[repr(C)]
            #[derive(Clone, Copy)]
            struct Vec3 { x: f32, y: f32, z: f32 }

            let pos = unsafe { &*(il2cpp_object_unbox(pos_obj) as *const Vec3) };
            let mut new_pos = Vec3 { x: pos_x, y: pos_y, z: pos.z };
            let mut p: [*mut c_void; 1] = [&mut new_pos as *mut Vec3 as _];
            invoke(set_pos_m, cg_tr as _, p.as_mut_ptr());
        }
    }
}

fn cleanup_impl() {
    let mut st = STATE.lock().unwrap();
    let notif = st.notification();
    if notif.is_null() || !is_native_alive(notif) { return; }
    let nk = unsafe { (*notif).klass() };

    st.fade_id = st.fade_id.wrapping_add(1);
    drop(st);

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if !cg_f.is_null() {
        let mut cg: *mut Il2CppObject = null_mut();
        il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
        if !cg.is_null() {
            let set_alpha_m = il2cpp_class_get_method_from_name(unsafe { (*cg).klass() }, c"set_alpha".as_ptr(), 1);
            let set_alpha_fp = unsafe { method_pointer(set_alpha_m) };
            if set_alpha_fp != 0 {
                let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = unsafe { std::mem::transmute(set_alpha_fp) };
                set_alpha(cg, 0.0);
            }
        }
    }

    let go_m = il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0);
    let go_fp = unsafe { method_pointer(go_m) };
    if go_fp != 0 {
        let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = unsafe { std::mem::transmute(go_fp) };
        let go = get_go(notif);
        if !go.is_null() {
            let sa_m = il2cpp_class_get_method_from_name(unsafe { (*go).klass() }, c"SetActive".as_ptr(), 1);
            let sa_fp = unsafe { method_pointer(sa_m) };
            if sa_fp != 0 {
                let set_active: extern "C" fn(*mut Il2CppObject, bool) = unsafe { std::mem::transmute(sa_fp) };
                set_active(go, false);
            }
        }
    }
}

pub struct Captions;

impl Captions {
    pub fn init() {
        seh_guard(init_impl);
    }

    pub fn show(text: &str, line_char_count: i32) {
        let text = text.to_owned();
        seh_guard(move || show_impl(&text, line_char_count));
    }

    pub fn set_display_time(time: f32) {
        seh_guard(move || set_display_time_impl(time));
    }

    pub fn set_format(
        font_size: i32,
        font_color: &str,
        outline_size: &str,
        outline_color: &str,
        pos_x: f32,
        pos_y: f32,
        bg_alpha: f32,
    ) {
        let fc = font_color.to_owned();
        let os = outline_size.to_owned();
        let oc = outline_color.to_owned();
        seh_guard(move || set_format_impl(font_size, &fc, &os, &oc, pos_x, pos_y, bg_alpha));
    }

    pub fn cleanup() {
        seh_guard(cleanup_impl);
    }

    pub fn reset() {
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}