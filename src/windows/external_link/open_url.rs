use crate::core::game::Region;
use crate::core::Hachimi;
use crate::il2cpp::types::Il2CppString;
use crate::windows::external_link::webview::WM_OPEN_WEBVIEW;
use crate::windows::wnd_hook::get_target_hwnd;
use once_cell::sync::Lazy;
use rust_i18n::t;
use std::collections::HashMap;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

const URL_HANDLER: &[fn(&String, &String, &HashMap<String, String>) -> Option<(String, String)>] =
    &[news_url, general_url];

static mut GACHA_URL_ID_MAP: Lazy<HashMap<String, i32>> = Lazy::new(|| HashMap::new());
fn news_url(
    _url: &String,
    base_url: &String,
    _params: &HashMap<String, String>,
) -> Option<(String, String)> {
    if base_url == "https://dmg.umamusume.jp/news" {
        let api_url =
            "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/info?p=2&c=0"
                .to_string();
        return Some((api_url.to_string(), "Notice".to_string()));
    }
    None
}

pub fn add_gacha_url(url: *mut Il2CppString, gacha_id: i32) {
    let url_string = il2cppstring_as_string(unsafe { &*url });
    unsafe {
        GACHA_URL_ID_MAP.insert(url_string, gacha_id);
    }
}
const BASE_API_URL: &str = "https://api.games.umamusume.jp/umamusume/contents/v/index.html#/";
fn gacha_url(url: &String, params: &HashMap<String, String>) -> Option<String> {
    let gacha_id = unsafe { GACHA_URL_ID_MAP.get(url)? };
    let v = params.get("v")?;
    let r = params.get("r")?;
    let p = params.get("p")?;
    let api_url = format!("{BASE_API_URL}gacha?v={}&r={}&g={}&p={}", v, r, gacha_id, p);
    Some(api_url)
}

fn general_url(
    url: &String,
    base_url: &String,
    params: &HashMap<String, String>,
) -> Option<(String, String)> {
    let mut url_type: String = "general".to_string();
    if base_url.starts_with("https://www.games.umamusume.jp/#/") {
        if let Some(pos) = url.find('?') {
            url_type = base_url[33..pos].to_string();
        }
        let translated_title_key = format!("external_link_dialog.title.{url_type}");
        let title = {
            if let Some(translated) =
                crate::_rust_i18n_try_translate(&rust_i18n::locale(), &translated_title_key)
            {
                translated.into()
            } else {
                rust_i18n::CowStr::from(t!("external_link_dialog.title.general").to_string())
                    .into_inner()
            }
        }
        .to_string();
        let api_url: Option<String> = match url_type.as_str() {
            "gacha" => gacha_url(url, params),
            _ => Some(url.replacen("https://www.games.umamusume.jp/#/", BASE_API_URL, 1)),
        };
        if api_url.is_none() {
            return None;
        }
        return Some((api_url.unwrap(), title.to_string()));
    }
    None
}
pub fn il2cppstring_as_string(string: &Il2CppString) -> String {
    let slice =
        unsafe { std::slice::from_raw_parts(string.chars.as_ptr(), string.length as usize) };
    String::from_utf16_lossy(slice)
}
fn pares_url(url: &String) -> (String, HashMap<String, String>) {
    if let Some(pos) = url.find('?') {
        (
            url[..pos].to_string(),
            url[pos + 1..]
                .split('&')
                .filter_map(|p| {
                    p.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                })
                .collect::<HashMap<String, String>>(),
        )
    } else {
        (url.clone(), HashMap::new())
    }
}
pub fn open(url: *mut Il2CppString) -> bool {
    if Hachimi::instance().game.region == Region::Japan
        && !Hachimi::instance()
            .config
            .load()
            .windows
            .open_external_link_with_hachimi
    {
        return false;
    }

    let url_string = il2cppstring_as_string(unsafe { &*url });

    let (base_url, params) = pares_url(&url_string);
    for handler in URL_HANDLER {
        if let Some((api_url, title)) = handler(&url_string, &base_url, &params) {
            unsafe {
                let _ = PostMessageW(
                    Some(get_target_hwnd()),
                    WM_OPEN_WEBVIEW,
                    Default::default(),
                    LPARAM(Box::into_raw(Box::new((api_url, title, url_string))) as isize),
                );
            }
            return true;
        }
    }
    false
}
