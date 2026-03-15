use crate::core::Hachimi;

// dyld internals — not exposed by the libc crate for iOS targets.
// These are part of Apple's libSystem and are safe to link against.
#[repr(C)]
struct MachHeader {
    _opaque: [u8; 0],
}

extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_header(image_index: u32) -> *const MachHeader;
    fn _dyld_get_image_name(image_index: u32) -> *const libc::c_char;
    fn _dyld_register_func_for_add_image(
        func: Option<unsafe extern "C" fn(*const MachHeader, libc::intptr_t)>,
    );
}

/// Called from the dyld image-add callback each time a new image
/// is loaded into the process address space.
unsafe extern "C" fn on_image_added(mh: *const MachHeader, _slide: libc::intptr_t) {
    let count = _dyld_image_count();
    for i in 0..count {
        let img_mh = _dyld_get_image_header(i);
        if img_mh == mh {
            let raw = _dyld_get_image_name(i);
            if raw.is_null() { break; }
            let name = std::ffi::CStr::from_ptr(raw)
                .to_str()
                .unwrap_or("");
            if crate::ios::hachimi_impl::is_il2cpp_lib(name) {
                info!("iOS: GameAssembly loaded at {:p}, slide={}", mh, _slide);
                Hachimi::instance().on_dlopen(name, mh as usize);
            }
            break;
        }
    }
}

fn init_internal() {
    unsafe {
        _dyld_register_func_for_add_image(Some(on_image_added));
    }
    info!("iOS: dyld image callback registered");
}

pub fn init() {
    init_internal();
}
