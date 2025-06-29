use crate::il2cpp::{symbols::{get_method, get_method_addr}, types::*};

static mut GET_BUTTONGALLERY_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_ButtonGallery, GET_BUTTONGALLERY_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

static mut GET_BUTTONTALKGALLERY_ADDR: usize = 0;
pub fn get_ButtonTalkGallery(this: *mut Il2CppObject) -> *mut Il2CppObject {
    let addr = unsafe { GET_BUTTONTALKGALLERY_ADDR };
    if addr == 0 {
        return 0 as _;
    }

    let orig_fn: extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject = unsafe {
        std::mem::transmute(addr)
    };
    orig_fn(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, CharacterNoteTopView);

    unsafe {
        GET_BUTTONGALLERY_ADDR = get_method_addr(CharacterNoteTopView, c"get_ButtonGallery", 0);

        if let Ok(method) = get_method(CharacterNoteTopView, c"get_ButtonTalkGallery", 0) {
            GET_BUTTONTALKGALLERY_ADDR = (*method).methodPointer;
        }
    }
}