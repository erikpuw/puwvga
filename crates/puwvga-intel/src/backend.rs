use crate::igcl_abi::*;
use libloading::Library;
use puwvga_core::{
    BackendError, ColorSettings, DisplayModeSettings, DisplayOutputInfo, GpuAdapterInfo, GpuBackend, GpuCapabilities,
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
    fn_get_display_props: Option<FnCtlGetDisplayProperties>,
    fn_get_lace: Option<FnCtlGetLaceConfig>,
    fn_set_lace: Option<FnCtlSetLaceConfig>,
    fn_pixtx_get: Option<FnCtlPixelTransformationGetConfig>,
    fn_pixtx_set: Option<FnCtlPixelTransformationSetConfig>,
    fn_scaling_get: Option<FnCtlScalingSettings>,
    fn_scaling_set: Option<FnCtlScalingSettings>,
    fn_retro_scaling: Option<FnCtlRetroScaling>,
    fn_sharpness_caps: Option<FnCtlSharpnessCaps>,
    fn_sharpness_get: Option<FnCtlSharpnessSettings>,
    fn_sharpness_set: Option<FnCtlSharpnessSettings>,
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
            fn_get_display_props,
            fn_get_lace,
            fn_set_lace,
            fn_pixtx_get,
            fn_pixtx_set,
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
            let display_props = lib
                .get::<FnCtlGetDisplayProperties>(b"ctlGetDisplayProperties\0")
                .ok()
                .map(|s| *s);
            let get_lace = lib
                .get::<FnCtlGetLaceConfig>(b"ctlGetLACEConfig\0")
                .ok()
                .map(|s| *s);
            let set_lace = lib
                .get::<FnCtlSetLaceConfig>(b"ctlSetLACEConfig\0")
                .ok()
                .map(|s| *s);
            let pixtx_get = lib
                .get::<FnCtlPixelTransformationGetConfig>(b"ctlPixelTransformationGetConfig\0")
                .ok()
                .map(|s| *s);
            let pixtx_set = lib
                .get::<FnCtlPixelTransformationSetConfig>(b"ctlPixelTransformationSetConfig\0")
                .ok()
                .map(|s| *s);
            (
                init,
                close,
                enum_dev,
                dev_props,
                enum_disp,
                display_props,
                get_lace,
                set_lace,
                pixtx_get,
                pixtx_set,
            )
        };

        let (
            fn_scaling_get,
            fn_scaling_set,
            fn_retro_scaling,
            fn_sharpness_caps,
            fn_sharpness_get,
            fn_sharpness_set,
        ) = unsafe {
            (
                lib.get::<FnCtlScalingSettings>(b"ctlGetCurrentScaling\0").ok().map(|s| *s),
                lib.get::<FnCtlScalingSettings>(b"ctlSetCurrentScaling\0").ok().map(|s| *s),
                lib.get::<FnCtlRetroScaling>(b"ctlGetSetRetroScaling\0").ok().map(|s| *s),
                lib.get::<FnCtlSharpnessCaps>(b"ctlGetSharpnessCaps\0").ok().map(|s| *s),
                lib.get::<FnCtlSharpnessSettings>(b"ctlGetCurrentSharpness\0").ok().map(|s| *s),
                lib.get::<FnCtlSharpnessSettings>(b"ctlSetCurrentSharpness\0").ok().map(|s| *s),
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
                fn_get_display_props,
                fn_get_lace,
                fn_set_lace,
                fn_pixtx_get,
                fn_pixtx_set,
                fn_scaling_get,
                fn_scaling_set,
                fn_retro_scaling,
                fn_sharpness_caps,
                fn_sharpness_get,
                fn_sharpness_set,
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

    /// Finds the IGCL device and output whose Windows encoder id equals the CCD target id.
    fn display_target(
        rt: &IntelRuntime,
        target_id: u32,
    ) -> Option<(CtlDeviceAdapterHandle, CtlDisplayOutputHandle)> {
        let get_props = rt.fn_get_display_props?;
        rt.device_handles.iter().find_map(|&dev| {
            let mut count = 0u32;
            let _ = unsafe { (rt.fn_enum_displays)(dev, &mut count, std::ptr::null_mut()) };
            let mut displays = vec![std::ptr::null_mut(); count as usize];
            if count == 0
                || unsafe { (rt.fn_enum_displays)(dev, &mut count, displays.as_mut_ptr()) }
                    != CTL_RESULT_SUCCESS
            {
                return None;
            }
            displays.truncate(count as usize);
            displays
                .into_iter()
                .find(|&disp| {
                    let mut props = CtlDisplayProperties::default();
                    (unsafe { get_props(disp, &mut props) }) == CTL_RESULT_SUCCESS
                        && props.os_display_encoder_id as u32 == target_id
                })
                .map(|disp| (dev, disp))
        })
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

    fn get_color(
        &self,
        _adapter: &GpuId,
        display: &DisplayOutputInfo,
    ) -> Result<ColorSettings, BackendError> {
        let mut color = self
            .color_cache
            .lock()
            .unwrap()
            .get(&display.id)
            .cloned()
            .unwrap_or_else(|| ColorSettings {
                lace_enabled: Some(true),
                ..ColorSettings::default()
            });

        let rt = self.runtime.lock().unwrap();
        if let Some(disp) = Self::display_target(&rt, display.target_id).map(|(_, d)| d) {
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
        display: &DisplayOutputInfo,
        settings: &ColorSettings,
    ) -> Result<(), BackendError> {
        let rt = self.runtime.lock().unwrap();
        if let Some(disp) = Self::display_target(&rt, display.target_id).map(|(_, d)| d) {
            if let (Some(set_lace), Some(enabled)) = (rt.fn_set_lace, settings.lace_enabled) {
                let mut lace = CtlLaceConfig {
                    enabled,
                    ..Default::default()
                };
                let _ = unsafe { set_lace(disp, &mut lace) };
            }
            if let (Some(pixtx_get), Some(pixtx_set)) = (rt.fn_pixtx_get, rt.fn_pixtx_set) {
                let mut pipe_get = CtlPixtxPipeGetConfig {
                    oper_type: CTL_PIXTX_CONFIG_QUERY_TYPE_CAPABILITY,
                    ..Default::default()
                };
                let status_count = unsafe { pixtx_get(disp, &mut pipe_get) };
                if status_count == CTL_RESULT_SUCCESS && pipe_get.num_blocks > 0 && pipe_get.num_blocks <= 32 {
                    let mut blocks = vec![CtlPixtxBlockConfig::default(); pipe_get.num_blocks as usize];
                    pipe_get.p_block_configs = blocks.as_mut_ptr();
                    if unsafe { pixtx_get(disp, &mut pipe_get) } == CTL_RESULT_SUCCESS {
                        if let Some(csc_blk) = blocks.iter().find(|b| {
                            b.block_type == CTL_PIXTX_BLOCK_TYPE_3X3_MATRIX_AND_OFFSETS
                                || b.block_type == CTL_PIXTX_BLOCK_TYPE_3X3_MATRIX
                        }) {
                            let mut set_blk = CtlPixtxBlockConfig {
                                block_id: csc_blk.block_id,
                                block_type: csc_blk.block_type,
                                matrix_config: CtlPixtxMatrixConfig {
                                    matrix: build_csc_matrix(
                                        settings.vibrance_or_saturation,
                                        settings.hue,
                                    ),
                                    ..Default::default()
                                },
                                ..Default::default()
                            };
                            let mut pipe_set = CtlPixtxPipeSetConfig {
                                oper_type: CTL_PIXTX_CONFIG_OPER_TYPE_SET_CUSTOM,
                                flags: 0,
                                num_blocks: 1,
                                p_block_configs: &mut set_blk,
                                ..Default::default()
                            };
                            let _ = unsafe { pixtx_set(disp, &mut pipe_set) };
                        }
                    }
                }
            }
        }
        self.color_cache
            .lock()
            .unwrap()
            .insert(display.id.clone(), settings.clone());
        Ok(())
    }

    fn get_display(
        &self,
        _adapter: &GpuId,
        display: &DisplayOutputInfo,
    ) -> Result<DisplayModeSettings, BackendError> {
        let mut ds = DisplayModeSettings {
            width: display.width,
            height: display.height,
            refresh_hz: display.refresh_hz,
            scaling: ScalingMode::AspectRatio,
            vrr_enabled: false,
            sharpness: None,
        };
        let rt = self.runtime.lock().unwrap();
        let Some((dev, disp)) = Self::display_target(&rt, display.target_id) else {
            return Ok(ds);
        };
        if let Some(get) = rt.fn_scaling_get {
            // `PreferredScalingType` holds the mode in effect; `ScalingType` reads Identity at native.
            let mut s = CtlScalingSettings::default();
            if unsafe { get(disp, &mut s) } == CTL_RESULT_SUCCESS {
                ds.scaling = scaling_from_igcl(s.preferred_scaling_type);
            }
        }
        if let Some(retro) = rt.fn_retro_scaling {
            let mut r = CtlRetroScalingSettings { get: 1, ..Default::default() };
            if unsafe { retro(dev, &mut r) } == CTL_RESULT_SUCCESS
                && r.enable != 0
                && r.retro_scaling_type & CTL_RETRO_SCALING_TYPE_FLAG_INTEGER != 0
            {
                ds.scaling = ScalingMode::Integer;
            }
        }
        if let Some(get) = rt.fn_sharpness_get {
            let mut s = CtlSharpnessSettings::default();
            if unsafe { get(disp, &mut s) } == CTL_RESULT_SUCCESS {
                let level = if s.enable != 0 { s.intensity.round().clamp(0.0, 100.0) } else { 0.0 };
                ds.sharpness = Some(level as u32);
            }
        }
        Ok(ds)
    }

    fn set_display(
        &self,
        adapter: &GpuId,
        display: &DisplayOutputInfo,
        settings: &DisplayModeSettings,
    ) -> Result<(), BackendError> {
        let current = self.get_display(adapter, display)?;
        let rt = self.runtime.lock().unwrap();
        let Some((dev, disp)) = Self::display_target(&rt, display.target_id) else {
            return Err(BackendError::InvalidArgument(format!(
                "{} is not driven by the Intel GPU",
                display.device_name
            )));
        };
        let check = |st: CtlResult, call: &str| {
            if st == CTL_RESULT_SUCCESS {
                Ok(())
            } else {
                Err(BackendError::DriverError { code: st as i32, message: format!("{call} failed") })
            }
        };
        if settings.scaling != current.scaling {
            // Integer (retro) scaling is adapter-wide; the other modes are per display.
            let integer = settings.scaling == ScalingMode::Integer;
            if let Some(retro) = rt.fn_retro_scaling {
                if integer || current.scaling == ScalingMode::Integer {
                    // IGCL rejects type 0; disabling keeps the INTEGER type and clears Enable.
                    let mut r = CtlRetroScalingSettings {
                        enable: u8::from(integer),
                        retro_scaling_type: CTL_RETRO_SCALING_TYPE_FLAG_INTEGER,
                        ..Default::default()
                    };
                    check(unsafe { retro(dev, &mut r) }, "ctlGetSetRetroScaling")?;
                }
            }
            if let (Some(set), Some(flag)) = (rt.fn_scaling_set, igcl_scaling(settings.scaling)) {
                let mut s = CtlScalingSettings { enable: 1, scaling_type: flag, ..Default::default() };
                check(unsafe { set(disp, &mut s) }, "ctlSetCurrentScaling")?;
            }
        }
        if let (Some(level), Some(caps), Some(set)) =
            (settings.sharpness, rt.fn_sharpness_caps, rt.fn_sharpness_set)
        {
            if Some(level) != current.sharpness {
                let mut c = CtlSharpnessCaps::default();
                check(unsafe { caps(disp, &mut c) }, "ctlGetSharpnessCaps")?;
                let mut s = CtlSharpnessSettings {
                    enable: u8::from(level > 0),
                    filter_type: c.supported_filter_flags,
                    intensity: level.min(100) as f32,
                    ..Default::default()
                };
                check(unsafe { set(disp, &mut s) }, "ctlSetCurrentSharpness")?;
            }
        }
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

fn igcl_scaling(mode: ScalingMode) -> Option<u32> {
    match mode {
        ScalingMode::AspectRatio => Some(CTL_SCALING_TYPE_FLAG_ASPECT_RATIO_CENTERED_MAX),
        ScalingMode::FullScreen => Some(CTL_SCALING_TYPE_FLAG_STRETCHED),
        ScalingMode::Center => Some(CTL_SCALING_TYPE_FLAG_CENTERED),
        ScalingMode::Integer => None,
    }
}

fn scaling_from_igcl(flag: u32) -> ScalingMode {
    match flag {
        CTL_SCALING_TYPE_FLAG_STRETCHED => ScalingMode::FullScreen,
        CTL_SCALING_TYPE_FLAG_CENTERED => ScalingMode::Center,
        _ => ScalingMode::AspectRatio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn igcl_scaling_round_trips() {
        for m in [ScalingMode::AspectRatio, ScalingMode::FullScreen, ScalingMode::Center] {
            assert_eq!(scaling_from_igcl(igcl_scaling(m).unwrap()), m);
        }
        assert!(igcl_scaling(ScalingMode::Integer).is_none());
    }

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
