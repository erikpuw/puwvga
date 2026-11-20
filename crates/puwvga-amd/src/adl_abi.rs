use std::ffi::c_void;

pub const ADL_OK: i32 = 0;

pub type AdlContextHandle = *mut c_void;
pub type FnAdlMalloc = unsafe extern "C" fn(i32) -> *mut c_void;

pub type FnAdl2MainControlCreate =
    unsafe extern "C" fn(FnAdlMalloc, i32, *mut AdlContextHandle) -> i32;
pub type FnAdl2MainControlDestroy = unsafe extern "C" fn(AdlContextHandle) -> i32;
pub type FnAdl2AdapterNumberOfAdaptersGet =
    unsafe extern "C" fn(AdlContextHandle, *mut i32) -> i32;

pub unsafe extern "C" fn adl_malloc(size: i32) -> *mut c_void {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
    unsafe { std::alloc::alloc_zeroed(layout) as *mut c_void }
}
