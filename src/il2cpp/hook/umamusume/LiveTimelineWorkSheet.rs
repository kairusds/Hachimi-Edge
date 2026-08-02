use crate::{
    il2cpp::{
        symbols::get_field_from_name,
        types::*,
    },
};

def_field_object_accessors!(get_postFilmKeys, set_postFilmKeys, POST_FILM_KEYS_FIELD, Il2CppObject);
def_field_object_accessors!(get_postFilm2Keys, set_postFilm2Keys, POST_FILM_2_KEYS_FIELD, Il2CppObject);
def_field_object_accessors!(get_postFilm3Keys, set_postFilm3Keys, POST_FILM_3_KEYS_FIELD, Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live.Cutt", LiveTimelineWorkSheet);

    unsafe {
        POST_FILM_KEYS_FIELD = get_field_from_name(LiveTimelineWorkSheet, c"postFilmKeys");
        POST_FILM_2_KEYS_FIELD = get_field_from_name(LiveTimelineWorkSheet, c"postFilm2Keys");
        POST_FILM_3_KEYS_FIELD = get_field_from_name(LiveTimelineWorkSheet, c"postFilm3Keys");
    }
}
