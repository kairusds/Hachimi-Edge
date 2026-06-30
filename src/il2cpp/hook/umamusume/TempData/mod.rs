use crate::il2cpp::types::*;

pub mod JukeboxSetListPlayingData;

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, TempData);

    JukeboxSetListPlayingData::init(TempData)
}