use crate::shortcut::create_shortcut;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub const APP_NAME: &str = "puwvga";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_PUBLISHER: &str = "erikpuw";
pub const APP_URL: &str = "https://github.com/erikpuw/puwvga";

const PAYLOAD_GZ: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/puwvga_payload.gz"));
const APP_ICON_ICO: &[u8] = include_bytes!("../../../assets/puwvga.ico");
const LICENSE_TXT: &str = include_str!("../../../LICENSE");

const UNINSTALL_REG_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\puwvga";

pub struct InstallOptions {
    pub install_dir: PathBuf,
    pub create_start_menu: bool,
    pub create_desktop: bool,
}

pub fn default_install_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("Programs").join("puwvga")
    } else {
        PathBuf::from(r"C:\Program Files\puwvga")
    }
}

pub fn start_menu_shortcut_path() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(|appdata| {
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("puwvga.lnk")
    })
}

pub fn desktop_shortcut_path() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .ok()
        .map(|home| PathBuf::from(home).join("Desktop").join("puwvga.lnk"))
}

pub fn decompress_payload() -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(PAYLOAD_GZ);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("Decompress payload failed: {e}"))?;
    if out.is_empty() {
        return Err("Installer payload is empty. Build puwvga-ui in release mode first.".into());
    }
    Ok(out)
}

pub fn perform_install(opts: &InstallOptions) -> Result<PathBuf, String> {
    let exe_bytes = decompress_payload()?;
    fs::create_dir_all(&opts.install_dir)
        .map_err(|e| format!("Cannot create directory {}: {e}", opts.install_dir.display()))?;

    let target_exe = opts.install_dir.join("puwvga.exe");
    let target_ico = opts.install_dir.join("puwvga.ico");
    let target_license = opts.install_dir.join("LICENSE.txt");
    let uninstaller_exe = opts.install_dir.join("puwvga-uninstall.exe");

    fs::write(&target_exe, &exe_bytes)
        .map_err(|e| format!("Failed writing puwvga.exe (close running puwvga first): {e}"))?;
    fs::write(&target_ico, APP_ICON_ICO)
        .map_err(|e| format!("Failed writing puwvga.ico: {e}"))?;
    let _ = fs::write(&target_license, LICENSE_TXT);

    if let Ok(self_exe) = std::env::current_exe() {
        if self_exe != uninstaller_exe {
            let _ = fs::copy(&self_exe, &uninstaller_exe);
        }
    }

    if opts.create_start_menu {
        if let Some(lnk) = start_menu_shortcut_path() {
            create_shortcut(
                &lnk,
                &target_exe,
                &opts.install_dir,
                &target_ico,
                "puwvga - Unified GPU Control Panel",
            )?;
        }
    }

    if opts.create_desktop {
        if let Some(lnk) = desktop_shortcut_path() {
            create_shortcut(
                &lnk,
                &target_exe,
                &opts.install_dir,
                &target_ico,
                "puwvga - Unified GPU Control Panel",
            )?;
        }
    }

    let estimated_kb = ((exe_bytes.len() as u64) / 1024).max(1) as u32;
    register_uninstall_entry(&opts.install_dir, &target_ico, &uninstaller_exe, estimated_kb)?;

    Ok(target_exe)
}

pub fn perform_uninstall(install_dir: &Path) -> Result<(), String> {
    if let Some(lnk) = start_menu_shortcut_path() {
        let _ = fs::remove_file(lnk);
    }
    if let Some(lnk) = desktop_shortcut_path() {
        let _ = fs::remove_file(lnk);
    }
    unregister_uninstall_entry();

    let _ = fs::remove_file(install_dir.join("puwvga.exe"));
    let _ = fs::remove_file(install_dir.join("puwvga.ico"));
    let _ = fs::remove_file(install_dir.join("LICENSE.txt"));
    let _ = fs::remove_dir(install_dir);
    Ok(())
}

pub fn launch_installed_app(exe_path: &Path) {
    let op = HSTRING::from("open");
    let file = HSTRING::from(exe_path.as_os_str());
    unsafe {
        let _ = ShellExecuteW(
            None,
            PCWSTR(op.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn reg_set_sz(hkey: HKEY, name: &str, value: &str) -> Result<(), String> {
    let name_h = HSTRING::from(name);
    let mut wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe {
        std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * std::mem::size_of::<u16>())
    };
    let status = unsafe { RegSetValueExW(hkey, PCWSTR(name_h.as_ptr()), 0, REG_SZ, Some(bytes)) };
    wide.clear();
    if status.is_ok() {
        Ok(())
    } else {
        Err(format!("RegSetValueExW({name}) failed: {:?}", status))
    }
}

fn reg_set_dword(hkey: HKEY, name: &str, value: u32) -> Result<(), String> {
    let name_h = HSTRING::from(name);
    let bytes = value.to_le_bytes();
    let status =
        unsafe { RegSetValueExW(hkey, PCWSTR(name_h.as_ptr()), 0, REG_DWORD, Some(&bytes)) };
    if status.is_ok() {
        Ok(())
    } else {
        Err(format!("RegSetValueExW({name}) failed: {:?}", status))
    }
}

fn register_uninstall_entry(
    install_dir: &Path,
    icon_path: &Path,
    uninstaller_exe: &Path,
    estimated_kb: u32,
) -> Result<(), String> {
    let subkey = HSTRING::from(UNINSTALL_REG_SUBKEY);
    let mut hkey = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if status.is_err() {
        return Err(format!("RegCreateKeyExW failed: {:?}", status));
    }

    let uninstall_cmd = format!("\"{}\" --uninstall", uninstaller_exe.display());
    let res = (|| {
        reg_set_sz(hkey, "DisplayName", APP_NAME)?;
        reg_set_sz(hkey, "DisplayVersion", APP_VERSION)?;
        reg_set_sz(hkey, "Publisher", APP_PUBLISHER)?;
        reg_set_sz(hkey, "URLInfoAbout", APP_URL)?;
        reg_set_sz(hkey, "DisplayIcon", &icon_path.display().to_string())?;
        reg_set_sz(hkey, "InstallLocation", &install_dir.display().to_string())?;
        reg_set_sz(hkey, "UninstallString", &uninstall_cmd)?;
        reg_set_dword(hkey, "NoModify", 1)?;
        reg_set_dword(hkey, "NoRepair", 1)?;
        reg_set_dword(hkey, "EstimatedSize", estimated_kb)?;
        Ok(())
    })();

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    res
}

fn unregister_uninstall_entry() {
    let subkey = HSTRING::from(UNINSTALL_REG_SUBKEY);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_install_dir_ends_with_puwvga() {
        let dir = default_install_dir();
        assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("puwvga"));
    }

    #[test]
    fn embedded_icon_is_valid_ico_header() {
        assert!(APP_ICON_ICO.len() > 22);
        assert_eq!(&APP_ICON_ICO[0..4], &[0, 0, 1, 0]);
    }
}
