use crate::core::game::Region;
use crate::core::Hachimi;
use crate::il2cpp::hook::umamusume::Screen as GallopScreen;
use crate::il2cpp::hook::UnityEngine_CoreModule;
use crate::il2cpp::hook::UnityEngine_CoreModule::Screen as UnityScreen;
use crate::windows::hachimi_impl::ResolutionScaling;
use windows::core::{w, PCSTR, PCWSTR};
use windows::Win32::Foundation;
use windows::Win32::Foundation::{FALSE, HWND, POINT, TRUE};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
static mut _HWND: HWND = HWND(0);

static mut GET_CURSOR_POS: isize = 0;
type GetCursorPosFn = extern "system" fn(*mut Foundation::POINT) -> Foundation::BOOL;
extern "system" fn get_cursor_pos(lppoint: *mut Foundation::POINT) -> Foundation::BOOL {
    let orig_fn = unsafe { std::mem::transmute::<isize, GetCursorPosFn>(GET_CURSOR_POS) };
    if orig_fn(lppoint) == FALSE {
        return FALSE;
    }
    unsafe {
        let window_width = UnityEngine_CoreModule::Screen::get_width();
        let window_height = UnityEngine_CoreModule::Screen::get_height();
        if window_height > window_width {
            return TRUE;
        }
        _ = ScreenToClient(_HWND, lppoint);
        if _HWND.0 == 0 {
            return TRUE;
        }

        let mut scale = 1f64;
        match Hachimi::instance().config.load().windows.resolution_scaling {
            ResolutionScaling::ScaleToScreenSize => {
                let resolution = UnityScreen::get_currentResolution();
                scale = resolution.width as f64 / 1920f64;
            }
            ResolutionScaling::ScaleToWindowSize => {
                scale = window_width as f64 / 1920f64;
            }
            ResolutionScaling::Default => {}
        }
        let mut y = (*lppoint).y as f64;
        let mut x = (*lppoint).x as f64;
        x *= scale;
        y = window_height as f64 - ((window_height as f64 - y) * scale);
        (*lppoint).x = x as i32;
        (*lppoint).y = y as i32;
        _ = ClientToScreen(_HWND, lppoint);
    }
    TRUE
}

pub fn init() {
    unsafe {
        let window_width = UnityEngine_CoreModule::Screen::get_width();
        let window_height = UnityEngine_CoreModule::Screen::get_height();
        if window_height < window_width {
            return;
        }
        let handle = GetModuleHandleW(PCWSTR(w!("user32.dll").as_ptr())).unwrap();
        let get_cursor_pos_addr = GetProcAddress(handle, PCSTR("GetCursorPos".as_ptr())).unwrap();
        match Hachimi::instance()
            .interceptor
            .hook(get_cursor_pos_addr as _, get_cursor_pos as _)
        {
            Ok(trampoline_addr) => GET_CURSOR_POS = trampoline_addr as _,
            Err(e) => error!("Failed to hook GetCursorPos: {}", e),
        }

        let hachimi = Hachimi::instance();
        let game = &hachimi.game;

        let window_name = if game.region == Region::Japan && game.is_steam_release {
            // lmao
            w!("UmamusumePrettyDerby_Jpn")
        } else {
            // global technically has "Umamusume" as its title but this api
            // is case insensitive so it works. why am i surprised
            w!("umamusume")
        };
        _HWND = FindWindowW(w!("UnityWndClass"), window_name);
        if _HWND.0 == 0 {
            error!("Failed to find game window");
            return;
        }
    }
}
