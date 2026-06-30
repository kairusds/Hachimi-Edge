use crate::{
    core::captions,
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
};

type DisposeFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Dispose(this: *mut Il2CppObject) {
    get_orig_fn!(Dispose, DisposeFn)(this);
    captions::Captions::cleanup();
}

pub fn init(CriMw_CriWare_Runtime: *const Il2CppImage) {
    get_class_or_return!(CriMw_CriWare_Runtime, CriWare, CriAtomExAcb);

    let dispose_addr = get_method_addr(CriAtomExAcb, c"Dispose", 0);
    new_hook!(dispose_addr, Dispose);
}
