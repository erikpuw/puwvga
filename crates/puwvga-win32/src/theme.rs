use std::ffi::c_void;
use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
};
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, ShowWindow, SW_HIDE, SW_SHOWNORMAL,
};

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn read_dword(root: HKEY, key: &str, value: &str) -> Option<u32> {
    let subkey = to_wide_null(key);
    let value = to_wide_null(value);
    let mut data: u32 = 0;
    let mut size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Reads Windows 10/11 system app theme preference from registry (`AppsUseLightTheme`).
pub fn is_windows_dark_mode() -> bool {
    read_dword(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "AppsUseLightTheme",
    )
    .is_none_or(|v| v == 0)
}

/// NVIDIA "Manage Display mode" state as the driver persists it after a switch:
/// `(InternalMuxState, InternalMuxIsAutomaticMode)`; state 1 = Optimus (iGPU), 2 = NVIDIA GPU only.
/// Observed on driver 616.92 by diffing the registry around NVIDIA Control Panel switches.
pub fn read_nvidia_mux_state() -> Option<(u32, bool)> {
    const KEY: &str = "SYSTEM\\CurrentControlSet\\Services\\nvlddmkm\\Global\\NvHybrid\\Persistence\\ACE";
    let state = read_dword(HKEY_LOCAL_MACHINE, KEY, "InternalMuxState")?;
    let automatic = read_dword(HKEY_LOCAL_MACHINE, KEY, "InternalMuxIsAutomaticMode").unwrap_or(0);
    Some((state, automatic != 0))
}

/// Opens NVIDIA Control Panel (Microsoft Store package), where the MUX mode is switched.
pub fn open_nvidia_control_panel() -> bool {
    let file = to_wide_null("explorer.exe");
    let args = to_wide_null(
        "shell:AppsFolder\\NVIDIACorp.NVIDIAControlPanel_56jybvy8sckqj!NVIDIACorp.NVIDIAControlPanel",
    );
    let res = unsafe {
        ShellExecuteW(
            HWND::default(),
            PCWSTR::null(),
            PCWSTR(file.as_ptr()),
            PCWSTR(args.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    (res.0 as isize) > 32
}

/// Hides this process's top-level windows with DWM transitions disabled. Call right before
/// `process::exit`: otherwise DWM keeps the last frame on screen for ~500 ms after the process dies.
pub fn hide_process_windows() {
    unsafe extern "system" fn hide_if_ours(hwnd: HWND, _: LPARAM) -> BOOL {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == std::process::id() {
            let disabled = BOOL::from(true);
            unsafe {
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_TRANSITIONS_FORCEDISABLED,
                    &disabled as *const BOOL as *const c_void,
                    size_of::<BOOL>() as u32,
                );
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
        BOOL::from(true)
    }
    unsafe {
        let _ = EnumWindows(Some(hide_if_ours), LPARAM(0));
    }
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
