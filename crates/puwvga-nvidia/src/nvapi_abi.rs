use std::ffi::{c_char, c_void};

pub type NvApiStatus = i32;
pub const NVAPI_OK: NvApiStatus = 0;
/// Adding a new setting to the Base profile needs an elevated process.
pub const NVAPI_INVALID_USER_PRIVILEGE: NvApiStatus = -137;

pub type NvPhysicalGpuHandle = *mut c_void;
pub type NvDisplayHandle = *mut c_void;
pub type NvDrsSessionHandle = *mut c_void;
pub type NvDrsProfileHandle = *mut c_void;

// Official NVAPI nvapi_QueryInterface IDs (zero NVML / zero telemetry)
pub const NVAPI_INITIALIZE: u32 = 0x0150_E828;
pub const NVAPI_UNLOAD: u32 = 0xD22B_DD7E;
pub const NVAPI_SYS_GET_DRIVER_AND_BRANCH_VERSION: u32 = 0x2926_AAAD;
pub const NVAPI_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC_921F;
pub const NVAPI_GPU_GET_FULL_NAME: u32 = 0xCEEE_8E9F;
pub const NVAPI_GET_ASSOCIATED_NVIDIA_DISPLAY_HANDLE: u32 = 0x35C2_9134;
pub const NVAPI_GET_DVC_INFO_EX: u32 = 0x0E45_002D;
pub const NVAPI_SET_DVC_LEVEL_EX: u32 = 0x4A82_C2B1;
pub const NVAPI_GET_HUE_INFO: u32 = 0x95B6_4341;
pub const NVAPI_SET_HUE_ANGLE: u32 = 0xF5A0_F22C;

// DRS (Driver Settings — nvdrsdb.bin)
pub const NVAPI_DRS_CREATE_SESSION: u32 = 0x0694_D52E;
pub const NVAPI_DRS_DESTROY_SESSION: u32 = 0xDAD9_CFF8;
pub const NVAPI_DRS_LOAD_SETTINGS: u32 = 0x375D_BD6B;
pub const NVAPI_DRS_SAVE_SETTINGS: u32 = 0xFCBC_7E14;
pub const NVAPI_DRS_GET_BASE_PROFILE: u32 = 0xDA84_66A0;
pub const NVAPI_DRS_GET_SETTING: u32 = 0x73BF_8338;
pub const NVAPI_DRS_SET_SETTING: u32 = 0x577D_D202;
pub const NVAPI_DRS_RESTORE_ALL_DEFAULTS: u32 = 0x5927_B094;

// DRS setting IDs, each confirmed by NvAPI_DRS_GetSettingNameFromId on a live driver.
/// "Maximum pre-rendered frames" (PRERENDERLIMIT), what Control Panel's Low Latency Mode writes.
pub const DRS_PRERENDER_LIMIT: u32 = 0x007B_A09E;
/// "Frame Rate Limiter"
pub const DRS_MAX_FRAME_RATE: u32 = 0x1083_5002;
/// "Idle Application Max FPS Limit"
pub const DRS_IDLE_MAX_FRAME_RATE: u32 = 0x1083_5016;
/// "Vertical Sync"
pub const DRS_VSYNC_MODE: u32 = 0x00A8_79CF;
/// "Power management mode"
pub const DRS_POWER_MANAGEMENT: u32 = 0x1057_EB71;
/// "Anisotropic filtering mode" (0 = application, 1 = user level)
pub const DRS_ANISO_MODE: u32 = 0x10D2_BB16;
/// "Anisotropic filtering setting" (1 = off, 2/4/8/16x)
pub const DRS_ANISO_LEVEL: u32 = 0x101E_61A9;
/// "Enable FXAA"
pub const DRS_FXAA_ENABLE: u32 = 0x1074_C972;
/// "Texture filtering - Quality"
pub const DRS_TEX_QUALITY: u32 = 0x00CE_2691;
/// "Texture filtering - Negative LOD bias"
pub const DRS_TEX_NEG_LOD_BIAS: u32 = 0x0019_BB68;
/// "Texture filtering - Anisotropic sample optimization"
pub const DRS_TEX_ANISO_OPT: u32 = 0x00E7_3211;
/// "Texture filtering - Trilinear optimization" (0 = on, 1 = off)
pub const DRS_TEX_TRILINEAR_OPT: u32 = 0x002E_CAF2;
/// "Threaded optimization" (0 = auto, 1 = on, 2 = off)
pub const DRS_THREADED_OPT: u32 = 0x20C1_221E;
/// "Shader disk cache maximum size" in MB (0 = disabled, 0xFFFFFFFF = unlimited)
pub const DRS_SHADER_CACHE_SIZE: u32 = 0x00AC_8497;
/// "Override DLSS-SR presets" (A = 1 ... K = 11, 0x00FFFFFF = latest)
pub const DRS_DLSS_SR_PRESET: u32 = 0x10E4_1DF3;

pub type FnQueryInterface = unsafe extern "C" fn(u32) -> *const c_void;
pub type FnInitialize = unsafe extern "C" fn() -> NvApiStatus;
pub type FnUnload = unsafe extern "C" fn() -> NvApiStatus;
pub type FnGetDriverVersion = unsafe extern "C" fn(*mut u32, *mut u8) -> NvApiStatus;
pub type FnEnumPhysicalGpus =
    unsafe extern "C" fn(*mut NvPhysicalGpuHandle, *mut u32) -> NvApiStatus;
