#![allow(non_snake_case, non_upper_case_globals)]
// These are meant to be hook functions, but they also act as proxies.

use std::os::raw::c_uint;

use widestring::{U16CString, Utf16Str};
use windows::{
    core::{HRESULT, PCWSTR},
    Win32::{
        Foundation::{E_NOTIMPL, S_OK},
        System::LibraryLoader::LoadLibraryW
    }
};

use crate::windows::utils;

proxy_proc!(CreateDXGIFactory, CreateDXGIFactory_orig);
proxy_proc!(CreateDXGIFactory1, CreateDXGIFactory1_orig);
proxy_proc!(CreateDXGIFactory2, CreateDXGIFactory2_orig);
proxy_proc!(DXGIGetDebugInterface1, DXGIGetDebugInterface1_orig);

// Windows 10 version 1803 and up
static mut DXGIDeclareAdapterRemovalSupport_orig: usize = 0;
type DXGIDeclareAdapterRemovalSupportFn = extern "C" fn() -> HRESULT;
#[no_mangle]
pub unsafe extern "C" fn DXGIDeclareAdapterRemovalSupport() -> HRESULT {
    let addr = DXGIDeclareAdapterRemovalSupport_orig;
    if addr == 0 {
        return S_OK;
    }

    let orig_fn: DXGIDeclareAdapterRemovalSupportFn = std::mem::transmute(addr);
    orig_fn()
}

// These are called internally by the Direct3D driver on some versions of Windows (even when using d3d11)
// Bogus but compatible fn typedef, dont mind it
static mut DXGID3D10CreateDevice_orig: usize = 0;
type DXGID3D10CreateDeviceFn = extern "C" fn(a: usize, b: usize, c: usize, d: c_uint, e: usize, f: c_uint, g: usize) -> HRESULT;
#[no_mangle]
pub unsafe extern "C" fn DXGID3D10CreateDevice(a: usize, b: usize, c: usize, d: c_uint, e: usize, f: c_uint, g: usize) -> HRESULT {
    let addr = DXGID3D10CreateDevice_orig;
    if addr == 0 {
        return E_NOTIMPL;
    }

    let orig_fn: DXGID3D10CreateDeviceFn = std::mem::transmute(addr);
    orig_fn(a, b, c, d, e, f, g)
}

static mut DXGID3D10RegisterLayers_orig: usize = 0;
type DXGID3D10RegisterLayersFn = extern "C" fn(a: usize, b: c_uint) -> HRESULT;
#[no_mangle]
pub unsafe extern "C" fn DXGID3D10RegisterLayers(a: usize, b: c_uint) -> HRESULT {
    let addr = DXGID3D10RegisterLayers_orig;
    if addr == 0 {
        return E_NOTIMPL;
    }

    let orig_fn: DXGID3D10RegisterLayersFn = std::mem::transmute(addr);
    orig_fn(a, b)
}

pub fn init(system_dir: &Utf16Str) {
    let dll_path = system_dir.to_owned() + "\\dxgi.dll";
    let dll_path_cstr = U16CString::from_vec(dll_path.into_vec()).unwrap();
    let handle = unsafe { LoadLibraryW(PCWSTR(dll_path_cstr.as_ptr())).expect("dxgi.dll") };

    unsafe {
        CreateDXGIFactory_orig = utils::get_proc_address(handle, c"CreateDXGIFactory");
        CreateDXGIFactory1_orig = utils::get_proc_address(handle, c"CreateDXGIFactory1");
        CreateDXGIFactory2_orig = utils::get_proc_address(handle, c"CreateDXGIFactory2");
        DXGIGetDebugInterface1_orig = utils::get_proc_address(handle, c"DXGIGetDebugInterface1");
        DXGIDeclareAdapterRemovalSupport_orig = utils::get_proc_address(handle, c"DXGIDeclareAdapterRemovalSupport");
        DXGID3D10CreateDevice_orig = utils::get_proc_address(handle, c"DXGID3D10CreateDevice");
        DXGID3D10RegisterLayers_orig = utils::get_proc_address(handle, c"DXGID3D10RegisterLayers");
    }
}