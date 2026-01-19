use std::ffi::CString;

use crate::core::{plugin_api::Plugin, Hachimi};

pub fn load_libraries() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    for name in Hachimi::instance().config.load().android.load_libraries.iter() {
        let Ok(name_cstr) = CString::new(name.as_str()) else {
            warn!("Invalid library name: {}", name);
            continue;
        };

        let handle = unsafe { libc::dlopen(name_cstr.as_ptr(), libc::RTLD_NOW) };
        if handle.is_null() {
            warn!("Failed to load library: {}", name);
            continue;
        }

        info!("Loaded library: {}", name);
        let init_addr = unsafe { libc::dlsym(handle, c"hachimi_init".as_ptr()) };
        if !init_addr.is_null() {
            plugins.push(Plugin {
                name: name.clone(),
                init_fn: unsafe { std::mem::transmute(init_addr) }
            });
        }
    }

    plugins
}
