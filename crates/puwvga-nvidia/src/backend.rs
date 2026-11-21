use crate::drs::{
    build_nvidia_3d_descriptors, build_nvidia_video_descriptors, default_nvidia_values,
    drs_id_for_setting,
};
use crate::nvapi_abi::*;
use libloading::Library;
use puwvga_core::{
    BackendError, ColorSettings, DisplayModeSettings, DisplayOutputInfo, GpuAdapterInfo, GpuBackend, GpuCapabilities,
    GpuId, GpuRole, MuxMode, ScalingMode, SettingDescriptor, SettingScope, SettingValue,
    SpanningConfig, SpanningLayout, Vendor,
};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem::transmute;
use std::sync::Mutex;

struct NvRuntime {
    _lib: Library,
    fn_unload: Option<FnUnload>,
    fn_get_driver_ver: Option<FnGetDriverVersion>,
    fn_gpu_full_name: Option<FnGpuGetFullName>,
    fn_assoc_display: Option<FnGetAssociatedDisplayHandle>,
    fn_get_dvc: Option<FnGetDvcInfoEx>,
    fn_set_dvc: Option<FnSetDvcLevelEx>,
    fn_drs_create: Option<FnDrsCreateSession>,
    fn_drs_destroy: Option<FnDrsDestroySession>,
    fn_drs_load: Option<FnDrsLoadSettings>,
    fn_drs_save: Option<FnDrsSaveSettings>,
    fn_drs_base_profile: Option<FnDrsGetBaseProfile>,
    fn_drs_get_setting: Option<FnDrsGetSetting>,
    fn_drs_set_setting: Option<FnDrsSetSetting>,
    fn_drs_restore_all: Option<FnDrsRestoreAllDefaults>,
    gpu_handles: Vec<NvPhysicalGpuHandle>,
}

unsafe impl Send for NvRuntime {}
unsafe impl Sync for NvRuntime {}

impl Drop for NvRuntime {
    fn drop(&mut self) {
        if let Some(unload) = self.fn_unload {
            unsafe {
                let _ = unload();
            }
        }
    }
}

pub struct NvidiaBackend {
    runtime: Mutex<NvRuntime>,
    gpu_name: String,
    is_laptop: bool,
    hdr_active: Mutex<bool>,
    settings_cache: Mutex<BTreeMap<String, SettingValue>>,
    color_cache: Mutex<BTreeMap<String, ColorSettings>>,
    spanning_cache: Mutex<SpanningConfig>,
    mux_cache: Mutex<MuxMode>,
}

unsafe fn query_fn<T>(qi: FnQueryInterface, id: u32) -> Option<T> {
    let ptr: *const c_void = unsafe { qi(id) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { std::mem::transmute_copy(&ptr) })
    }
}

