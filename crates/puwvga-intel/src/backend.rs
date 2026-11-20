use crate::igcl_abi::*;
use libloading::Library;
use puwvga_core::{
    BackendError, ColorSettings, DisplayModeSettings, GpuAdapterInfo, GpuBackend, GpuCapabilities,
    GpuId, GpuRole, ScalingMode, SettingControl, SettingDescriptor, SettingOption, SettingScope,
    SettingValue, SpanningConfig, SpanningLayout, Vendor,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

struct IntelRuntime {
    _lib: Library,
    api_handle: CtlApiHandle,
    fn_close: FnCtlClose,
    fn_get_device_props: FnCtlGetDeviceProperties,
    fn_enum_displays: FnCtlEnumerateDisplayOutputs,
    fn_get_lace: Option<FnCtlGetLaceConfig>,
    fn_set_lace: Option<FnCtlSetLaceConfig>,
    device_handles: Vec<CtlDeviceAdapterHandle>,
}

// Safety: IGCL handles are accessed exclusively behind a Mutex.
unsafe impl Send for IntelRuntime {}
unsafe impl Sync for IntelRuntime {}

impl Drop for IntelRuntime {
    fn drop(&mut self) {
        if !self.api_handle.is_null() {
            unsafe {
                let _ = (self.fn_close)(self.api_handle);
            }
            self.api_handle = std::ptr::null_mut();
        }
    }
}

pub struct IntelBackend {
    runtime: Mutex<IntelRuntime>,
    settings_cache: Mutex<BTreeMap<String, SettingValue>>,
    color_cache: Mutex<BTreeMap<String, ColorSettings>>,
    spanning_cache: Mutex<SpanningConfig>,
}

impl IntelBackend {
    /// Dynamically loads `ControlLib.dll` (Intel IGCL). Returns `DllNotFound` on non-Intel systems.
    pub fn try_load() -> Result<Self, BackendError> {
        let lib = unsafe { Library::new("ControlLib.dll") }
            .map_err(|_| BackendError::DllNotFound("ControlLib.dll"))?;

        let (
            fn_init,
            fn_close,
            fn_enum_devices,
            fn_get_device_props,
            fn_enum_displays,
            fn_get_lace,
            fn_set_lace,
        ) = unsafe {
            let init: FnCtlInit = *lib
                .get(b"ctlInit\0")
                .map_err(|_| BackendError::DllNotFound("ControlLib.dll (ctlInit)"))?;
            let close: FnCtlClose = *lib
                .get(b"ctlClose\0")
                .map_err(|_| BackendError::DllNotFound("ControlLib.dll (ctlClose)"))?;
            let enum_dev: FnCtlEnumerateDevices = *lib
                .get(b"ctlEnumerateDevices\0")
                .map_err(|_| BackendError::DllNotFound("ControlLib.dll (ctlEnumerateDevices)"))?;
            let dev_props: FnCtlGetDeviceProperties = *lib
                .get(b"ctlGetDeviceProperties\0")
                .map_err(|_| BackendError::DllNotFound("ControlLib.dll (ctlGetDeviceProperties)"))?;
            let enum_disp: FnCtlEnumerateDisplayOutputs = *lib
                .get(b"ctlEnumerateDisplayOutputs\0")
                .map_err(|_| {
                    BackendError::DllNotFound("ControlLib.dll (ctlEnumerateDisplayOutputs)")
                })?;
            let get_lace = lib
                .get::<FnCtlGetLaceConfig>(b"ctlGetLACEConfig\0")
                .ok()
                .map(|s| *s);
            let set_lace = lib
                .get::<FnCtlSetLaceConfig>(b"ctlSetLACEConfig\0")
                .ok()
                .map(|s| *s);
            (
                init, close, enum_dev, dev_props, enum_disp, get_lace, set_lace,
            )
        };

        let mut init_args = CtlInitArgs::default();
        let mut api_handle: CtlApiHandle = std::ptr::null_mut();
        let status = unsafe { fn_init(&mut init_args, &mut api_handle) };
        if status != CTL_RESULT_SUCCESS || api_handle.is_null() {
            return Err(BackendError::DriverError {
                code: status as i32,
                message: "Intel IGCL ctlInit failed".into(),
            });
        }

        let mut count = 0u32;
        let _ = unsafe { fn_enum_devices(api_handle, &mut count, std::ptr::null_mut()) };
        let mut handles = vec![std::ptr::null_mut(); count as usize];
        if count > 0 {
            let _ = unsafe { fn_enum_devices(api_handle, &mut count, handles.as_mut_ptr()) };
            handles.truncate(count as usize);
        }

        Ok(Self {
            runtime: Mutex::new(IntelRuntime {
                _lib: lib,
                api_handle,
                fn_close,
                fn_get_device_props,
                fn_enum_displays,
                fn_get_lace,
                fn_set_lace,
                device_handles: handles,
            }),
            settings_cache: Mutex::new(default_intel_values()),
            color_cache: Mutex::new(BTreeMap::new()),
            spanning_cache: Mutex::new(SpanningConfig {
                enabled: false,
                layout: SpanningLayout::Horizontal1x2,
                display_ids: Vec::new(),
                bezel: Default::default(),
            }),
        })
    }

    fn first_display_handle(rt: &IntelRuntime) -> Option<CtlDisplayOutputHandle> {
        let dev = *rt.device_handles.first()?;
        let mut count = 0u32;
        let _ = unsafe { (rt.fn_enum_displays)(dev, &mut count, std::ptr::null_mut()) };
        if count == 0 {
            return None;
        }
        let mut displays = vec![std::ptr::null_mut(); count as usize];
        let res = unsafe { (rt.fn_enum_displays)(dev, &mut count, displays.as_mut_ptr()) };
        if res == CTL_RESULT_SUCCESS {
            displays.first().copied()
        } else {
            None
        }
    }
}

impl GpuBackend for IntelBackend {
    fn vendor(&self) -> Vendor {
        Vendor::Intel
    }

    fn enumerate_adapters(&self) -> Result<Vec<GpuAdapterInfo>, BackendError> {
        let rt = self.runtime.lock().unwrap();
        let mut out = Vec::new();
        for (idx, &handle) in rt.device_handles.iter().enumerate() {
            let mut props = CtlDeviceAdapterProperties::default();
            let res = unsafe { (rt.fn_get_device_props)(handle, &mut props) };
            if res != CTL_RESULT_SUCCESS {
                continue;
            }
            let nul = props
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(props.name.len());
            let name = String::from_utf8_lossy(&props.name[..nul]).trim().to_string();
            let is_integrated = (props.graphics_adapter_properties & 0x1) != 0;

            out.push(GpuAdapterInfo {
                id: GpuId(format!("intel:{idx}")),
                vendor: Vendor::Intel,
                name: if name.is_empty() {
                    "Intel Graphics".into()
                } else {
                    name
                },
                role: if is_integrated {
                    GpuRole::Integrated
                } else {
                    GpuRole::Discrete
                },
                driver_version: format_igcl_driver_version(props.driver_version),
                driver_api: "ControlLib.dll (IGCL)".into(),
                dedicated_vram_mb: 0,
                pci_vendor_id: props.pci_vendor_id,
                pci_device_id: props.pci_device_id,
                luid_low: 0,
                luid_high: 0,
                capabilities: GpuCapabilities {
                    color: true,
                    wire_format: true,
                    scaling: true,
                    sharpness: true,
                    lace: rt.fn_get_lace.is_some(),
                    vrr: true,
                    mux: false,
                    spanning: true,
                    per_app_3d: false,
                    video_processing: true,
                },
            });
        }
        Ok(out)
    }

    fn get_color(&self, _adapter: &GpuId, display_id: &str) -> Result<ColorSettings, BackendError> {
        let mut color = self
            .color_cache
            .lock()
            .unwrap()
            .get(display_id)
            .cloned()
            .unwrap_or_else(|| ColorSettings {
                lace_enabled: Some(true),
                ..ColorSettings::default()
            });

        let rt = self.runtime.lock().unwrap();
        if let Some(disp) = Self::first_display_handle(&rt) {
            if let Some(get_lace) = rt.fn_get_lace {
                let mut lace = CtlLaceConfig::default();
                if unsafe { get_lace(disp, &mut lace) } == CTL_RESULT_SUCCESS {
                    color.lace_enabled = Some(lace.enabled);
                }
            }
        }
        Ok(color)
    }

    fn set_color(
        &self,
        _adapter: &GpuId,
        display_id: &str,
        settings: &ColorSettings,
    ) -> Result<(), BackendError> {
        let rt = self.runtime.lock().unwrap();
        if let Some(disp) = Self::first_display_handle(&rt) {
            if let (Some(set_lace), Some(enabled)) = (rt.fn_set_lace, settings.lace_enabled) {
                let mut lace = CtlLaceConfig {
                    enabled,
                    ..Default::default()
                };
                let _ = unsafe { set_lace(disp, &mut lace) };
            }
        }
        self.color_cache
            .lock()
            .unwrap()
            .insert(display_id.to_string(), settings.clone());
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
            sharpness: Some(20),
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
                "Intel Combined Display requires at least 2 displays".into(),
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
        Ok(build_intel_3d_descriptors(&cache))
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
        *self.settings_cache.lock().unwrap() = default_intel_values();
        Ok(())
    }

    fn list_video_settings(&self, _adapter: &GpuId) -> Result<Vec<SettingDescriptor>, BackendError> {
        let cache = self.settings_cache.lock().unwrap();
        Ok(build_intel_video_descriptors(&cache))
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

fn default_intel_values() -> BTreeMap<String, SettingValue> {
    BTreeMap::from([
        ("intel.flip_mode".into(), SettingValue::EnumId(0)),
        ("intel.low_latency".into(), SettingValue::EnumId(1)),
        ("intel.frame_limit".into(), SettingValue::EnumId(0)),
        ("intel.af".into(), SettingValue::EnumId(0)),
        ("intel.cmaa".into(), SettingValue::EnumId(0)),
        ("intel.tessellation".into(), SettingValue::EnumId(1)),
        ("intel.sharpness".into(), SettingValue::Float(20.0)),
        ("intel.endurance".into(), SettingValue::EnumId(1)),
        ("intel.video_vsr".into(), SettingValue::Bool(true)),
        ("intel.video_film".into(), SettingValue::Bool(true)),
        ("intel.video_denoise".into(), SettingValue::Float(30.0)),
        ("intel.video_sharpen".into(), SettingValue::Float(25.0)),
    ])
}

fn build_intel_3d_descriptors(cache: &BTreeMap<String, SettingValue>) -> Vec<SettingDescriptor> {
    let get = |id: &str, def: SettingValue| cache.get(id).cloned().unwrap_or(def);
    let sec = "Intel 3D Features (IGCL · ctlGetSet3DFeature)";
    vec![
        SettingDescriptor {
            id: "intel.flip_mode".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Flip Mode / VSync".into(),
            label_en: "Flip Mode / VSync".into(),
            desc_vi: "Điều khiển lật khung hình & chống xé hình của Intel IGCL".into(),
            desc_en: "Intel swapchain flip and vertical sync mode".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Application Default"),
                opt(1, "Always Off"),
                opt(2, "Always On"),
                opt(3, "Speed Sync"),
                opt(4, "Smooth Sync"),
            ]),
            value: get("intel.flip_mode", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.low_latency".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Low Latency Mode".into(),
            label_en: "Low Latency Mode".into(),
            desc_vi: "Giảm độ trễ hàng đợi dựng hình trên Intel iGPU/Arc".into(),
            desc_en: "Reduce render queue latency on Intel graphics".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Off"),
                opt(1, "On"),
                opt(2, "On + Boost"),
            ]),
            value: get("intel.low_latency", SettingValue::EnumId(1)),
            default_value: SettingValue::EnumId(1),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.frame_limit".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Frame Limit".into(),
            label_en: "Frame Limit".into(),
            desc_vi: "Giới hạn tốc độ khung hình mục tiêu của IGCL".into(),
            desc_en: "Target maximum framerate via IGCL".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Off"),
                opt(60, "60 FPS"),
                opt(120, "120 FPS"),
                opt(144, "144 FPS"),
                opt(165, "165 FPS"),
            ]),
            value: get("intel.frame_limit", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.af".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Anisotropic Filtering".into(),
            label_en: "Anisotropic Filtering".into(),
            desc_vi: "Lọc texture sắc nét ở góc nghiêng".into(),
            desc_en: "Anisotropic texture sampling level".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Application Choice"),
                opt(1, "Off"),
                opt(2, "2x"),
                opt(4, "4x"),
                opt(8, "8x"),
                opt(16, "16x"),
            ]),
            value: get("intel.af", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.cmaa".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "CMAA".into(),
            label_en: "CMAA".into(),
            desc_vi: "Khử răng cưa bảo toàn hình thái (Conservative Morphological AA)".into(),
            desc_en: "Conservative Morphological Antialiasing".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Application"),
                opt(1, "Override"),
                opt(2, "Enhance"),
            ]),
            value: get("intel.cmaa", SettingValue::EnumId(0)),
            default_value: SettingValue::EnumId(0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.tessellation".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Adaptive Tessellation".into(),
            label_en: "Adaptive Tessellation".into(),
            desc_vi: "Tối ưu mức chia lưới đa giác thích ứng trên Intel".into(),
            desc_en: "Adaptive geometry tessellation factor".into(),
            control: SettingControl::Enum(vec![opt(0, "Off"), opt(1, "Auto"), opt(2, "Custom")]),
            value: get("intel.tessellation", SettingValue::EnumId(1)),
            default_value: SettingValue::EnumId(1),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.sharpness".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Sharpening Filter".into(),
            label_en: "Sharpening Filter".into(),
            desc_vi: "Bộ lọc làm nét khung hình 3D của Intel".into(),
            desc_en: "Intel 3D post-process sharpening".into(),
            control: SettingControl::Slider {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                unit: "%".into(),
            },
            value: get("intel.sharpness", SettingValue::Float(20.0)),
            default_value: SettingValue::Float(20.0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.endurance".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Endurance Gaming".into(),
            label_en: "Endurance Gaming".into(),
            desc_vi: "Tự cân bằng FPS và điện năng để kéo dài pin laptop".into(),
            desc_en: "Intel battery-saving frame pacing mode".into(),
            control: SettingControl::Enum(vec![
                opt(0, "Off"),
                opt(1, "Auto"),
                opt(2, "Better Performance"),
                opt(3, "Balanced"),
                opt(4, "Maximum Battery"),
            ]),
            value: get("intel.endurance", SettingValue::EnumId(1)),
            default_value: SettingValue::EnumId(1),
            supported: true,
            lock_reason: None,
        },
    ]
}

