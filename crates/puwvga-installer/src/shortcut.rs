use std::path::Path;
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

/// Creates a Windows `.lnk` shortcut using Win32 COM `IShellLinkW` + `IPersistFile`
/// without spawning PowerShell or any external shell process.
pub fn create_shortcut(
    lnk_path: &Path,
    target_exe: &Path,
    working_dir: &Path,
    icon_path: &Path,
    description: &str,
) -> Result<(), String> {
    if let Some(parent) = lnk_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create shortcut dir: {e}"))?;
    }

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let needs_uninit = hr.is_ok();

        let result = (|| -> Result<(), String> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("CoCreateInstance(ShellLink): {e}"))?;

            let target_h = HSTRING::from(target_exe.as_os_str());
            let dir_h = HSTRING::from(working_dir.as_os_str());
            let icon_h = HSTRING::from(icon_path.as_os_str());
            let desc_h = HSTRING::from(description);

            link.SetPath(PCWSTR(target_h.as_ptr()))
                .map_err(|e| format!("SetPath: {e}"))?;
            link.SetWorkingDirectory(PCWSTR(dir_h.as_ptr()))
                .map_err(|e| format!("SetWorkingDirectory: {e}"))?;
            link.SetIconLocation(PCWSTR(icon_h.as_ptr()), 0)
                .map_err(|e| format!("SetIconLocation: {e}"))?;
            link.SetDescription(PCWSTR(desc_h.as_ptr()))
                .map_err(|e| format!("SetDescription: {e}"))?;

            let persist: IPersistFile = link
                .cast()
                .map_err(|e| format!("QueryInterface(IPersistFile): {e}"))?;
            let lnk_h = HSTRING::from(lnk_path.as_os_str());
            persist
                .Save(PCWSTR(lnk_h.as_ptr()), true)
                .map_err(|e| format!("IPersistFile::Save: {e}"))?;
            Ok(())
        })();

        if needs_uninit {
            CoUninitialize();
        }
        result
    }
}
