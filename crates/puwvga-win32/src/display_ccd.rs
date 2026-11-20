use puwvga_core::{BackendError, DisplayOutputInfo};
use std::collections::BTreeSet;
use std::mem::size_of;
use windows::core::PCWSTR;
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, LUID, WIN32_ERROR};
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplaySettingsExW, CDS_UPDATEREGISTRY, DEVMODEW,
    DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
    ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS,
};

fn wide_to_string(slice: &[u16]) -> String {
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end]).trim().to_string()
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Enumerates all active displays using Win32 CCD API, mapping each display to its owning GPU LUID.
pub fn enumerate_active_displays() -> Result<Vec<DisplayOutputInfo>, BackendError> {
    let mut path_count = 0u32;
    let mut mode_count = 0u32;

    let err = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    };
    if err != ERROR_SUCCESS {
        return Err(BackendError::DriverError {
            code: err.0 as i32,
            message: format!("GetDisplayConfigBufferSizes failed: {:?}", err),
        });
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    let err = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    };
    if err != ERROR_SUCCESS {
        return Err(BackendError::DriverError {
            code: err.0 as i32,
            message: format!("QueryDisplayConfig failed: {:?}", err),
        });
    }
    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);

    let mut displays = Vec::new();

    for (idx, path) in paths.iter().enumerate() {
        let adapter_luid: LUID = path.sourceInfo.adapterId;

        // 1. Source GDI device name (e.g. \\.\DISPLAY1)
        let mut src_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: adapter_luid,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };
        let device_name =
            if unsafe { DisplayConfigGetDeviceInfo(&mut src_name.header) } == ERROR_SUCCESS.0 as i32
            {
                wide_to_string(&src_name.viewGdiDeviceName)
            } else {
                format!("\\\\.\\DISPLAY{}", idx + 1)
            };

        // 2. Target friendly monitor name
        let mut tgt_name = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let friendly_name =
            if unsafe { DisplayConfigGetDeviceInfo(&mut tgt_name.header) } == ERROR_SUCCESS.0 as i32
            {
                let s = wide_to_string(&tgt_name.monitorFriendlyDeviceName);
                if s.is_empty() {
                    format!("Display {}", idx + 1)
                } else {
                    s
                }
            } else {
                format!("Display {}", idx + 1)
            };

        // 3. Advanced color info (bits per color + Windows HDR status)
        let mut color_info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                size: size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let (bits_per_color, hdr_active) =
            if unsafe { DisplayConfigGetDeviceInfo(&mut color_info.header) }
                == ERROR_SUCCESS.0 as i32
            {
                let bpc = if color_info.bitsPerColorChannel > 0 {
                    color_info.bitsPerColorChannel
                } else {
                    8
                };
                // Bit 1 of Anonymous.value is advancedColorEnabled (Windows HDR active)
                let flags = unsafe { color_info.Anonymous.value };
                let hdr = (flags & 0x2) != 0;
                (bpc, hdr)
            } else {
                (8, false)
            };

        // 4. Current resolution & refresh rate
        let mut width = 0u32;
        let mut height = 0u32;
        let mut is_primary = false;

        let src_mode_idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx } as usize;
        if let Some(m) = modes.get(src_mode_idx) {
            if m.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                let sm = unsafe { m.Anonymous.sourceMode };
                width = sm.width;
                height = sm.height;
                is_primary = sm.position.x == 0 && sm.position.y == 0;
            }
        }

        let refresh_hz = if path.targetInfo.refreshRate.Denominator > 0 {
            ((path.targetInfo.refreshRate.Numerator as f64)
                / (path.targetInfo.refreshRate.Denominator as f64))
                .round() as u32
        } else {
            query_current_hz(&device_name).unwrap_or(60)
        };

        displays.push(DisplayOutputInfo {
            id: format!(
                "{:08X}:{:08X}:{}",
                adapter_luid.HighPart, adapter_luid.LowPart, path.targetInfo.id
            ),
            device_name,
            friendly_name,
            adapter_luid_low: adapter_luid.LowPart,
            adapter_luid_high: adapter_luid.HighPart,
            width,
            height,
            refresh_hz,
            bits_per_color,
            hdr_active,
            is_primary,
        });
    }

    Ok(displays)
}