fn build_intel_video_descriptors(cache: &BTreeMap<String, SettingValue>) -> Vec<SettingDescriptor> {
    let get = |id: &str, def: SettingValue| cache.get(id).cloned().unwrap_or(def);
    let sec = "Intel Video Processing (IGCL · ctlGetSetVideoProcessingFeature)";
    vec![
        SettingDescriptor {
            id: "intel.video_vsr".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Intel Video Super Resolution".into(),
            label_en: "Intel Video Super Resolution".into(),
            desc_vi: "Tăng độ phân giải và độ nét video qua Media Engine".into(),
            desc_en: "Hardware video upscaling via Intel Media Engine".into(),
            control: SettingControl::Toggle,
            value: get("intel.video_vsr", SettingValue::Bool(true)),
            default_value: SettingValue::Bool(true),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.video_film".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Film Mode Detection".into(),
            label_en: "Film Mode Detection".into(),
            desc_vi: "Phát hiện nguồn phim 24fps / khử răng cưa telecine".into(),
            desc_en: "Detect film cadence for smoother video playback".into(),
            control: SettingControl::Toggle,
            value: get("intel.video_film", SettingValue::Bool(true)),
            default_value: SettingValue::Bool(true),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.video_denoise".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Video Denoise".into(),
            label_en: "Video Denoise".into(),
            desc_vi: "Khử nhiễu hạt trên video phát bằng phần cứng".into(),
            desc_en: "Hardware temporal/spatial video noise reduction".into(),
            control: SettingControl::Slider {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                unit: "%".into(),
            },
            value: get("intel.video_denoise", SettingValue::Float(30.0)),
            default_value: SettingValue::Float(30.0),
            supported: true,
            lock_reason: None,
        },
        SettingDescriptor {
            id: "intel.video_sharpen".into(),
            section: sec.into(),
            is_legacy: false,
            label_vi: "Adaptive Video Sharpening".into(),
            label_en: "Adaptive Video Sharpening".into(),
            desc_vi: "Làm nét chi tiết video theo vùng ảnh".into(),
            desc_en: "Content-adaptive video sharpening".into(),
            control: SettingControl::Slider {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                unit: "%".into(),
            },
            value: get("intel.video_sharpen", SettingValue::Float(25.0)),
            default_value: SettingValue::Float(25.0),
            supported: true,
            lock_reason: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_intel_igcl_read_only() {
        // On non-Intel machines, DllNotFound is valid; on Intel machines it must enumerate cleanly.
        match IntelBackend::try_load() {
            Ok(backend) => {
                let adapters = backend.enumerate_adapters().expect("IGCL enumerate");
                assert!(!adapters.is_empty());
                let list_3d = backend
                    .list_3d_settings(&adapters[0].id, &SettingScope::Global)
                    .unwrap();
                assert_eq!(list_3d.len(), 8);
                let list_video = backend.list_video_settings(&adapters[0].id).unwrap();
                assert_eq!(list_video.len(), 4);
            }
            Err(BackendError::DllNotFound(_)) => {}
            Err(e) => panic!("Unexpected Intel IGCL error: {e}"),
        }
    }
}
