use std::ffi::c_void;

pub type CtlResult = u32;
pub const CTL_RESULT_SUCCESS: CtlResult = 0;

pub type CtlApiHandle = *mut c_void;
pub type CtlDeviceAdapterHandle = *mut c_void;
pub type CtlDisplayOutputHandle = *mut c_void;

pub const fn ctl_make_version(major: u32, minor: u32) -> u32 {
    (major << 16) | (minor & 0xFFFF)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlInitArgs {
    pub size: u32,
    pub version: u8,
    pub app_version: u32,
    pub flags: u32,
    pub supported_version: u32,
    pub application_uid: [u8; 16],
}

impl Default for CtlInitArgs {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            app_version: ctl_make_version(1, 0),
            flags: 0,
            supported_version: 0,
            application_uid: [0u8; 16],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CtlFirmwareVersion {
    pub major_version: u64,
    pub minor_version: u64,
    pub build_number: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CtlAdapterBdf {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlDeviceAdapterProperties {
    pub size: u32,
    pub version: u8,
    pub p_device_id: *mut c_void,
    pub device_id_size: u32,
    pub device_type: u32,
    pub supported_subfunction_flags: u32,
    pub driver_version: u64,
    pub firmware_version: CtlFirmwareVersion,
    pub pci_vendor_id: u32,
    pub pci_device_id: u32,
    pub rev_id: u32,
    pub num_eus_per_sub_slice: u32,
    pub num_sub_slices_per_slice: u32,
    pub num_slices: u32,
    pub name: [u8; 100],
    pub graphics_adapter_properties: u32,
    pub frequency: u32,
    pub pci_subsys_id: u16,
    pub pci_subsys_vendor_id: u16,
    pub adapter_bdf: CtlAdapterBdf,
    pub reserved: [u8; 100],
}

impl Default for CtlDeviceAdapterProperties {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            p_device_id: std::ptr::null_mut(),
            device_id_size: 0,
            device_type: 0,
            supported_subfunction_flags: 0,
            driver_version: 0,
            firmware_version: CtlFirmwareVersion::default(),
            pci_vendor_id: 0,
            pci_device_id: 0,
            rev_id: 0,
            num_eus_per_sub_slice: 0,
            num_sub_slices_per_slice: 0,
            num_slices: 0,
            name: [0u8; 100],
            graphics_adapter_properties: 0,
            frequency: 0,
            pci_subsys_id: 0,
            pci_subsys_vendor_id: 0,
            adapter_bdf: CtlAdapterBdf::default(),
            reserved: [0u8; 100],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlGetBrightness {
    pub size: u32,
    pub version: u8,
    pub target_brightness: u32,
    pub current_brightness: u32,
}

impl Default for CtlGetBrightness {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            target_brightness: 0,
            current_brightness: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlSetBrightness {
    pub size: u32,
    pub version: u8,
    pub smooth_transition_time_in_ms: u32,
    pub target_brightness: u32,
}

impl Default for CtlSetBrightness {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            smooth_transition_time_in_ms: 0,
            target_brightness: 50000,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlLaceConfig {
    pub size: u32,
    pub version: u8,
    pub enabled: bool,
    pub op_type_get: u32,
    pub op_type_set: u32,
    pub trigger: u32,
    pub aggressiveness_percent: u8,
}

impl Default for CtlLaceConfig {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            enabled: false,
            op_type_get: 0,
            op_type_set: 0,
            trigger: 0,
            aggressiveness_percent: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlPixtxMatrixConfig {
    pub size: u32,
    pub version: u8,
    pub _pad0: [u8; 3],
    pub pre_offsets: [f64; 3],
    pub post_offsets: [f64; 3],
    pub matrix: [[f64; 3]; 3],
}

impl Default for CtlPixtxMatrixConfig {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            _pad0: [0; 3],
            pre_offsets: [0.0; 3],
            post_offsets: [0.0; 3],
            matrix: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlPixtxBlockConfig {
    pub size: u32,
    pub version: u8,
    pub _pad0: [u8; 3],
    pub block_id: u32,
    pub block_type: u32,
    pub matrix_config: CtlPixtxMatrixConfig,
}

impl Default for CtlPixtxBlockConfig {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            _pad0: [0; 3],
            block_id: 0,
            block_type: 0,
            matrix_config: CtlPixtxMatrixConfig::default(),
        }
    }
}

#[repr(C)]
pub struct CtlPixtxPipeGetConfig {
    pub size: u32,
    pub version: u8,
    pub _pad0: [u8; 3],
    pub oper_type: u32,
    pub pixel_formats_pad: [u8; 244],
    pub num_blocks: u32,
    pub _pad1: u32,
    pub p_block_configs: *mut CtlPixtxBlockConfig,
}

impl Default for CtlPixtxPipeGetConfig {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            _pad0: [0; 3],
            oper_type: 0,
            pixel_formats_pad: [0; 244],
            num_blocks: 0,
            _pad1: 0,
            p_block_configs: std::ptr::null_mut(),
        }
    }
}

#[repr(C)]
pub struct CtlPixtxPipeSetConfig {
    pub size: u32,
    pub version: u8,
    pub _pad0: [u8; 3],
    pub oper_type: u32,
    pub flags: u32,
    pub num_blocks: u32,
    pub _pad1: u32,
    pub p_block_configs: *mut CtlPixtxBlockConfig,
}

impl Default for CtlPixtxPipeSetConfig {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            _pad0: [0; 3],
            oper_type: 0,
            flags: 0,
            num_blocks: 0,
            _pad1: 0,
            p_block_configs: std::ptr::null_mut(),
        }
    }
}

pub const CTL_PIXTX_BLOCK_TYPE_3X3_MATRIX: u32 = 3;
pub const CTL_PIXTX_BLOCK_TYPE_3X3_MATRIX_AND_OFFSETS: u32 = 4;
pub const CTL_PIXTX_CONFIG_QUERY_TYPE_CAPABILITY: u32 = 0;
pub const CTL_PIXTX_CONFIG_QUERY_TYPE_CURRENT: u32 = 1;
pub const CTL_PIXTX_CONFIG_OPER_TYPE_RESTORE_DEFAULT: u32 = 1;
pub const CTL_PIXTX_CONFIG_OPER_TYPE_SET_CUSTOM: u32 = 2;

/// Builds a 3x3 Color Space Conversion (CSC) matrix for Saturation (0.0..=100.0, neutral 50.0)
/// and Hue (-180.0..=180.0, neutral 0.0) using BT.709 luma weights.
pub fn build_csc_matrix(saturation_pct: f32, hue_deg: f32) -> [[f64; 3]; 3] {
    let s = (saturation_pct.clamp(0.0, 100.0) as f64) / 50.0;
    let theta = (hue_deg.clamp(-180.0, 180.0) as f64).to_radians();
    let cos_h = theta.cos() * s;
    let sin_h = theta.sin() * s;

    // BT.709 luma coefficients
    const WR: f64 = 0.2126;
    const WG: f64 = 0.7152;
    const WB: f64 = 0.0722;

    [
        [
            WR + (1.0 - WR) * cos_h - WR * sin_h,
            WG - WG * cos_h - WG * sin_h,
            WB - WB * cos_h + (1.0 - WB) * sin_h,
        ],
        [
            WR - WR * cos_h + 0.143 * sin_h,
            WG + (1.0 - WG) * cos_h + 0.140 * sin_h,
            WB - WB * cos_h - 0.283 * sin_h,
        ],
        [
            WR - WR * cos_h - (1.0 - WR) * sin_h,
            WG - WG * cos_h + WG * sin_h,
            WB + (1.0 - WB) * cos_h + WB * sin_h,
        ],
    ]
}

pub type FnCtlInit = unsafe extern "C" fn(*mut CtlInitArgs, *mut CtlApiHandle) -> CtlResult;
pub type FnCtlClose = unsafe extern "C" fn(CtlApiHandle) -> CtlResult;
pub type FnCtlEnumerateDevices =
    unsafe extern "C" fn(CtlApiHandle, *mut u32, *mut CtlDeviceAdapterHandle) -> CtlResult;
pub type FnCtlGetDeviceProperties = unsafe extern "C" fn(
    CtlDeviceAdapterHandle,
    *mut CtlDeviceAdapterProperties,
) -> CtlResult;
pub type FnCtlEnumerateDisplayOutputs = unsafe extern "C" fn(
    CtlDeviceAdapterHandle,
    *mut u32,
    *mut CtlDisplayOutputHandle,
) -> CtlResult;
/// `ctl_display_properties_t` (192 bytes). Only the header and the OS encoder id union are read;
/// on Windows the union's low 32 bits are the CCD `DISPLAYCONFIG_PATH_TARGET_INFO.id`.
#[repr(C)]
pub struct CtlDisplayProperties {
    pub size: u32,
    pub version: u8,
    pub os_display_encoder_id: u64,
    _rest: [u8; 176],
}

impl Default for CtlDisplayProperties {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            os_display_encoder_id: 0,
            _rest: [0u8; 176],
        }
    }
}

// Display scaling / sharpness, from igcl_api.h (bools are C `bool`, read as u8).
pub const CTL_SCALING_TYPE_FLAG_CENTERED: u32 = 1 << 1;
pub const CTL_SCALING_TYPE_FLAG_STRETCHED: u32 = 1 << 2;
pub const CTL_SCALING_TYPE_FLAG_ASPECT_RATIO_CENTERED_MAX: u32 = 1 << 3;
pub const CTL_RETRO_SCALING_TYPE_FLAG_INTEGER: u32 = 1 << 0;

/// `ctl_scaling_settings_t`; the Intel sample sets `Version = 1`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlScalingSettings {
    pub size: u32,
    pub version: u8,
    pub enable: u8,
    pub scaling_type: u32,
    pub custom_scaling_x: u32,
    pub custom_scaling_y: u32,
    pub hardware_mode_set: u8,
    pub preferred_scaling_type: u32,
}

