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
pub type FnCtlGetBrightnessSetting =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlGetBrightness) -> CtlResult;
pub type FnCtlSetBrightnessSetting =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlSetBrightness) -> CtlResult;
pub type FnCtlGetLaceConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlLaceConfig) -> CtlResult;
pub type FnCtlSetLaceConfig =
    unsafe extern "C" fn(CtlDisplayOutputHandle, *mut CtlLaceConfig) -> CtlResult;

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
    fn igcl_abi_struct_sizes_are_valid() {
        assert_eq!(std::mem::size_of::<CtlInitArgs>(), 36);
        assert_eq!(std::mem::size_of::<CtlGetBrightness>(), 16);
        assert_eq!(std::mem::size_of::<CtlSetBrightness>(), 16);
        assert!(std::mem::size_of::<CtlDeviceAdapterProperties>() >= 260);
    }

    #[test]
    fn formats_packed_driver_version() {
        let packed = (32u64 << 48) | (0u64 << 32) | (101u64 << 16) | 6881u64;
        assert_eq!(format_igcl_driver_version(packed), "32.0.101.6881");
    }
}