fn query_current_hz(device_name: &str) -> Option<u32> {
    let wide = to_wide_null(device_name);
    let mut dm = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    let ok = unsafe {
        EnumDisplaySettingsExW(
            PCWSTR(wide.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut dm,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
    };
    if ok.as_bool() && dm.dmDisplayFrequency > 1 {
        Some(dm.dmDisplayFrequency)
    } else {
        None
    }
}

/// Lists deduplicated, descending-sorted `(width, height, refresh_hz)` display modes.
pub fn list_supported_modes(device_name: &str) -> Vec<(u32, u32, u32)> {
    let wide = to_wide_null(device_name);
    let mut raw = Vec::new();
    let mut mode_num = 0u32;

    loop {
        let mut dm = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let ok = unsafe {
            EnumDisplaySettingsExW(
                PCWSTR(wide.as_ptr()),
                windows::Win32::Graphics::Gdi::ENUM_DISPLAY_SETTINGS_MODE(mode_num),
                &mut dm,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
        };
        if !ok.as_bool() {
            break;
        }
        mode_num += 1;
        if dm.dmBitsPerPel >= 32 && dm.dmPelsWidth >= 800 && dm.dmPelsHeight >= 600 {
            raw.push((dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency));
        }
    }

    dedup_and_sort_modes(raw)
}

pub fn dedup_and_sort_modes(modes: Vec<(u32, u32, u32)>) -> Vec<(u32, u32, u32)> {
    let set: BTreeSet<(u32, u32, u32)> = modes.into_iter().collect();
    let mut out: Vec<(u32, u32, u32)> = set.into_iter().collect();
    out.sort_by(|a, b| {
        (b.0 as u64 * b.1 as u64)
            .cmp(&(a.0 as u64 * a.1 as u64))
            .then_with(|| b.0.cmp(&a.0))
            .then_with(|| b.2.cmp(&a.2))
    });
    out
}

/// Applies resolution and refresh rate to a display via Win32 ChangeDisplaySettingsExW.
pub fn apply_display_mode(
    device_name: &str,
    width: u32,
    height: u32,
    hz: u32,
) -> Result<(), BackendError> {
    let wide = to_wide_null(device_name);
    let mut dm = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        dmFields: DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY,
        dmPelsWidth: width,
        dmPelsHeight: height,
        dmDisplayFrequency: hz,
        ..Default::default()
    };

    let status = unsafe {
        ChangeDisplaySettingsExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut dm),
            None,
            CDS_UPDATEREGISTRY,
            None,
        )
    };

    if status == DISP_CHANGE_SUCCESSFUL {
        Ok(())
    } else {
        Err(BackendError::DriverError {
            code: status.0,
            message: format!(
                "ChangeDisplaySettingsExW({device_name}, {width}x{height}@{hz}Hz) returned {}",
                WIN32_ERROR(status.0 as u32).0
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_and_sort_orders_highest_resolution_and_hz_first() {
        let input = vec![
            (1920, 1080, 60),
            (2560, 1440, 60),
            (2560, 1440, 165),
            (1920, 1080, 60),
            (1920, 1080, 144),
        ];
        assert_eq!(
            dedup_and_sort_modes(input),
            vec![
                (2560, 1440, 165),
                (2560, 1440, 60),
                (1920, 1080, 144),
                (1920, 1080, 60),
            ]
        );
    }

    #[test]
    fn smoke_enumerate_active_displays() {
        let displays = enumerate_active_displays().expect("CCD display query should succeed");
        assert!(!displays.is_empty(), "expected at least 1 active display");
        for d in &displays {
            assert!(d.width > 0 && d.height > 0);
            assert!(d.refresh_hz > 0);
        }
    }
}
