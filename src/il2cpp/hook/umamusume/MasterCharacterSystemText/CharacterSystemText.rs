use crate::{
    il2cpp::{
        symbols::get_field_from_name,
        types::*
    }
};

def_field_value_accessors!(get_CueId, set_CueId, _CUEID_FIELD, i32);
def_field_object_accessors!(get_CueSheet, set_CueSheet, _CUESHEET_FIELD, Il2CppString);
def_field_object_accessors!(get_Text, set_Text, _TEXT_FIELD, Il2CppString);
def_field_value_accessors!(get_VoiceId, set_VoiceId, _VOICEID_FIELD, i32);

pub fn init(MasterCharacterSystemText: *mut Il2CppClass) {
    find_nested_class_or_return!(MasterCharacterSystemText, CharacterSystemText);

    unsafe {
        _CUEID_FIELD = get_field_from_name(CharacterSystemText, c"CueId");
        _CUESHEET_FIELD = get_field_from_name(CharacterSystemText, c"CueSheet");
        _TEXT_FIELD = get_field_from_name(CharacterSystemText, c"Text");
        _VOICEID_FIELD = get_field_from_name(CharacterSystemText, c"VoiceId");
    }
}
