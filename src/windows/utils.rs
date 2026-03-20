use std::{ffi::CStr, path::PathBuf};

use widestring::{U16CString, Utf16Str, Utf16String};
use windows::{
    core::{w, PCSTR, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HMODULE, HWND, MAX_PATH},
        System::{
            Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPALL},
            LibraryLoader::{GetModuleFileNameW, GetProcAddress},
            SystemInformation::GetSystemDirectoryW,
            Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE}
        },
        UI::WindowsAndMessaging::{MessageBoxW, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, MB_ICONERROR, MB_OK, SWP_NOMOVE, SWP_NOSIZE}
    }
};

use crate::core::{utils::scale_to_aspect_ratio, Hachimi};
use rust_i18n::t;
use windows::Win32::UI::Input::KeyboardAndMouse::{self as km, VIRTUAL_KEY};

use super::hachimi_impl::ResolutionScaling;

pub fn _get_system_directory() -> Utf16String {
    let mut buffer = [0u16; MAX_PATH as usize];
    let length = unsafe { GetSystemDirectoryW(Some(&mut buffer)) };
    unsafe { Utf16String::from_vec_unchecked(buffer[..length as usize].to_vec()) }
}

pub fn get_proc_address(hmodule: HMODULE, name: &CStr) -> usize {
    let res = unsafe { GetProcAddress(hmodule, PCSTR(name.as_ptr() as *const u8)) };
    if let Some(proc) = res {
        proc as usize
    }
    else {
        0
    }
}

pub fn get_exec_path() -> PathBuf {
    let mut slice = [0u16; MAX_PATH as usize];
    let length = unsafe { GetModuleFileNameW(None, &mut slice) } as usize;
    let exec_path_str = unsafe { Utf16Str::from_slice_unchecked(&slice[..length]) }.to_string();

    PathBuf::from(exec_path_str)
}

pub fn get_game_dir() -> PathBuf {
    let exec_path = get_exec_path();
    let parent = exec_path.parent().unwrap();
    parent.to_owned()
}

/*
pub fn get_game_dir_str() -> Option<String> {
    let mut slice = [0u16; MAX_PATH as usize];
    let length = unsafe { GetModuleFileNameW(HMODULE::default(), &mut slice) } as usize;
    let exec_path_str = unsafe { Utf16Str::from_slice_unchecked(&slice[..length]) }.to_string();
    let exec_path = Path::new(&exec_path_str);
    let parent = exec_path.parent()?;

    Some(parent.display().to_string())
}
*/

pub unsafe fn kill_process_by_name(target_name: &CStr) -> Result<(), windows::core::Error> {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPALL, 0)?;
    let mut entry = PROCESSENTRY32::default();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
    let mut res = Process32First(snapshot, &mut entry);

    while res.is_ok() {
        let process_name = CStr::from_ptr(entry.szExeFile.as_ptr());
        if process_name == target_name {
            if let Ok(process) = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID) {
                TerminateProcess(process, 0)?;
                CloseHandle(process)?;
            }
        }

        res = Process32Next(snapshot, &mut entry);
    }

    Ok(())
}

pub fn get_tmp_installer_path() -> PathBuf {
    let mut installer_path = std::env::temp_dir();
    installer_path.push("hachimi_installer.exe");
    installer_path
}

pub fn get_scaling_res() -> Option<(i32, i32)> {
    use crate::il2cpp::hook::UnityEngine_CoreModule::Screen as UnityScreen;
    use crate::il2cpp::hook::umamusume::Screen as GallopScreen;

    match Hachimi::instance().config.load().windows.resolution_scaling {
        ResolutionScaling::Default => None,
        ResolutionScaling::ScaleToScreenSize => {
            let res = UnityScreen::get_currentResolution(); // screen res, not game window res
            let aspect_ratio = GallopScreen::get_Width_orig() as f32 / GallopScreen::get_Height_orig() as f32;
            Some(scale_to_aspect_ratio((res.width, res.height), aspect_ratio, true))
        },
        ResolutionScaling::ScaleToWindowSize => {
            let mut width = UnityScreen::get_width();
            let mut height = UnityScreen::get_height();
            if (GallopScreen::get_Width_orig() > GallopScreen::get_Height_orig()) != (width > height) {
                std::mem::swap(&mut width, &mut height);
            }
            Some((width, height))
        },
    }
}

pub unsafe fn set_window_topmost(hwnd: HWND, topmost: bool) -> Result<(), windows::core::Error> {
    let insert_after = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
    SetWindowPos(hwnd, Some(insert_after), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE)
}

