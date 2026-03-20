use crate::il2cpp::types::*;

pub mod Data;

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FormType {
    NONE = -1, // 4294967295 as i32 is -1
    SMALL_NO_BUTTON = 0,
    SMALL_ONE_BUTTON = 1,
    SMALL_TWO_BUTTON = 2,
    SMALL_THREE_BUTTON = 3,
    MIDDLE_NO_BUTTON = 4,
    MIDDLE_ONE_BUTTON = 5,
    MIDDLE_TWO_BUTTON = 6,
    MIDDLE_THREE_BUTTON = 7,
    BIG_NO_BUTTON = 8,
    BIG_ONE_BUTTON = 9,
    BIG_TWO_BUTTON = 10,
    BIG_THREE_BUTTON = 11,
    WITHOUT_FRAME = 12
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, DialogCommon);

    Data::init(DialogCommon)
}