impl Default for CtlScalingSettings {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 1,
            enable: 0,
            scaling_type: 0,
            custom_scaling_x: 0,
            custom_scaling_y: 0,
            hardware_mode_set: 0,
            preferred_scaling_type: 0,
        }
    }
}

/// `ctl_retro_scaling_settings_t` (adapter-wide integer / nearest-neighbour scaling).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlRetroScalingSettings {
    pub size: u32,
    pub version: u8,
    pub get: u8,
    pub enable: u8,
    pub retro_scaling_type: u32,
}

impl Default for CtlRetroScalingSettings {
    fn default() -> Self {
        Self { size: std::mem::size_of::<Self>() as u32, version: 0, get: 0, enable: 0, retro_scaling_type: 0 }
    }
}

/// `ctl_sharpness_caps_t`; `pFilterProperty` is left null, only `SupportedFilterFlags` is read.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlSharpnessCaps {
    pub size: u32,
    pub version: u8,
    pub supported_filter_flags: u32,
    pub num_filter_types: u8,
    pub p_filter_property: *mut c_void,
}

impl Default for CtlSharpnessCaps {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            version: 0,
            supported_filter_flags: 0,
            num_filter_types: 0,
            p_filter_property: std::ptr::null_mut(),
        }
    }
}

/// `ctl_sharpness_settings_t`; `Intensity` is 0..=100 in the current driver implementation.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CtlSharpnessSettings {
    pub size: u32,
    pub version: u8,
    pub enable: u8,
    pub filter_type: u32,
    pub intensity: f32,
}