pub fn show_error(e: impl AsRef<str>) {
    let s = e.as_ref();
    error!("{}", s);

    let cstr = U16CString::from_str(s).unwrap();
    unsafe { MessageBoxW(None, PCWSTR(cstr.as_ptr()), w!("Hachimi Error"), MB_ICONERROR | MB_OK); }
}

pub fn vk_to_display_label(vk: u16) -> String {
    if (0x41..=0x5A).contains(&vk) { // A-Z
        return (vk as u8 as char).to_string();
    }
    if (0x30..=0x39).contains(&vk) { // 0-9
        return (vk as u8 as char).to_string();
    }

    let code = VIRTUAL_KEY(vk).0;

    // Function keys as a range
    if code >= km::VK_F1.0 && code <= km::VK_F24.0 {
        let n = (code - km::VK_F1.0) + 1;
        return format!("F{}", n);
    }

    // Localized name mapping
    let name_map: &[(u16, &str)] = &[
        (km::VK_LEFT.0, "key_names.left"),
        (km::VK_RIGHT.0, "key_names.right"),
        (km::VK_UP.0, "key_names.up"),
        (km::VK_DOWN.0, "key_names.down"),
        (km::VK_INSERT.0, "key_names.insert"),
        (km::VK_DELETE.0, "key_names.delete"),
        (km::VK_HOME.0, "key_names.home"),
        (km::VK_END.0, "key_names.end"),
        (km::VK_PRIOR.0, "key_names.page_up"),
        (km::VK_NEXT.0, "key_names.page_down"),
        (km::VK_ESCAPE.0, "key_names.escape"),
        (km::VK_BACK.0, "key_names.backspace"),
        (km::VK_CLEAR.0, "key_names.clear"),
        (km::VK_PAUSE.0, "key_names.pause"),
        (km::VK_CAPITAL.0, "key_names.caps_lock"),
        (km::VK_SCROLL.0, "key_names.scroll_lock"),
        (km::VK_NUMLOCK.0, "key_names.num_lock"),
        (km::VK_SNAPSHOT.0, "key_names.print_screen"),
        (km::VK_LWIN.0, "key_names.left_win"),
        (km::VK_RWIN.0, "key_names.right_win"),
        (km::VK_APPS.0, "key_names.apps"),
        (km::VK_MENU.0, "key_names.alt"),
        (km::VK_CONTROL.0, "key_names.ctrl"),
        (km::VK_SHIFT.0, "key_names.shift"),
        (km::VK_TAB.0, "key_names.tab"),
        (km::VK_RETURN.0, "key_names.enter"),
        (km::VK_SPACE.0, "key_names.space"),
        (km::VK_NUMPAD0.0, "key_names.numpad_0"),
        (km::VK_NUMPAD1.0, "key_names.numpad_1"),
        (km::VK_NUMPAD2.0, "key_names.numpad_2"),
        (km::VK_NUMPAD3.0, "key_names.numpad_3"),
        (km::VK_NUMPAD4.0, "key_names.numpad_4"),
        (km::VK_NUMPAD5.0, "key_names.numpad_5"),
        (km::VK_NUMPAD6.0, "key_names.numpad_6"),
        (km::VK_NUMPAD7.0, "key_names.numpad_7"),
        (km::VK_NUMPAD8.0, "key_names.numpad_8"),
        (km::VK_NUMPAD9.0, "key_names.numpad_9"),
        (km::VK_ADD.0, "key_names.numpad_add"),
        (km::VK_SUBTRACT.0, "key_names.numpad_subtract"),
        (km::VK_MULTIPLY.0, "key_names.numpad_multiply"),
        (km::VK_DIVIDE.0, "key_names.numpad_divide"),
        (km::VK_DECIMAL.0, "key_names.numpad_decimal"),
    ];

    if let Some((_, key)) = name_map.iter().find(|(k, _)| *k == code) {
        return t!(*key).into_owned();
    }

    // OEM symbol mapping
    let oem_map: &[(u16, &str)] = &[
        (km::VK_OEM_1.0, ";"),
        (km::VK_OEM_PLUS.0, "="),
        (km::VK_OEM_COMMA.0, ","),
        (km::VK_OEM_MINUS.0, "-"),
        (km::VK_OEM_PERIOD.0, "."),
        (km::VK_OEM_2.0, "/"),
        (km::VK_OEM_3.0, "`"),
        (km::VK_OEM_4.0, "["),
        (km::VK_OEM_5.0, "\\"),
        (km::VK_OEM_6.0, "]"),
        (km::VK_OEM_7.0, "'"),
        (km::VK_OEM_8.0, "OEM8"),
        (km::VK_OEM_102.0, "<>"),
    ];
    if let Some((_, s)) = oem_map.iter().find(|(k, _)| *k == code) {
        return (*s).to_owned();
    }

    format!("VK 0x{:02X}", vk)
}