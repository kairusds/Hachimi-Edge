use crate::core::{Error, Hachimi};

/// Called from the dyld image-add callback each time a new
/// image is loaded into the process address space.
///
/// When GameAssembly is detected, notify Hachimi core and
/// start IL2CPP hooking exactly like Android does after dlopen.
unsafe extern "C" fn on_image_added(
    mh: *const libc::mach_header,
    _slide: libc::intptr_t,
) {
    // Walk the currently-loaded images to find out which name
    // corresponds to the just-added mach_header pointer.
    let count = libc::_dyld_image_count();
    for i in 0..count {
        let img_mh = libc::_dyld_get_image_header(i);
        if img_mh == mh as *const _ {
            if let Some(raw_name) = libc::_dyld_get_image_name(i).as_ref() {
                let name = std::ffi::CStr::from_ptr(raw_name)
                    .to_str()
                    .unwrap_or("");

                if crate::ios::hachimi_impl::is_il2cpp_lib(name) {
                    info!("iOS: GameAssembly loaded at {:p}, slide={}", mh, _slide);
                    Hachimi::instance().on_dlopen(name, mh as usize);
                }
            }
            break;
        }
    }
}

fn init_internal() -> Result<(), Error> {
    unsafe {
        libc::_dyld_register_func_for_add_image(Some(on_image_added));
    }
    info!("iOS: dyld image callback registered");
    Ok(())
}

pub fn init() {
    init_internal().unwrap_or_else(|e| {
        error!("iOS hook init failed: {}", e);
    });
}
