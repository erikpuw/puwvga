use puwvga_core::BackendError;
use std::ffi::c_void;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC};

#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetDeviceGammaRamp(hdc: *mut c_void, lp_ramp: *mut c_void) -> i32;
    fn SetDeviceGammaRamp(hdc: *mut c_void, lp_ramp: *const c_void) -> i32;
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Builds a 3x256 RGB hardware gamma LUT from brightness (0..100, default 50),
/// contrast (0..100, default 50), and gamma (0.5..2.5, default 1.0).
pub fn build_gamma_ramp(brightness: f32, contrast: f32, gamma: f32) -> [u16; 768] {
    let mut ramp = [0u16; 768];
    let c_scale = (contrast.clamp(10.0, 100.0)) / 50.0;
    let b_offset = (brightness.clamp(0.0, 100.0) - 50.0) / 100.0;
    let inv_gamma = 1.0 / gamma.clamp(0.5, 2.5);

    for i in 0..256 {
        let x = (i as f32) / 255.0;
        let linear = ((x - 0.5) * c_scale + 0.5 + b_offset).clamp(0.0, 1.0);
        let curved = linear.powf(inv_gamma).clamp(0.0, 1.0);
        let val = (curved * 65535.0).round() as u16;
        ramp[i] = val;
        ramp[256 + i] = val;
        ramp[512 + i] = val;
    }
    ramp
}

/// Applies Brightness, Contrast, and Gamma immediately to the physical display hardware LUT
/// via Win32 `SetDeviceGammaRamp` (works instantaneously on Intel, NVIDIA, and AMD displays).
pub fn apply_gamma_ramp(
    device_name: &str,
    brightness: f32,
    contrast: f32,
    gamma: f32,
) -> Result<(), BackendError> {
    let driver = to_wide_null("DISPLAY");
    let dev = to_wide_null(device_name);
    let hdc = unsafe { CreateDCW(PCWSTR(driver.as_ptr()), PCWSTR(dev.as_ptr()), None, None) };
    if hdc.is_invalid() {
        return Err(BackendError::DriverError {
            code: -1,
            message: format!("CreateDCW failed for {device_name}"),
        });
    }

    let ramp = build_gamma_ramp(brightness, contrast, gamma);
    let ok = unsafe { SetDeviceGammaRamp(hdc.0, ramp.as_ptr() as *const c_void) };
    unsafe {
        let _ = DeleteDC(hdc);
    }

    if ok != 0 {
        Ok(())
    } else {
        Err(BackendError::DriverError {
            code: -2,
            message: format!(
                "SetDeviceGammaRamp rejected extreme range for {device_name} (Windows gamma guard)"
            ),
        })
    }
}

/// Reads the current hardware gamma LUT from the display and estimates (brightness, contrast, gamma).
pub fn read_gamma_ramp_estimate(device_name: &str) -> Option<(f32, f32, f32)> {
    let driver = to_wide_null("DISPLAY");
    let dev = to_wide_null(device_name);
    let hdc = unsafe { CreateDCW(PCWSTR(driver.as_ptr()), PCWSTR(dev.as_ptr()), None, None) };
    if hdc.is_invalid() {
        return None;
    }

    let mut ramp = [0u16; 768];
    let ok = unsafe { GetDeviceGammaRamp(hdc.0, ramp.as_mut_ptr() as *mut c_void) };
    unsafe {
        let _ = DeleteDC(hdc);
    }
    if ok == 0 {
        return None;
    }

    // Sample green channel at 0%, 50%, 100%
    let g0 = (ramp[256] as f32) / 65535.0;
    let g50 = (ramp[256 + 128] as f32) / 65535.0;
    let g100 = (ramp[256 + 255] as f32) / 65535.0;

    let contrast = ((g100 - g0) * 50.0).clamp(10.0, 100.0).round();
    let brightness = (50.0 + ((g0 + g100) * 0.5 - 0.5) * 100.0)
        .clamp(0.0, 100.0)
        .round();
    let mid_norm = if (g100 - g0).abs() > 0.05 {
        ((g50 - g0) / (g100 - g0)).clamp(0.05, 0.95)
    } else {
        0.5
    };
    let gamma = (0.5_f32.ln() / mid_norm.ln()).clamp(0.5, 2.5);
    let gamma_rounded = (gamma * 20.0).round() / 20.0;

    Some((brightness, contrast, gamma_rounded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_gamma_ramp_is_linear() {
        let ramp = build_gamma_ramp(50.0, 50.0, 1.0);
        assert_eq!(ramp[0], 0);
        assert_eq!(ramp[255], 65535);
        assert!((ramp[128] as i32 - 32896).abs() <= 256);
    }
}
