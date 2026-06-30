use crate::il2cpp::types::*;

pub mod JukeboxSetlistMusicData;

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, MasterJukeboxSetlistMusicData);

    JukeboxSetlistMusicData::init(MasterJukeboxSetlistMusicData)
}