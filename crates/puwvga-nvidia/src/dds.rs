//! NVIDIA "Display Mode" (Dynamic Display Switching / Advanced Optimus MUX).
//!
//! NVIDIA publishes no API for this. NVIDIA App's `NvCplDisplayPlugin` switches it through the
//! `NvCpl.dll` that ships with NVIDIA App (the driver's own NVCPL.DLL lacks the DDS setting):
//! `GetInfo(0x17)` reads a 16-byte state, `SetSetting(.., 0x332, state, 16)` + `Execute` applies it.
//! The calls below replay what that plugin was observed doing (Frida trace, NVIDIA App 11.0.9).
use libloading::Library;
use puwvga_core::BackendError;
use std::ffi::c_void;

pub const NVCPL_PATH: &str = r"C:\Program Files\NVIDIA Corporation\NVIDIA App\NvCpl\NvCpl.dll";

const INFO_DDS_STATE: u32 = 0x17;
const SETTING_DDS_INTERNAL_MUX_STATE: u32 = 0x332;
const EXECUTE_COMMIT: u32 = 0x10000;
/// `LoadLibraryExW` flag so NvCpl.dll finds NvUI.dll and friends next to itself.
const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x8;

/// `MuxState` as NVIDIA App's schema names it; equals `InternalMuxState` in the driver registry.
pub const MUX_IGPU: u32 = 1;
pub const MUX_DGPU: u32 = 2;

type FnInit = unsafe extern "C" fn() -> i32;
type FnGetInfo = unsafe extern "C" fn(u32, *mut c_void, *mut u8, *mut u32) -> i32;
type FnSetSetting = unsafe extern "C" fn(u32, *mut u8, u32, *mut u8, u64) -> i32;
type FnExecute = unsafe extern "C" fn(u32, *mut c_void, u32, *mut c_void) -> i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DdsState {
    pub supported: bool,
    pub automatic: bool,
    pub mux: u32,
}

/// Layout: `u8 bIsSupported, u8 bIsAutomatic, u16 pad, u32 MuxState, u8 reserved[8]`.
fn encode(s: DdsState) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0] = u8::from(s.supported);
    b[1] = u8::from(s.automatic);
    b[4..8].copy_from_slice(&s.mux.to_le_bytes());
    b
}

fn decode(b: &[u8; 16]) -> DdsState {
    DdsState {
        supported: b[0] != 0,
        automatic: b[1] != 0,
        mux: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
    }
}

pub struct Dds {
    _lib: Library,
    get_info: FnGetInfo,
    set_setting: FnSetSetting,
    execute: FnExecute,
}

// Safety: the function pointers stay valid while `_lib` is alive; calls are made from the UI thread.
unsafe impl Send for Dds {}
unsafe impl Sync for Dds {}

impl Dds {
    /// `None` when NVIDIA App (and so its NvCpl.dll) is not installed or the init call fails.
    pub fn load() -> Option<Self> {
        let lib: Library = unsafe {
            libloading::os::windows::Library::load_with_flags(NVCPL_PATH, LOAD_WITH_ALTERED_SEARCH_PATH)
        }
        .ok()?
        .into();
        unsafe {
            let init: FnInit = *lib.get(b"NvCplApiInit\0").ok()?;
            let get_info: FnGetInfo = *lib.get(b"NvCplApiGetInfo\0").ok()?;
            let set_setting: FnSetSetting = *lib.get(b"NvCplApiSetSetting\0").ok()?;
            let execute: FnExecute = *lib.get(b"NvCplApiExecute\0").ok()?;
            if init() != 0 {
                return None;
            }
            Some(Self { _lib: lib, get_info, set_setting, execute })
        }
    }

    pub fn state(&self) -> Option<DdsState> {
        let mut buf = [0u8; 16];
        let mut size = buf.len() as u32;
        let st = unsafe { (self.get_info)(INFO_DDS_STATE, std::ptr::null_mut(), buf.as_mut_ptr(), &mut size) };
        let state = decode(&buf);
        (st == 0 && state.supported).then_some(state)
    }

    /// Switches like NVIDIA App's Apply: `automatic` keeps the current MUX and lets the driver
    /// decide; otherwise `mux` (`MUX_IGPU` / `MUX_DGPU`) is forced.
    pub fn set(&self, automatic: bool, mux: u32) -> Result<(), BackendError> {
        let mut state = encode(DdsState { supported: true, automatic, mux });
        let mut target = [0u8; 32];
        let st = unsafe {
            (self.set_setting)(1, target.as_mut_ptr(), SETTING_DDS_INTERNAL_MUX_STATE, state.as_mut_ptr(), 16)
        };
        if st != 0 {
            return Err(BackendError::DriverError { code: st, message: "NvCplApiSetSetting(DDS) failed".into() });
        }
        let st = unsafe { (self.execute)(EXECUTE_COMMIT, std::ptr::null_mut(), u32::MAX, std::ptr::null_mut()) };
        if st != 0 {
            return Err(BackendError::DriverError { code: st, message: "NvCplApiExecute(DDS) failed".into() });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dds_state_matches_observed_bytes() {
        // Bytes NVIDIA App sent for Optimus / Auto / NVIDIA GPU.
        let optimus = [1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let auto = [1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let dgpu = [1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(encode(DdsState { supported: true, automatic: false, mux: MUX_IGPU }), optimus);
        assert_eq!(encode(DdsState { supported: true, automatic: true, mux: MUX_IGPU }), auto);
        assert_eq!(encode(DdsState { supported: true, automatic: false, mux: MUX_DGPU }), dgpu);
        assert_eq!(decode(&dgpu), DdsState { supported: true, automatic: false, mux: MUX_DGPU });
    }
}