impl Default for CtlSharpnessSettings {
    fn default() -> Self {
        Self { size: std::mem::size_of::<Self>() as u32, version: 0, enable: 0, filter_type: 0, intensity: 0.0 }
    }
}

pub type FnCtlScalingSettings =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlScalingSettings) -> CtlResult;
pub type FnCtlRetroScaling =
    unsafe extern "C" fn(CtlDeviceAdapterHandle, *mut CtlRetroScalingSettings) -> CtlResult;
pub type FnCtlSharpnessCaps =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlSharpnessCaps) -> CtlResult;
pub type FnCtlSharpnessSettings =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlSharpnessSettings) -> CtlResult;

pub type FnCtlGetDisplayProperties =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlDisplayProperties) -> CtlResult;
pub type FnCtlGetBrightnessSetting =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlGetBrightness) -> CtlResult;
pub type FnCtlSetBrightnessSetting =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlSetBrightness) -> CtlResult;
pub type FnCtlGetLaceConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlLaceConfig) -> CtlResult;
pub type FnCtlSetLaceConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlLaceConfig) -> CtlResult;
pub type FnCtlPixelTransformationGetConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlPixtxPipeGetConfig) -> CtlResult;
pub type FnCtlPixelTransformationSetConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlPixtxPipeSetConfig) -> CtlResult;

pub fn format_igcl_driver_version(v: u64) -> String {
    let part1 = (v >> 48) & 0xFFFF;
    let part2 = (v >> 32) & 0xFFFF;
    let part3 = (v >> 16) & 0xFFFF;
    let part4 = v & 0xFFFF;
    format!("{part1}.{part2}.{part3}.{part4}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_structs_match_igcl_layout() {
        assert_eq!(std::mem::size_of::<CtlScalingSettings>(), 28);
        assert_eq!(std::mem::offset_of!(CtlScalingSettings, preferred_scaling_type), 24);
        assert_eq!(std::mem::size_of::<CtlRetroScalingSettings>(), 12);
        assert_eq!(std::mem::size_of::<CtlSharpnessCaps>(), 24);
        assert_eq!(std::mem::size_of::<CtlSharpnessSettings>(), 16);
    }

    #[test]
    fn igcl_abi_struct_sizes_are_valid() {
        assert_eq!(std::mem::size_of::<CtlInitArgs>(), 36);
        assert_eq!(std::mem::size_of::<CtlGetBrightness>(), 16);
        assert_eq!(std::mem::size_of::<CtlSetBrightness>(), 16);
        assert_eq!(std::mem::size_of::<CtlPixtxMatrixConfig>(), 128);
        assert_eq!(std::mem::size_of::<CtlPixtxBlockConfig>(), 144);
        assert_eq!(std::mem::size_of::<CtlPixtxPipeGetConfig>(), 272);
        assert_eq!(std::mem::size_of::<CtlPixtxPipeSetConfig>(), 32);
        assert!(std::mem::size_of::<CtlDeviceAdapterProperties>() >= 260);
    }

    #[test]
    fn csc_matrix_is_identity_at_neutral_saturation_and_hue() {
        let m = build_csc_matrix(50.0, 0.0);
        for r in 0..3 {
            for c in 0..3 {
                let expected = if r == c { 1.0 } else { 0.0 };
                assert!((m[r][c] - expected).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn formats_packed_driver_version() {
        let packed = (32u64 << 48) | (0u64 << 32) | (101u64 << 16) | 6881u64;
        assert_eq!(format_igcl_driver_version(packed), "32.0.101.6881");
    }
}
