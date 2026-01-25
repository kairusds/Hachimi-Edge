use jni::JNIEnv;
use std::path::PathBuf;
use super::game_impl;

pub fn get_device_api_level(env: *mut jni::sys::JNIEnv) -> i32 {
    let mut env = unsafe { JNIEnv::from_raw(env).unwrap() };
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .unwrap()
        .i()
        .unwrap()
}

pub fn get_game_dir() -> PathBuf {
    let package_name = game_impl::get_package_name();
    game_impl::get_data_dir(&package_name)
}