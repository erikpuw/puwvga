use std::ffi::c_void;
use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, HWND};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Reads Windows 10/11 system app theme preference from registry (`AppsUseLightTheme`).
pub fn is_windows_dark_mode() -> bool {
    let subkey = to_wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = to_wide_null("AppsUseLightTheme");
    let mut data: u32 = 0;
    let mut size = size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };

    if status == ERROR_SUCCESS {
        data == 0
    } else {
        true
    }
}

/// Applies native Windows 11 DWM immersive dark titlebar to the given HWND.
pub fn apply_dwm_dark_titlebar(hwnd: isize, dark: bool) {
    if hwnd == 0 {
        return;
    }
    let val = BOOL::from(dark);
    let _ = unsafe {
        DwmSetWindowAttribute(
            HWND(hwnd as *mut c_void),
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &val as *const BOOL as *const c_void,
            size_of::<BOOL>() as u32,
        )
    };
}

/// Relaunches the current executable with UAC elevation (`runas`) — only called on explicit user action.
pub fn relaunch_as_administrator() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let verb = to_wide_null("runas");
    let file = to_wide_null(&exe.to_string_lossy());
    let res = unsafe {
        ShellExecuteW(
            HWND::default(),
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    (res.0 as isize) > 32
}
