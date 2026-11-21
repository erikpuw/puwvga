//! ADL2 ABI, transcribed from AMD's display-library SDK (`include/adl_structures.h`,
//! `include/adl_defines.h`) and the prototypes used by its `ColorCaps` / `AMDDisplayTool` samples.
use std::ffi::c_void;

pub const ADL_OK: i32 = 0;
pub const ADL_MAX_PATH: usize = 256;

pub const ADL_DISPLAY_COLOR_SATURATION: i32 = 1 << 2;
pub const ADL_DISPLAY_COLOR_HUE: i32 = 1 << 3;
pub const ADL_DISPLAY_COLOR_TEMPERATURE: i32 = 1 << 4;

pub const ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED: i32 = 0x0000_0001;
pub const ADL_DISPLAY_DISPLAYINFO_DISPLAYMAPPED: i32 = 0x0000_0002;

pub type AdlContextHandle = *mut c_void;
pub type FnAdlMalloc = unsafe extern "C" fn(i32) -> *mut c_void;

/// `AdapterInfo` (Windows layout, 1572 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AdapterInfo {
    pub size: i32,
    pub adapter_index: i32,
    pub udid: [u8; ADL_MAX_PATH],
    pub bus_number: i32,
    pub device_number: i32,
    pub function_number: i32,
    pub vendor_id: i32,
    pub adapter_name: [u8; ADL_MAX_PATH],
    /// GDI source name, e.g. `\\.\DISPLAY1`.
    pub display_name: [u8; ADL_MAX_PATH],
    pub present: i32,
    pub exist: i32,
    pub driver_path: [u8; ADL_MAX_PATH],
    pub driver_path_ext: [u8; ADL_MAX_PATH],
    pub pnp_string: [u8; ADL_MAX_PATH],
    pub os_display_index: i32,
}

impl Default for AdapterInfo {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as i32,
            adapter_index: 0,
            udid: [0; ADL_MAX_PATH],
            bus_number: 0,
            device_number: 0,
            function_number: 0,
            vendor_id: 0,
            adapter_name: [0; ADL_MAX_PATH],
            display_name: [0; ADL_MAX_PATH],
            present: 0,
            exist: 0,
            driver_path: [0; ADL_MAX_PATH],
            driver_path_ext: [0; ADL_MAX_PATH],
            pnp_string: [0; ADL_MAX_PATH],
            os_display_index: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AdlDisplayId {
    pub logical_index: i32,
    pub physical_index: i32,
    pub logical_adapter_index: i32,
    pub physical_adapter_index: i32,
}

/// `ADLDisplayInfo` (552 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AdlDisplayInfo {
    pub display_id: AdlDisplayId,
    pub controller_index: i32,
    pub display_name: [u8; ADL_MAX_PATH],
    pub manufacturer_name: [u8; ADL_MAX_PATH],
    pub display_type: i32,
    pub output_type: i32,
    pub connector: i32,
    pub info_mask: i32,
    pub info_value: i32,
}

pub type FnAdl2MainControlCreate =
    unsafe extern "C" fn(FnAdlMalloc, i32, *mut AdlContextHandle) -> i32;
pub type FnAdl2MainControlDestroy = unsafe extern "C" fn(AdlContextHandle) -> i32;
pub type FnAdl2AdapterNumberOfAdaptersGet =
    unsafe extern "C" fn(AdlContextHandle, *mut i32) -> i32;
pub type FnAdl2AdapterInfoGet = unsafe extern "C" fn(AdlContextHandle, *mut AdapterInfo, i32) -> i32;
/// `(ctx, adapter, *num_displays, **info, force_detect)`; `*info` comes from our malloc callback.
pub type FnAdl2DisplayInfoGet =
    unsafe extern "C" fn(AdlContextHandle, i32, *mut i32, *mut *mut AdlDisplayInfo, i32) -> i32;
/// `(ctx, adapter, display, *caps, *valid_bits)`
pub type FnAdl2DisplayColorCapsGet =
    unsafe extern "C" fn(AdlContextHandle, i32, i32, *mut i32, *mut i32) -> i32;
/// `(ctx, adapter, display, color_type, *current, *default, *min, *max, *step)`
pub type FnAdl2DisplayColorGet = unsafe extern "C" fn(
    AdlContextHandle,
    i32,
    i32,
    i32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
) -> i32;
/// `(ctx, adapter, display, color_type, current)`
pub type FnAdl2DisplayColorSet = unsafe extern "C" fn(AdlContextHandle, i32, i32, i32, i32) -> i32;

const ALLOC_HEADER: usize = 16;

/// ADL's malloc callback. Stores the size in a header so [`adl_free`] can rebuild the layout.
///
/// # Safety
/// Only meant to be handed to ADL as `ADL_MAIN_MALLOC_CALLBACK`; free the result with [`adl_free`].
pub unsafe extern "C" fn adl_malloc(size: i32) -> *mut c_void {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    let Ok(layout) = std::alloc::Layout::from_size_align(size as usize + ALLOC_HEADER, 16) else {
        return std::ptr::null_mut();
    };
    unsafe {
        let base = std::alloc::alloc_zeroed(layout);
        if base.is_null() {
            return std::ptr::null_mut();
        }
        (base as *mut usize).write(layout.size());
        base.add(ALLOC_HEADER) as *mut c_void
    }
}

/// Frees memory ADL obtained through [`adl_malloc`].
///
/// # Safety
/// `ptr` must be null or a pointer returned by [`adl_malloc`] that has not been freed.
pub unsafe fn adl_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = (ptr as *mut u8).sub(ALLOC_HEADER);
        let total = (base as *const usize).read();
        std::alloc::dealloc(base, std::alloc::Layout::from_size_align_unchecked(total, 16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structs_match_sdk_layout() {
        assert_eq!(std::mem::size_of::<AdapterInfo>(), 1572);
        assert_eq!(std::mem::size_of::<AdlDisplayInfo>(), 552);
        assert_eq!(std::mem::offset_of!(AdapterInfo, display_name), 4 * 2 + 256 + 4 * 4 + 256);
        assert_eq!(std::mem::offset_of!(AdlDisplayInfo, info_value), 548);
    }

    #[test]
    fn malloc_free_round_trip() {
        unsafe {
            let p = adl_malloc(552 * 3) as *mut u8;
            assert!(!p.is_null());
            assert_eq!(*p.add(552 * 3 - 1), 0);
            adl_free(p as *mut c_void);
            assert!(adl_malloc(0).is_null());
        }
    }
}
