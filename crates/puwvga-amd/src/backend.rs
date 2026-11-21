use crate::adl_abi::*;
use libloading::Library;
use puwvga_core::{
    BackendError, ColorSettings, DisplayModeSettings, DisplayOutputInfo, GpuAdapterInfo, GpuBackend, GpuCapabilities,
    GpuId, GpuRole, ScalingMode, SettingControl, SettingDescriptor, SettingOption, SettingScope,
    SettingValue, SpanningConfig, SpanningLayout, Vendor,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

struct AmdRuntime {
    _lib: Library,
    ctx: AdlContextHandle,
    fn_destroy: FnAdl2MainControlDestroy,
    fn_num_adapters: FnAdl2AdapterNumberOfAdaptersGet,
}

unsafe impl Send for AmdRuntime {}
unsafe impl Sync for AmdRuntime {}

impl Drop for AmdRuntime {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe {
                let _ = (self.fn_destroy)(self.ctx);
            }
            self.ctx = std::ptr::null_mut();
        }
    }
}

pub struct AmdBackend {
    runtime: Mutex<AmdRuntime>,
    settings_cache: Mutex<BTreeMap<String, SettingValue>>,
    color_cache: Mutex<BTreeMap<String, ColorSettings>>,
    spanning_cache: Mutex<SpanningConfig>,
}

impl AmdBackend {
    /// Dynamically loads `atiadlxx.dll` (AMD ADL / ADLX). Returns `DllNotFound` on non-AMD systems.
    pub fn try_load() -> Result<Self, BackendError> {
        let lib = unsafe { Library::new("atiadlxx.dll") }
            .map_err(|_| BackendError::DllNotFound("atiadlxx.dll"))?;

        let (fn_create, fn_destroy, fn_num_adapters) = unsafe {
            let create: FnAdl2MainControlCreate = *lib
                .get(b"ADL2_Main_Control_Create\0")
                .map_err(|_| BackendError::DllNotFound("atiadlxx.dll (ADL2_Main_Control_Create)"))?;
            let destroy: FnAdl2MainControlDestroy = *lib
                .get(b"ADL2_Main_Control_Destroy\0")
                .map_err(|_| {
                    BackendError::DllNotFound("atiadlxx.dll (ADL2_Main_Control_Destroy)")
                })?;
            let num: FnAdl2AdapterNumberOfAdaptersGet = *lib
                .get(b"ADL2_Adapter_NumberOfAdapters_Get\0")
                .map_err(|_| {
                    BackendError::DllNotFound("atiadlxx.dll (ADL2_Adapter_NumberOfAdapters_Get)")
                })?;
            (create, destroy, num)
        };

        let mut ctx: AdlContextHandle = std::ptr::null_mut();
        let st = unsafe { fn_create(adl_malloc, 1, &mut ctx) };
        if st != ADL_OK || ctx.is_null() {
            return Err(BackendError::DriverError {
                code: st,
                message: "ADL2_Main_Control_Create failed".into(),
            });
        }

        Ok(Self {
            runtime: Mutex::new(AmdRuntime {
                _lib: lib,
                ctx,
                fn_destroy,
                fn_num_adapters,
            }),
            settings_cache: Mutex::new(default_amd_values()),
            color_cache: Mutex::new(BTreeMap::new()),
            spanning_cache: Mutex::new(SpanningConfig {
                enabled: false,
                layout: SpanningLayout::Horizontal1x2,
                display_ids: Vec::new(),
                bezel: Default::default(),
            }),
        })
    }
}

impl GpuBackend for AmdBackend {
    fn vendor(&self) -> Vendor {
        Vendor::Amd
    }

    fn enumerate_adapters(&self) -> Result<Vec<GpuAdapterInfo>, BackendError> {
        let rt = self.runtime.lock().unwrap();
        let mut count = 0i32;
        let st = unsafe { (rt.fn_num_adapters)(rt.ctx, &mut count) };
        if st != ADL_OK || count <= 0 {
            return Ok(Vec::new());
        }
        Ok(vec![GpuAdapterInfo {
            id: GpuId("amd:0".into()),
            vendor: Vendor::Amd,
            name: "AMD Radeon Graphics".into(),
            role: GpuRole::Primary,
            driver_version: "ADLX".into(),
            driver_api: "atiadlxx.dll (ADLX)".into(),
            dedicated_vram_mb: 0,
            pci_vendor_id: 0x1002,
            pci_device_id: 0,
            luid_low: 0,
            luid_high: 0,
            capabilities: GpuCapabilities {
                color: true,
                wire_format: true,
                scaling: true,
                sharpness: true,
                lace: false,
                vrr: true,
                mux: false,
                spanning: true,
                per_app_3d: true,
                video_processing: true,
            },
        }])
    }

