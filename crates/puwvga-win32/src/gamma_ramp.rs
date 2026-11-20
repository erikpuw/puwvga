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

/// Decodes (brightness, contrast, gamma) from a 3x256 hardware gamma ramp using interior points
/// (i = 64 and i = 191, whose midpoint is x = 0.5) so endpoint clipping at 0 and 255 does not skew results.
pub fn decode_gamma_ramp(ramp: &[u16; 768]) -> (f32, f32, f32) {
    let x1 = 64.0_f32 / 255.0;
    let x3 = 191.0_f32 / 255.0;
    let y1 = (ramp[256 + 64] as f32) / 65535.0;
    let y3 = (ramp[256 + 191] as f32) / 65535.0;

    let mut best = (50.0_f32, 50.0_f32, 1.0_f32);
    let mut best_err = f64::MAX;
    let sample_indices = [48usize, 64, 96, 128, 160, 191, 207];

    for step_idx in 10..=50 {
        let g = (step_idx as f32) * 0.05;
        let l1 = y1.max(1e-4).powf(g);
        let l3 = y3.max(1e-4).powf(g);

        let c_scale = ((l3 - l1) / (x3 - x1)).clamp(0.2, 2.0);
        // Because (64 + 191) / 255 = 1.0, (x1 + x3) / 2 == 0.5 exactly!
        let l_mid = 0.5 * (l1 + l3);
        let b_offset = (l_mid - 0.5).clamp(-0.5, 0.5);

        let brightness = (50.0 + b_offset * 100.0).clamp(0.0, 100.0).round();
        let contrast = (c_scale * 50.0).clamp(10.0, 100.0).round();

        let candidate = build_gamma_ramp(brightness, contrast, g);
        let err: f64 = sample_indices
            .iter()
            .map(|&idx| {
                let d = (ramp[256 + idx] as f64) - (candidate[256 + idx] as f64);
                d * d
            })
            .sum();

        if err < best_err {
            best_err = err;
            best = (brightness, contrast, (g * 20.0).round() / 20.0);
        }
    }

    best
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

    Some(decode_gamma_ramp(&ramp))
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
        assert_eq!(decode_gamma_ramp(&ramp), (50.0, 50.0, 1.0));
    }

    #[test]
    fn round_trips_brightness_45_contrast_55_even_with_clipped_endpoints() {
        let ramp = build_gamma_ramp(45.0, 55.0, 1.0);
        let (b, c, g) = decode_gamma_ramp(&ramp);
        assert_eq!(b, 45.0);
        assert_eq!(c, 55.0);
        assert!((g - 1.0).abs() < 1e-3);

        let ramp2 = build_gamma_ramp(45.0, 55.0, 1.20);
        let (b2, c2, g2) = decode_gamma_ramp(&ramp2);
        assert_eq!(b2, 45.0);
        assert_eq!(c2, 55.0);
        assert!((g2 - 1.20).abs() < 1e-3);
    }
}
