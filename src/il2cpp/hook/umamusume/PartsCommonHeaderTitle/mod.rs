mod TitlePlayer;

use crate::il2cpp::types::*;

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsCommonHeaderTitle);
    
    TitlePlayer::init(PartsCommonHeaderTitle);
}