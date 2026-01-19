use std::{
    collections::HashSet,
    ffi::CString,
    fs,
    path::{Path, PathBuf},
};

use crate::core::{plugin_api::Plugin, Hachimi};

pub fn load_libraries() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    let mut loaded = HashSet::new();
    let config = Hachimi::instance().config.load();
    let names = &config.android.load_libraries;

    if names.is_empty() {
        if let Some(lib_dir) = find_native_lib_dir() {
            for entry in collect_candidate_libs(&lib_dir) {
                let display = entry.display().to_string();
                if loaded.contains(&display) {
                    continue;
                }
                if let Some(plugin) = try_load_library(&display) {
                    loaded.insert(display);
                    plugins.push(plugin);
                }
            }
        } else {
            warn!("Failed to locate native lib dir for plugin autoscan");
        }
    } else {
        for name in names.iter() {
            if loaded.contains(name) {
                continue;
            }
            if let Some(plugin) = try_load_library(name) {
                loaded.insert(name.clone());
                plugins.push(plugin);
            }
        }
    }

    plugins
}

fn try_load_library(name_or_path: &str) -> Option<Plugin> {
    let Ok(name_cstr) = CString::new(name_or_path) else {
        warn!("Invalid library name: {}", name_or_path);
        return None;
    };

    let handle = unsafe { libc::dlopen(name_cstr.as_ptr(), libc::RTLD_NOW) };
    if handle.is_null() {
        let err = unsafe { libc::dlerror() };
        if err.is_null() {
            warn!("Failed to load library: {}", name_or_path);
        } else {
            let err = unsafe { std::ffi::CStr::from_ptr(err) };
            warn!(
                "Failed to load library: {} ({})",
                name_or_path,
                err.to_string_lossy()
            );
        }
        return None;
    }

    let init_addr = unsafe { libc::dlsym(handle, c"hachimi_init".as_ptr()) };
    if init_addr.is_null() {
        warn!("Library loaded but missing hachimi_init: {}", name_or_path);
        unsafe {
            libc::dlclose(handle);
        }
        return None;
    }

    info!("Loaded library: {}", name_or_path);
    Some(Plugin {
        name: name_or_path.to_string(),
        init_fn: unsafe { std::mem::transmute(init_addr) },
    })
}

fn find_native_lib_dir() -> Option<PathBuf> {
    let maps = fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        let Some(path) = line.split_whitespace().last() else {
            continue;
        };
        if path.ends_with("/libmain.so") {
            return Path::new(path).parent().map(Path::to_path_buf);
        }
    }
    None
}

const AUTOSCAN_PREFIX: &str = "libhachimi_";

fn collect_candidate_libs(lib_dir: &Path) -> Vec<PathBuf> {
    let mut libs = Vec::new();
    let Ok(entries) = fs::read_dir(lib_dir) else {
        return libs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if !file_name.starts_with(AUTOSCAN_PREFIX) || !file_name.ends_with(".so") {
            continue;
        }
        libs.push(path);
    }
    libs
}
