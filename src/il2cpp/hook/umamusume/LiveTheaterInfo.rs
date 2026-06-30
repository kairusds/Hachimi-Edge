use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{StringExt, Il2CppStringExt},
        hook::umamusume::Director,
        symbols::{Array, get_assembly_image, get_class, get_method_addr},
        types::*
    }
};
use super::DownloadPathRegister;

type RegisterDownloadForLiveDirectorFn = extern "C" fn(register: *mut Il2CppObject, id: i32);
extern "C" fn RegisterDownloadForLiveDirector(register: *mut Il2CppObject, id: i32) {
    get_orig_fn!(RegisterDownloadForLiveDirector, RegisterDownloadForLiveDirectorFn)(register, id);

    let config = Hachimi::instance().config.load();
    if !config.champions_live_show_text { return; }

    if id != 1054 { return; }

    // Master3dLive.Live3dData.ExtraResource ChampionsMeeting = 1
    Director::RegisterDownloadExtraResource(register, 1);

    let mscorlib = match get_assembly_image(c"mscorlib.dll") {
        Ok(img) => img,
        Err(_) => return
    };
    let string_class = match get_class(mscorlib, c"System", c"String") {
        Ok(c) => c,
        Err(_) => return
    };

    let resource_id = config.champions_live_resource_id;
    let texture_year_idx = (config.champions_live_year - 2022).max(0);
    let paths = [
        format!("Live/Image/Champions/tex_championslive_racename_{:02}", resource_id),
        format!("Live/Image/Champions/tex_championslive_year_{:02}", texture_year_idx),
        format!("Live/Image/Champions/tex_championslive_year_l_{:02}", texture_year_idx),
    ];

    let path_array = Array::<*mut Il2CppString>::new(string_class, paths.len());
    if path_array.this.is_null() { return; }

    unsafe {
        let slice = path_array.as_slice();
        for (i, p) in paths.iter().enumerate() {
            slice[i] = p.to_il2cpp_string();
            info!("{}", (&*slice[i]).as_utf16str().to_string());
        }
    }

    DownloadPathRegister::RegisterPath(register, path_array.this);

    info!(
        "Pre-registered ChampionsMeeting bundle + title/year textures for Champions Live (music_id=1054, resource_id={}, year={} {:02}, {} paths)",
        resource_id, config.champions_live_year, texture_year_idx, paths.len()
    );
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, LiveTheaterInfo);

    let addr = get_method_addr(LiveTheaterInfo, c"RegisterDownloadForLiveDirector", 2);
    new_hook!(addr, RegisterDownloadForLiveDirector);
}