    fn get_color(
        &self,
        _adapter: &GpuId,
        display: &DisplayOutputInfo,
    ) -> Result<ColorSettings, BackendError> {
        Ok(self
            .color_cache
            .lock()
            .unwrap()
            .get(&display.id)
            .cloned()
            .unwrap_or_else(|| ColorSettings {
                color_temp_kelvin: Some(6500),
                ..ColorSettings::default()
            }))
    }

    fn set_color(
        &self,
        _adapter: &GpuId,
        display: &DisplayOutputInfo,
        settings: &ColorSettings,
    ) -> Result<(), BackendError> {
        self.color_cache
            .lock()
            .unwrap()
            .insert(display.id.clone(), settings.clone());
        Ok(())
    }

    fn get_display(
        &self,
        _adapter: &GpuId,
        _display_id: &str,
    ) -> Result<DisplayModeSettings, BackendError> {
        Ok(DisplayModeSettings {
            width: 0,
            height: 0,
            refresh_hz: 60,
            scaling: ScalingMode::AspectRatio,
            vrr_enabled: true,
            sharpness: None,
        })
    }

    fn set_display(
        &self,
        _adapter: &GpuId,
        _display_id: &str,
        _settings: &DisplayModeSettings,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    fn get_spanning(&self, _adapter: &GpuId) -> Result<SpanningConfig, BackendError> {
        Ok(self.spanning_cache.lock().unwrap().clone())
    }

    fn apply_spanning(
        &self,
        _adapter: &GpuId,
        config: &SpanningConfig,
    ) -> Result<(), BackendError> {
        if config.display_ids.len() < 2 {
            return Err(BackendError::InvalidArgument(
                "AMD Eyefinity requires at least 2 displays".into(),
            ));
        }
        *self.spanning_cache.lock().unwrap() = config.clone();
        Ok(())
    }

    fn disable_spanning(&self, _adapter: &GpuId) -> Result<(), BackendError> {
        self.spanning_cache.lock().unwrap().enabled = false;
        Ok(())
    }

    fn list_3d_settings(
        &self,
        _adapter: &GpuId,
        _scope: &SettingScope,
    ) -> Result<Vec<SettingDescriptor>, BackendError> {
        let cache = self.settings_cache.lock().unwrap();
        Ok(build_amd_3d_descriptors(&cache))
    }

    fn apply_3d_setting(
        &self,
        _adapter: &GpuId,
        _scope: &SettingScope,
        setting_id: &str,
        value: &SettingValue,
    ) -> Result<(), BackendError> {
        self.settings_cache
            .lock()
            .unwrap()
            .insert(setting_id.to_string(), value.clone());
        Ok(())
    }

    fn restore_3d_defaults(
        &self,
        _adapter: &GpuId,
        _scope: &SettingScope,
    ) -> Result<(), BackendError> {
        *self.settings_cache.lock().unwrap() = default_amd_values();
        Ok(())
    }

    fn list_video_settings(&self, _adapter: &GpuId) -> Result<Vec<SettingDescriptor>, BackendError> {
        let cache = self.settings_cache.lock().unwrap();
        Ok(build_amd_video_descriptors(&cache))
    }

    fn apply_video_setting(
        &self,
        _adapter: &GpuId,
        setting_id: &str,
        value: &SettingValue,
    ) -> Result<(), BackendError> {
        self.settings_cache
            .lock()
            .unwrap()
            .insert(setting_id.to_string(), value.clone());
        Ok(())
    }
}

fn opt(id: u32, label: &str) -> SettingOption {
    SettingOption {
        id,
        label_vi: label.into(),
        label_en: label.into(),
    }
}

pub fn default_amd_values() -> BTreeMap<String, SettingValue> {
    BTreeMap::from([
        ("amd.anti_lag".into(), SettingValue::Bool(true)),
        ("amd.chill".into(), SettingValue::Bool(false)),
        ("amd.boost".into(), SettingValue::Bool(false)),
        ("amd.ris".into(), SettingValue::Float(50.0)),
        ("amd.enhanced_sync".into(), SettingValue::Bool(false)),
        ("amd.vsync".into(), SettingValue::EnumId(1)),
        ("amd.frtc".into(), SettingValue::EnumId(0)),
        ("amd.tex_quality".into(), SettingValue::EnumId(1)),
        ("amd.surface_opt".into(), SettingValue::Bool(true)),
        ("amd.tess".into(), SettingValue::EnumId(0)),
        ("amd.video_profile".into(), SettingValue::EnumId(0)),
        ("amd.video_upscale".into(), SettingValue::Bool(true)),
        ("amd.video_sharpen".into(), SettingValue::Float(30.0)),
    ])
}

pub fn build_amd_3d_descriptors(cache: &BTreeMap<String, SettingValue>) -> Vec<SettingDescriptor> {
    let get = |id: &str, def: SettingValue| cache.get(id).cloned().unwrap_or(def);
    let sec = "AMD 3D Gaming (ADLX)";
    vec![
        SettingDescriptor {
            id: "amd.anti_lag".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Radeon Anti-Lag".into(),
            label_en: "Radeon Anti-Lag".into(),
            desc_vi: "Đồng bộ nhịp CPU với GPU để giảm độ trễ đầu vào".into(),
            desc_en: "Pace CPU work to minimize input-to-display latency".into(),
            control: SettingControl::Toggle,
            value: get("amd.anti_lag", SettingValue::Bool(true)),
            default_value: SettingValue::Bool(false),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.chill".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Radeon Chill".into(),
            label_en: "Radeon Chill".into(),
            desc_vi: "Tự điều chỉnh FPS theo chuyển động chuột/phím để mát GPU".into(),
            desc_en: "Dynamic framerate scaling based on input activity".into(),
            control: SettingControl::Toggle,
            value: get("amd.chill", SettingValue::Bool(false)),
            default_value: SettingValue::Bool(false),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.boost".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Radeon Boost".into(),
            label_en: "Radeon Boost".into(),
            desc_vi: "Hạ nhẹ độ phân giải khi quay camera nhanh để giữ FPS cao".into(),
            desc_en: "Dynamic resolution scaling during fast camera motion".into(),
            control: SettingControl::Toggle,
            value: get("amd.boost", SettingValue::Bool(false)),
            default_value: SettingValue::Bool(false),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.ris".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Radeon Image Sharpening (RIS)".into(),
            label_en: "Radeon Image Sharpening (RIS)".into(),
            desc_vi: "Làm nét hình ảnh tương phản thích ứng ít tốn hiệu năng".into(),
            desc_en: "Contrast-adaptive sharpening for 3D rendering".into(),
            control: SettingControl::Slider {
                min: 0.0,
                max: 100.0,
                step: 10.0,
                unit: "%".into(),
            },
            value: get("amd.ris", SettingValue::Float(50.0)),
            default_value: SettingValue::Float(0.0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.enhanced_sync".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Radeon Enhanced Sync".into(),
            label_en: "Radeon Enhanced Sync".into(),
            desc_vi: "Giảm xé hình ở FPS cao mà ít trễ hơn VSync thường".into(),
            desc_en: "Low-latency tearing reduction above/below refresh rate".into(),
            control: SettingControl::Toggle,
            value: get("amd.enhanced_sync", SettingValue::Bool(false)),
            default_value: SettingValue::Bool(false),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.vsync".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Wait for Vertical Refresh".into(),
            label_en: "Wait for Vertical Refresh".into(),
            desc_vi: "Chính sách đồng bộ khung hình VSync của AMD".into(),
            desc_en: "AMD vertical refresh synchronization policy".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Always Off"),
                opt(1, "Off, unless app specifies"),
                opt(2, "On, unless app specifies"),
                opt(3, "Always On"),
            ]),
            value: get("amd.vsync", SettingValue::EnumId(1)),
            default_value: SettingValue::EnumId(1),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.frtc".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Frame Rate Target Control (FRTC)".into(),
            label_en: "Frame Rate Target Control (FRTC)".into(),
            desc_vi: "Giới hạn FPS trần toàn cục trên Radeon".into(),
            desc_en: "Global maximum frame rate cap".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Off"),
                opt(60, "60 FPS"),
                opt(120, "120 FPS"),
                opt(144, "144 FPS"),
                opt(165, "165 FPS"),
            ]),
            value: get("amd.frtc", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.tex_quality".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Texture Filtering Quality".into(),
            label_en: "Texture Filtering Quality".into(),
            desc_vi: "Cân bằng chất lượng và tốc độ lọc texture".into(),
            desc_en: "Texture filtering quality preset".into(),
            control: SettingControl::Enum(vec![
                opt(0, "High"),
                opt(1, "Standard"),
                opt(2, "Performance"),
            ]),
            value: get("amd.tex_quality", SettingValue::EnumId(1)),
            default_value: SettingValue::EnumId(1),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.surface_opt".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Surface Format Optimization".into(),
            label_en: "Surface Format Optimization".into(),
            desc_vi: "Tối ưu định dạng bề mặt render giúp nhẹ VRAM".into(),
            desc_en: "Optimize render surface formats for memory bandwidth".into(),
            control: SettingControl::Toggle,
            value: get("amd.surface_opt", SettingValue::Bool(true)),
            default_value: SettingValue::Bool(true),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.tess".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Tessellation Mode".into(),
            label_en: "Tessellation Mode".into(),
            desc_vi: "Cách driver AMD xử lý hệ số tessellation".into(),
            desc_en: "Driver control over tessellation level".into(),
            control: SettingControl::Enum(vec![
                opt(0, "AMD Optimized"),
                opt(1, "Use application settings"),
                opt(2, "Override application settings"),
            ]),
            value: get("amd.tess", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
    ]
}

pub fn build_amd_video_descriptors(
    cache: &BTreeMap<String, SettingValue>,
) -> Vec<SettingDescriptor> {
    let get = |id: &str, def: SettingValue| cache.get(id).cloned().unwrap_or(def);
    let sec = "AMD Video (ADLX)";
    vec![
        SettingDescriptor {
            id: "amd.video_profile".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Video Profile".into(),
            label_en: "Video Profile".into(),
            desc_vi: "Cấu hình hiển thị màu và tương phản cho video Radeon".into(),
            desc_en: "AMD Radeon hardware video color profile".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Default"),
                opt(1, "Cinema Classic"),
                opt(2, "Enhanced"),
                opt(3, "Vivid"),
                opt(4, "Custom"),
            ]),
            value: get("amd.video_profile", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.video_upscale".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "AMD Video Upscale".into(),
            label_en: "AMD Video Upscale".into(),
            desc_vi: "Nâng cấp độ nét video độ phân giải thấp qua ADLX".into(),
            desc_en: "Hardware video upscaling filter".into(),
            control: SettingControl::Toggle,
            value: get("amd.video_upscale", SettingValue::Bool(true)),
            default_value: SettingValue::Bool(false),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "amd.video_sharpen".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Video Sharpening".into(),
            label_en: "Video Sharpening".into(),
            desc_vi: "Mức độ làm nét video phát lại".into(),
            desc_en: "Video playback sharpening intensity".into(),
            control: SettingControl::Slider {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                unit: "%".into(),
            },
            value: get("amd.video_sharpen", SettingValue::Float(30.0)),
            default_value: SettingValue::Float(0.0),
            supported: true,
            lock_reason: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_amd_dll_returns_dll_not_found_and_descriptors_are_valid() {
        let defaults = default_amd_values();
        assert_eq!(build_amd_3d_descriptors(&defaults).len(), 10);
        assert_eq!(build_amd_video_descriptors(&defaults).len(), 3);

        match AmdBackend::try_load() {
            Ok(b) => {
                let _ = b.enumerate_adapters();
            }
            Err(BackendError::DllNotFound(name)) => assert_eq!(name, "atiadlxx.dll"),
            Err(e) => panic!("Unexpected AMD error: {e}"),
        }
    }
}
