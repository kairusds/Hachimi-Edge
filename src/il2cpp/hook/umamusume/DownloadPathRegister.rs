use crate::il2cpp::{symbols::get_method_overload_addr, types::*};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

// public Void RegisterPath(String[] pathArray) { }
static mut REGISTER_PATH_ARRAY_ADDR: usize = 0;
impl_addr_wrapper_fn!(RegisterPath, REGISTER_PATH_ARRAY_ADDR, (), this: *mut Il2CppObject,path_array: *mut Il2CppArray);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DownloadPathRegister);

    unsafe {
        CLASS = DownloadPathRegister;
        REGISTER_PATH_ARRAY_ADDR = get_method_overload_addr(DownloadPathRegister, "RegisterPath", &[Il2CppTypeEnum_IL2CPP_TYPE_SZARRAY]);
    }
}