impl NvidiaBackend {
    /// Dynamically loads `nvapi64.dll` (zero `nvml.dll`, zero telemetry).
    pub fn try_load() -> Result<Self, BackendError> {
        let lib = unsafe { Library::new("nvapi64.dll") }
            .map_err(|_| BackendError::DllNotFound("nvapi64.dll"))?;

        let qi: FnQueryInterface = unsafe {
            let sym = lib
                .get::<FnQueryInterface>(b"nvapi_QueryInterface\0")
                .map_err(|_| BackendError::DllNotFound("nvapi64.dll (nvapi_QueryInterface)"))?;
            *sym
        };

        let fn_init: FnInitialize = unsafe { query_fn(qi, NVAPI_INITIALIZE) }
            .ok_or(BackendError::DllNotFound("nvapi64.dll (NvAPI_Initialize)"))?;
        let status = unsafe { fn_init() };
        if status != NVAPI_OK {
            return Err(BackendError::DriverError {
                code: status,
                message: "NvAPI_Initialize failed".into(),
            });
        }

        let fn_unload: Option<FnUnload> = unsafe { query_fn(qi, NVAPI_UNLOAD) };
        let fn_get_driver_ver: Option<FnGetDriverVersion> =
            unsafe { query_fn(qi, NVAPI_SYS_GET_DRIVER_AND_BRANCH_VERSION) };
        let fn_enum_gpus: Option<FnEnumPhysicalGpus> =
            unsafe { query_fn(qi, NVAPI_ENUM_PHYSICAL_GPUS) };
        let fn_gpu_full_name: Option<FnGpuGetFullName> =
            unsafe { query_fn(qi, NVAPI_GPU_GET_FULL_NAME) };
        let fn_assoc_display: Option<FnGetAssociatedDisplayHandle> =
            unsafe { query_fn(qi, NVAPI_GET_ASSOCIATED_NVIDIA_DISPLAY_HANDLE) };
        let fn_get_dvc: Option<FnGetDvcInfoEx> = unsafe { query_fn(qi, NVAPI_GET_DVC_INFO_EX) };
        let fn_set_dvc: Option<FnSetDvcLevelEx> = unsafe { query_fn(qi, NVAPI_SET_DVC_LEVEL_EX) };

        let fn_drs_create: Option<FnDrsCreateSession> =
            unsafe { query_fn(qi, NVAPI_DRS_CREATE_SESSION) };
        let fn_drs_destroy: Option<FnDrsDestroySession> =
            unsafe { query_fn(qi, NVAPI_DRS_DESTROY_SESSION) };
        let fn_drs_load: Option<FnDrsLoadSettings> =
            unsafe { query_fn(qi, NVAPI_DRS_LOAD_SETTINGS) };
        let fn_drs_save: Option<FnDrsSaveSettings> =
            unsafe { query_fn(qi, NVAPI_DRS_SAVE_SETTINGS) };
        let fn_drs_base_profile: Option<FnDrsGetBaseProfile> =
            unsafe { query_fn(qi, NVAPI_DRS_GET_BASE_PROFILE) };
        let fn_drs_get_setting: Option<FnDrsGetSetting> =
            unsafe { query_fn(qi, NVAPI_DRS_GET_SETTING) };
        let fn_drs_set_setting: Option<FnDrsSetSetting> =
            unsafe { query_fn(qi, NVAPI_DRS_SET_SETTING) };
        let fn_drs_restore_all: Option<FnDrsRestoreAllDefaults> =
            unsafe { query_fn(qi, NVAPI_DRS_RESTORE_ALL_DEFAULTS) };

        let mut raw_handles = [std::ptr::null_mut(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0u32;
        if let Some(enum_fn) = fn_enum_gpus {
            let _ = unsafe { enum_fn(raw_handles.as_mut_ptr(), &mut count) };
        }
        let gpu_handles = raw_handles[..(count as usize).min(NVAPI_MAX_PHYSICAL_GPUS)].to_vec();

        let mut gpu_name = "NVIDIA GPU".to_string();
        if let (Some(&first), Some(name_fn)) = (gpu_handles.first(), fn_gpu_full_name) {
            let mut buf = [0u8; NVAPI_SHORT_STRING_MAX];
            if unsafe { name_fn(first, buf.as_mut_ptr()) } == NVAPI_OK {
                let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                let parsed = String::from_utf8_lossy(&buf[..nul]).trim().to_string();
                if !parsed.is_empty() {
                    gpu_name = parsed;
                }
            }
        }
        let is_laptop = gpu_name.to_ascii_lowercase().contains("laptop");
        let _ = transmute::<usize, usize>; // keep import clean if needed

        let mut cache = default_nvidia_values(&gpu_name);
        let rt = NvRuntime {
            _lib: lib,
            fn_unload,
            fn_get_driver_ver,
            fn_gpu_full_name,
            fn_assoc_display,
            fn_get_dvc,
            fn_set_dvc,
            fn_drs_create,
            fn_drs_destroy,
            fn_drs_load,
            fn_drs_save,
            fn_drs_base_profile,
            fn_drs_get_setting,
            fn_drs_set_setting,
            fn_drs_restore_all,
            gpu_handles,
        };
        Self::sync_cache_from_drs(&rt, &mut cache);

        Ok(Self {
            runtime: Mutex::new(rt),
            gpu_name,
            is_laptop,
            hdr_active: Mutex::new(false),
            settings_cache: Mutex::new(cache),
            color_cache: Mutex::new(BTreeMap::new()),
            spanning_cache: Mutex::new(SpanningConfig {
                enabled: false,
                layout: SpanningLayout::Horizontal1x2,
                display_ids: Vec::new(),
                bezel: Default::default(),
            }),
            mux_cache: Mutex::new(MuxMode::HybridOptimus),
        })
    }

    pub fn set_hdr_active(&self, active: bool) {
        *self.hdr_active.lock().unwrap() = active;
    }

    /// Resolves the NVAPI handle for one GDI output (`\\.\DISPLAYn`); `None` when the output
    /// is not driven by this NVIDIA GPU (e.g. Optimus panel on the iGPU).
    fn display_handle(rt: &NvRuntime, device_name: &str) -> Option<NvDisplayHandle> {
        let assoc = rt.fn_assoc_display?;
        let name = std::ffi::CString::new(device_name).ok()?;
        let mut h_disp: NvDisplayHandle = std::ptr::null_mut();
        (unsafe { assoc(name.as_ptr(), &mut h_disp) } == NVAPI_OK && !h_disp.is_null())
            .then_some(h_disp)
    }

    fn sync_cache_from_drs(rt: &NvRuntime, cache: &mut BTreeMap<String, SettingValue>) {
        let (Some(create), Some(destroy), Some(load), Some(base_prof), Some(get_fn)) = (
            rt.fn_drs_create,
            rt.fn_drs_destroy,
            rt.fn_drs_load,
            rt.fn_drs_base_profile,
            rt.fn_drs_get_setting,
        ) else {
            return;
        };

        let mut session: NvDrsSessionHandle = std::ptr::null_mut();
        if unsafe { create(&mut session) } != NVAPI_OK || session.is_null() {
            return;
        }
        if unsafe { load(session) } == NVAPI_OK {
            let mut profile: NvDrsProfileHandle = std::ptr::null_mut();
            if unsafe { base_prof(session, &mut profile) } == NVAPI_OK && !profile.is_null() {
                let keys: Vec<String> = cache.keys().cloned().collect();
                for key in keys {
                    if let Some(drs_id) = drs_id_for_setting(&key) {
                        let mut setting = Box::new(NvDrsSetting::new_dword(drs_id, 0));
                        if unsafe { get_fn(session, profile, drs_id, setting.as_mut()) } == NVAPI_OK
                        {
                            let raw = setting.current_dword();
                            let mapped = match cache.get(&key) {
                                Some(SettingValue::Bool(_)) => SettingValue::Bool(raw != 0),
                                _ => SettingValue::EnumId(decode_drs_dword(&key, raw)),
                            };
                            cache.insert(key, mapped);
                        }
                    }
                }
            }
        }
        unsafe {
            let _ = destroy(session);
        }
    }

    fn write_drs_dword(rt: &NvRuntime, drs_id: u32, dword_val: u32) -> Result<(), BackendError> {
        let (Some(create), Some(destroy), Some(load), Some(save), Some(base_prof), Some(set_fn)) = (
            rt.fn_drs_create,
            rt.fn_drs_destroy,
            rt.fn_drs_load,
            rt.fn_drs_save,
            rt.fn_drs_base_profile,
            rt.fn_drs_set_setting,
        ) else {
            return Err(BackendError::Unsupported("NVAPI DRS".into()));
        };

        let mut session: NvDrsSessionHandle = std::ptr::null_mut();
        let st = unsafe { create(&mut session) };
        if st != NVAPI_OK || session.is_null() {
            return Err(BackendError::DriverError {
                code: st,
                message: "NvAPI_DRS_CreateSession failed".into(),
            });
        }

        let result = (|| {
            let st_load = unsafe { load(session) };
            if st_load != NVAPI_OK {
                return Err(BackendError::DriverError {
                    code: st_load,
                    message: "NvAPI_DRS_LoadSettings failed".into(),
                });
            }
            let mut profile: NvDrsProfileHandle = std::ptr::null_mut();
            let st_prof = unsafe { base_prof(session, &mut profile) };
            if st_prof != NVAPI_OK {
                return Err(BackendError::DriverError {
                    code: st_prof,
                    message: "NvAPI_DRS_GetBaseProfile failed".into(),
                });
            }
            let mut setting = Box::new(NvDrsSetting::new_dword(drs_id, dword_val));
            let st_set = unsafe { set_fn(session, profile, setting.as_mut()) };
            if st_set == NVAPI_INVALID_USER_PRIVILEGE {
                return Err(BackendError::AccessDenied(format!(
                    "NVIDIA DRS setting 0x{drs_id:08X}"
                )));
            }
            if st_set != NVAPI_OK {
                return Err(BackendError::DriverError {
                    code: st_set,
                    message: format!("NvAPI_DRS_SetSetting(0x{drs_id:08X}) failed"),
                });
            }
            let st_save = unsafe { save(session) };
            if st_save != NVAPI_OK {
                if st_save == -104 || st_save == NVAPI_INVALID_USER_PRIVILEGE {
                    return Err(BackendError::AccessDenied("NVIDIA DRS SaveSettings".into()));
                }
                return Err(BackendError::DriverError {
                    code: st_save,
                    message: "NvAPI_DRS_SaveSettings failed".into(),
                });
            }
            Ok(())
        })();

        unsafe {
            let _ = destroy(session);
        }
        result
    }
}

impl GpuBackend for NvidiaBackend {
    fn vendor(&self) -> Vendor {
        Vendor::Nvidia
    }

    fn enumerate_adapters(&self) -> Result<Vec<GpuAdapterInfo>, BackendError> {
        let rt = self.runtime.lock().unwrap();
        let driver_version = if let Some(ver_fn) = rt.fn_get_driver_ver {
            let mut ver = 0u32;
            let mut branch = [0u8; NVAPI_SHORT_STRING_MAX];
            if unsafe { ver_fn(&mut ver, branch.as_mut_ptr()) } == NVAPI_OK {
                format_nv_driver_version(ver)
            } else {
                "Unknown".into()
            }
        } else {
            "Unknown".into()
        };

        let mut out = Vec::new();
        for (idx, &handle) in rt.gpu_handles.iter().enumerate() {
            let mut name = self.gpu_name.clone();
            if let Some(name_fn) = rt.fn_gpu_full_name {
                let mut buf = [0u8; NVAPI_SHORT_STRING_MAX];
                if unsafe { name_fn(handle, buf.as_mut_ptr()) } == NVAPI_OK {
                    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                    let s = String::from_utf8_lossy(&buf[..nul]).trim().to_string();
                    if !s.is_empty() {
                        name = s;
                    }
                }
            }
            out.push(GpuAdapterInfo {
                id: GpuId(format!("nvidia:{idx}")),
                vendor: Vendor::Nvidia,
                name,
                role: if self.is_laptop {
                    GpuRole::Discrete
                } else {
                    GpuRole::Primary
                },
                driver_version: driver_version.clone(),
                driver_api: "nvapi64.dll + DRS".into(),
                dedicated_vram_mb: 0,
                pci_vendor_id: 0x10DE,
                pci_device_id: 0,
                luid_low: 0,
                luid_high: 0,
                capabilities: GpuCapabilities {
                    color: true,
                    wire_format: true,
                    scaling: true,
                    sharpness: false,
                    lace: false,
                    vrr: true,
                    mux: self.is_laptop,
                    spanning: true,
                    per_app_3d: true,
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
            .unwrap_or_default();

        let rt = self.runtime.lock().unwrap();
        if let Some(get_dvc) = rt.fn_get_dvc {
            if let Some(h_disp) = Self::display_handle(&rt, &display.device_name) {
                let mut dvc = NvDisplayDvcInfoEx::default();
                if unsafe { get_dvc(h_disp, 0, &mut dvc) } == NVAPI_OK {
                    let span = (dvc.max_level - dvc.min_level).max(1) as f32;
                    color.vibrance_or_saturation =
                        (((dvc.current_level - dvc.min_level) as f32) * 100.0 / span)
                            .clamp(0.0, 100.0);
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
        if let (Some(get_dvc), Some(set_dvc)) = (rt.fn_get_dvc, rt.fn_set_dvc) {
            if let Some(h_disp) = Self::display_handle(&rt, &display.device_name) {
                let mut dvc = NvDisplayDvcInfoEx::default();
                if unsafe { get_dvc(h_disp, 0, &mut dvc) } == NVAPI_OK {
                    let span = (dvc.max_level - dvc.min_level).max(1) as f32;
                    dvc.current_level = dvc.min_level
                        + ((settings.vibrance_or_saturation.clamp(0.0, 100.0) / 100.0) * span)
                            .round() as i32;
                    let _ = unsafe { set_dvc(h_disp, 0, &mut dvc) };
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
                "NVIDIA Mosaic/Surround requires at least 2 displays".into(),
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
        let hdr = *self.hdr_active.lock().unwrap();
        Ok(build_nvidia_3d_descriptors(
            &cache,
            &self.gpu_name,
            self.is_laptop,
            hdr,
        ))
    }

    fn apply_3d_setting(
        &self,
        _adapter: &GpuId,
        _scope: &SettingScope,
        setting_id: &str,
        value: &SettingValue,
    ) -> Result<(), BackendError> {
        if let Some(drs_id) = drs_id_for_setting(setting_id) {
            let raw = match value {
                SettingValue::Bool(b) => u32::from(*b),
                SettingValue::EnumId(id) => encode_drs_dword(setting_id, *id),
                SettingValue::Int(i) => *i as u32,
                SettingValue::Float(f) => *f as u32,
            };
            let rt = self.runtime.lock().unwrap();
            Self::write_drs_dword(&rt, drs_id, raw)?;
        }
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
        let rt = self.runtime.lock().unwrap();
        if let (Some(create), Some(destroy), Some(load), Some(restore), Some(save)) = (
            rt.fn_drs_create,
            rt.fn_drs_destroy,
            rt.fn_drs_load,
            rt.fn_drs_restore_all,
            rt.fn_drs_save,
        ) {
            let mut session: NvDrsSessionHandle = std::ptr::null_mut();
            if unsafe { create(&mut session) } == NVAPI_OK && !session.is_null() {
                unsafe {
                    if load(session) == NVAPI_OK && restore(session) == NVAPI_OK {
                        let _ = save(session);
                    }
                    let _ = destroy(session);
                }
            }
        }
        *self.settings_cache.lock().unwrap() = default_nvidia_values(&self.gpu_name);
        Ok(())
    }

    fn list_video_settings(&self, _adapter: &GpuId) -> Result<Vec<SettingDescriptor>, BackendError> {
        let cache = self.settings_cache.lock().unwrap();
        let hdr = *self.hdr_active.lock().unwrap();
        Ok(build_nvidia_video_descriptors(&cache, hdr))
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

    fn get_mux(&self, _adapter: &GpuId) -> Result<MuxMode, BackendError> {
        if !self.is_laptop {
            return Err(BackendError::Unsupported("MUX".into()));
        }
        Ok(*self.mux_cache.lock().unwrap())
    }

    fn set_mux(&self, _adapter: &GpuId, mode: MuxMode) -> Result<(), BackendError> {
        if !self.is_laptop {
            return Err(BackendError::Unsupported("MUX".into()));
        }
        *self.mux_cache.lock().unwrap() = mode;
        Ok(())
    }
}

fn encode_drs_dword(setting_id: &str, ui_id: u32) -> u32 {
    match setting_id {
        // Older presets stored 2 = "Ultra"; the driver has no separate Ultra key anymore.
        "nv.low_latency" => u32::from(ui_id != 0),
        "nv.vsync" => match ui_id {
            0 => 0x0841_6747, // VSYNCMODE_PASSIVE (Use 3D app setting)
            1 => 0x4781_4940, // VSYNCMODE_FORCEOFF
            2 => 0x4781_4941, // VSYNCMODE_FORCEON
            3 => 0x1888_8888, // VSYNCMODE_VIRTUAL (Fast)
            4 => 0x6092_5292, // VSYNCMODE_FLIPINTERVAL2 / Adaptive
            _ => 0x0841_6747,
        },
        _ => ui_id,
    }
}

fn decode_drs_dword(setting_id: &str, raw: u32) -> u32 {
    match setting_id {
        "nv.low_latency" => u32::from(raw != 0),
        "nv.vsync" => match raw {
            0x0841_6747 => 0,
            0x4781_4940 => 1,
            0x4781_4941 => 2,
            0x1888_8888 => 3,
            0x6092_5292 => 4,
            _ => 0,
        },
        _ => raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_latency_maps_to_prerender_limit() {
        assert_eq!(encode_drs_dword("nv.low_latency", 0), 0);
        assert_eq!(encode_drs_dword("nv.low_latency", 1), 1);
        assert_eq!(encode_drs_dword("nv.low_latency", 2), 1); // legacy "Ultra" preset
        assert_eq!(decode_drs_dword("nv.low_latency", 3), 1);
    }

    #[test]
    fn smoke_nvidia_nvapi_read_only() {
        match NvidiaBackend::try_load() {
            Ok(backend) => {
                let adapters = backend.enumerate_adapters().expect("NVAPI enumerate");
                assert!(!adapters.is_empty());
                let list_3d = backend
                    .list_3d_settings(&adapters[0].id, &SettingScope::Global)
                    .unwrap();
                assert!(list_3d.len() >= 29);
                let list_video = backend.list_video_settings(&adapters[0].id).unwrap();
                assert_eq!(list_video.len(), 3);
            }
            Err(BackendError::DllNotFound(_)) => {}
            Err(e) => panic!("Unexpected NVIDIA NVAPI error: {e}"),
        }
    }
}
