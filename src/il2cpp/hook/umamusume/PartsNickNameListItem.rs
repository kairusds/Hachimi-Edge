use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::umamusume::PartsNickNameRibbon,
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
    },
};

static mut RIBBON_FIELD: *mut FieldInfo = 0 as _;
pub fn get_ribbon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { RIBBON_FIELD })
}

type SetupFn = extern "C" fn(this: *mut Il2CppObject, nickNameId: i32, onSelect: *mut Il2CppDelegate) -> *mut Il2CppObject;
extern "C" fn Setup(this: *mut Il2CppObject, nickNameId: i32, onSelect: *mut Il2CppDelegate) -> *mut Il2CppObject {
    let orig = get_orig_fn!(Setup, SetupFn)(this, nickNameId, onSelect);
    PartsNickNameRibbon::fit_text(get_ribbon(this));
    orig
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsNickNameListItem);

    let initialize_addr = get_method_addr(PartsNickNameListItem, c"Setup", 2);
    new_hook!(initialize_addr, Setup);

    unsafe {
        RIBBON_FIELD = get_field_from_name(PartsNickNameListItem, c"_ribbon");
    }
}