pub type FnGpuGetFullName = unsafe extern "C" fn(NvPhysicalGpuHandle, *mut u8) -> NvApiStatus;
/// `NvAPI_GetAssociatedNvidiaDisplayHandle("\\.\DISPLAYn", &handle)`
pub type FnGetAssociatedDisplayHandle =
    unsafe extern "C" fn(*const c_char, *mut NvDisplayHandle) -> NvApiStatus;

pub type FnDrsCreateSession = unsafe extern "C" fn(*mut NvDrsSessionHandle) -> NvApiStatus;
pub type FnDrsDestroySession = unsafe extern "C" fn(NvDrsSessionHandle) -> NvApiStatus;
pub type FnDrsLoadSettings = unsafe extern "C" fn(NvDrsSessionHandle) -> NvApiStatus;
pub type FnDrsSaveSettings = unsafe extern "C" fn(NvDrsSessionHandle) -> NvApiStatus;
pub type FnDrsGetBaseProfile =
    unsafe extern "C" fn(NvDrsSessionHandle, *mut NvDrsProfileHandle) -> NvApiStatus;
pub type FnDrsGetSetting = unsafe extern "C" fn(
    NvDrsSessionHandle,
    NvDrsProfileHandle,
    u32,
    *mut NvDrsSetting,
) -> NvApiStatus;
pub type FnDrsSetSetting = unsafe extern "C" fn(
    NvDrsSessionHandle,
    NvDrsProfileHandle,
    *mut NvDrsSetting,
) -> NvApiStatus;
pub type FnDrsRestoreAllDefaults = unsafe extern "C" fn(NvDrsSessionHandle) -> NvApiStatus;

pub const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;
pub const NVAPI_SHORT_STRING_MAX: usize = 64;
pub const NVAPI_UNICODE_STRING_MAX: usize = 2048;
pub const NVDRS_VALUE_UNION_SIZE: usize = 4 + 4096;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NvDisplayDvcInfoEx {
    pub version: u32,
    pub current_level: i32,
    pub min_level: i32,
    pub max_level: i32,
    pub default_level: i32,
}

impl Default for NvDisplayDvcInfoEx {
    fn default() -> Self {
        Self {
            version: (std::mem::size_of::<Self>() as u32) | (1 << 16),
            current_level: 50,
            min_level: 0,
            max_level: 100,
            default_level: 50,
        }
    }
}

pub type FnGetDvcInfoEx =
    unsafe extern "C" fn(NvDisplayHandle, u32, *mut NvDisplayDvcInfoEx) -> NvApiStatus;
pub type FnSetDvcLevelEx =
    unsafe extern "C" fn(NvDisplayHandle, u32, *mut NvDisplayDvcInfoEx) -> NvApiStatus;

/// `NVDRS_SETTING_V1` (12320 bytes). Each value union is sized by its largest member,
/// `NVDRS_BINARY_SETTING { u32 valueLength; u8 valueData[4096]; }` = 4100 bytes.
/// DWORD values live in the first 4 bytes of the union.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NvDrsSetting {
    pub version: u32,
    pub setting_name: [u16; NVAPI_UNICODE_STRING_MAX],
    pub setting_id: u32,
    pub setting_type: u32, // 0 = NVDRS_DWORD_TYPE
    pub setting_location: u32,
    pub is_current_predefined: u32,
    pub is_predefined_valid: u32,
    pub predefined_value: [u8; NVDRS_VALUE_UNION_SIZE],
    pub current_value: [u8; NVDRS_VALUE_UNION_SIZE],
}

impl NvDrsSetting {
    pub fn new_dword(setting_id: u32, value: u32) -> Self {
        let mut s = Self {
            version: (std::mem::size_of::<Self>() as u32) | (1 << 16),
            setting_name: [0u16; NVAPI_UNICODE_STRING_MAX],
            setting_id,
            setting_type: 0,
            setting_location: 0,
            is_current_predefined: 0,
            is_predefined_valid: 0,
            predefined_value: [0u8; NVDRS_VALUE_UNION_SIZE],
            current_value: [0u8; NVDRS_VALUE_UNION_SIZE],
        };
        s.current_value[..4].copy_from_slice(&value.to_le_bytes());
        s
    }

    pub fn current_dword(&self) -> u32 {
        u32::from_le_bytes([
            self.current_value[0],
            self.current_value[1],
            self.current_value[2],
            self.current_value[3],
        ])
    }
}

pub fn format_nv_driver_version(v: u32) -> String {
    format!("{}.{:02}", v / 100, v % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_nvapi_driver_version() {
        assert_eq!(format_nv_driver_version(57260), "572.60");
        assert_eq!(format_nv_driver_version(61692), "616.92");
    }

    #[test]
    fn drs_setting_matches_sdk_size() {
        // NVDRS_SETTING_VER1 = MAKE_NVAPI_VERSION(NVDRS_SETTING_V1, 1) = 0x0001_3020
        assert_eq!(std::mem::size_of::<NvDrsSetting>(), 0x3020);
        assert_eq!(NvDrsSetting::new_dword(0, 0).version, 0x0001_3020);
    }

    #[test]
    fn drs_setting_dword_round_trip() {
        let s = NvDrsSetting::new_dword(DRS_PRERENDER_LIMIT, 2);
        assert_eq!(s.setting_id, DRS_PRERENDER_LIMIT);
        assert_eq!(s.current_dword(), 2);
    }
}
