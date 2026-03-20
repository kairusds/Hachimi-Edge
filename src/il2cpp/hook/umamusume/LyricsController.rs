use std::path::Path;

use fnv::FnvHashMap;

use crate::{
    core::{ext::Utf16StringExt, Hachimi, game::Region},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        symbols::{get_field_from_name, get_field_object_value, get_method_addr, Array, Dictionary},
        types::*
    }
};

static mut LYRICSDATADIC_FIELD: *mut FieldInfo = 0 as _;
fn get__lyricsDataDic(this: *mut Il2CppObject) -> Dictionary<i32, Array<u8>> {
    Dictionary::from(get_field_object_value(this, unsafe { LYRICSDATADIC_FIELD }))
}

// dead code needed here, currently unused in-game
#[repr(i32)]
#[allow(dead_code)]
enum AdditionalSetting {
    None = 0,
    SheetVariationId = 1
}

trait LyricsDataCommon {
    fn time(&self) -> f32;
    fn lyrics_mut(&mut self) -> &mut *mut Il2CppString;

    fn get_key(&self) -> i32 {
        f32::to_bits(self.time()).cast_signed()
    }
}

#[repr(C)]
struct LyricsDataJP {
    time: f32,
    lyrics: *mut Il2CppString,
    additionalsetting_type: AdditionalSetting,
    additionalsetting_value: i32
}

#[repr(C)]
struct LyricsDataGlobal {
    time: f32,
    lyrics: *mut Il2CppString,
}

impl LyricsDataCommon for LyricsDataJP {
    fn time(&self) -> f32 { self.time }
    fn lyrics_mut(&mut self) -> &mut *mut Il2CppString { &mut self.lyrics }
}

impl LyricsDataCommon for LyricsDataGlobal {
    fn time(&self) -> f32 { self.time }
    fn lyrics_mut(&mut self) -> &mut *mut Il2CppString { &mut self.lyrics }
}

type LoadLyricsFn = extern "C" fn(this: *mut Il2CppObject, id: i32, path: *mut Il2CppString) -> bool;
extern "C" fn LoadLyrics(this: *mut Il2CppObject, id: i32, path: *mut Il2CppString) -> bool {
    if !get_orig_fn!(LoadLyrics, LoadLyricsFn)(this, id, path) {
        return false;
    }

    // Live/MusicScores/mXXXX/mXXXX_lyrics
    let path_str = unsafe { (*path).as_utf16str() };

    let mut dict_path = Path::new("lyrics").join(path_str.path_filename().to_string());
    dict_path.set_extension("json");
    let localized_data = Hachimi::instance().localized_data.load();
    let Some(dict): Option<FnvHashMap<i32, String>> = localized_data.load_assets_dict(Some(&dict_path)) else {
        return true;
    };
    // dont let pbork interactive know about this
    let secs_dict: FnvHashMap<i32, String> = dict.into_iter()
        .map(|(time, lyrics)| (f32::to_bits(time as f32 / 1000.0).cast_signed(), lyrics) )
        .collect();

    let lyrics_data_dict = get__lyricsDataDic(this);
    let Some(lyrics_data_array) = lyrics_data_dict.get(&id) else {
        return true;
    };

    let process_element = |data: &mut dyn LyricsDataCommon| {
        let time_key = data.get_key();
        if let Some(text) = secs_dict.get(&time_key) {
            *data.lyrics_mut() = text.to_il2cpp_string();
        }
    };

    unsafe {
        let raw_array: *mut Il2CppArray = lyrics_data_array.this;

        let length = (*raw_array).max_length;

        let klass_ref: &mut *mut Il2CppClass =
            (&mut (*raw_array).obj.__bindgen_anon_1.klass).as_mut();

        let element_size = (*(*klass_ref)).element_size as usize;

        let data_ptr = (raw_array as *mut u8).add(kIl2CppSizeOfArray);

        match Hachimi::instance().game.region {
            Region::Japan => {
                if element_size != std::mem::size_of::<LyricsDataJP>() {
                    return true;
                }

                for i in 0..length {
                    let element_ptr = data_ptr.add(i * element_size) as *mut LyricsDataJP;
                    process_element(&mut *element_ptr);
                }
            }
            _ => {
                if element_size != std::mem::size_of::<LyricsDataGlobal>() {
                    // Log an error.
                    return true;
                }

                for i in 0..length {
                    let element_ptr = data_ptr.add(i * element_size) as *mut LyricsDataGlobal;
                    process_element(&mut *element_ptr);
                }
            }
        }
    }

    true
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop.Live", LyricsController);

    let LoadLyrics_addr = get_method_addr(LyricsController, c"LoadLyrics", 2);

    new_hook!(LoadLyrics_addr, LoadLyrics);

    unsafe {
        LYRICSDATADIC_FIELD = get_field_from_name(LyricsController, c"_lyricsDataDic");
    }
}