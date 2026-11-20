use puwvga_amd::AmdBackend;
use puwvga_core::{
    BackendError, ColorSettings, DisplayModeSettings, DisplayOutputInfo, GpuAdapterInfo,
    GpuBackend, GpuCapabilities, GpuId, GpuRole, Language, MuxMode, PresetBundle, PresetStore,
    ScalingMode, SettingDescriptor, SettingScope, SpanningConfig, SpanningLayout,
    ThemePreference, Vendor,
};
use puwvga_intel::IntelBackend;
use puwvga_nvidia::NvidiaBackend;
use puwvga_win32::{
    apply_display_mode, enumerate_active_displays, enumerate_dxgi_adapters, is_windows_dark_mode,
    list_supported_modes,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Color,
    Display,
    Graphics,
    Settings,
}

#[derive(Debug, Clone)]
pub struct PendingDisplayRevert {
    pub deadline: Instant,
    pub kind: RevertKind,
}

#[derive(Debug, Clone)]
pub enum RevertKind {
    Mode {
        device_name: String,
        prev_width: u32,
        prev_height: u32,
        prev_hz: u32,
    },
    Spanning {
        gpu_id: GpuId,
    },
}

pub struct AppState {
    pub page: Page,
    pub language: Language,
    pub theme: ThemePreference,
    pub system_dark: bool,
    pub preset_path: PathBuf,

    pub adapters: Vec<GpuAdapterInfo>,
    pub displays: Vec<DisplayOutputInfo>,
    pub selected_gpu_idx: usize,
    pub selected_display_idx: usize,

    pub backends: BTreeMap<Vendor, Arc<dyn GpuBackend>>,

    // Cached per-GPU & per-display editable state
    pub color_by_display: BTreeMap<String, ColorSettings>,
    pub modes_by_device: BTreeMap<String, Vec<(u32, u32, u32)>>,
    pub display_settings_by_id: BTreeMap<String, DisplayModeSettings>,
    pub spanning_by_gpu: BTreeMap<GpuId, SpanningConfig>,
    pub mux_mode: MuxMode,

    // Vendor-native 3D & Video descriptors for selected GPU
    pub gfx_scope: SettingScope,
    pub per_app_name: String,
    pub show_legacy_nv: bool,
    pub settings_3d: Vec<SettingDescriptor>,
    pub settings_video: Vec<SettingDescriptor>,

    // Safety & feedback
    pub pending_revert: Option<PendingDisplayRevert>,
    pub toast: Option<(String, Instant)>,
    pub needs_admin_banner: bool,
}

impl AppState {
    pub fn new() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let appdata_dir = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| exe_dir.clone());
        let preset_path = PresetStore::resolve_path(&exe_dir, &appdata_dir);

        let loaded = PresetStore::load(&preset_path).unwrap_or_default();

        let mut state = Self {
            page: Page::Dashboard,
            language: loaded.language,
            theme: loaded.theme,
            system_dark: is_windows_dark_mode(),
            preset_path,
            adapters: Vec::new(),
            displays: Vec::new(),
            selected_gpu_idx: 0,
            selected_display_idx: 0,
            backends: BTreeMap::new(),
            color_by_display: loaded.display_colors,
            modes_by_device: BTreeMap::new(),
            display_settings_by_id: BTreeMap::new(),
            spanning_by_gpu: BTreeMap::new(),
            mux_mode: MuxMode::HybridOptimus,
            gfx_scope: SettingScope::Global,
            per_app_name: "cs2.exe".into(),
            show_legacy_nv: false,
            settings_3d: Vec::new(),
            settings_video: Vec::new(),
            pending_revert: None,
            toast: None,
            needs_admin_banner: false,
        };

        state.init_backends();
        state.refresh_hardware();
        state
    }

    fn init_backends(&mut self) {
        if let Ok(intel) = IntelBackend::try_load() {
            self.backends.insert(Vendor::Intel, Arc::new(intel));
        }
        if let Ok(nv) = NvidiaBackend::try_load() {
            self.backends.insert(Vendor::Nvidia, Arc::new(nv));
        }
        if let Ok(amd) = AmdBackend::try_load() {
            self.backends.insert(Vendor::Amd, Arc::new(amd));
        }
    }

    /// Reads static GPU & display topology once on startup or explicit refresh (zero background polling).
    pub fn refresh_hardware(&mut self) {
        self.system_dark = is_windows_dark_mode();
        self.displays = enumerate_active_displays().unwrap_or_default();

        let dxgi_list = enumerate_dxgi_adapters().unwrap_or_default();
        let multi_gpu = dxgi_list.len() > 1;

        let mut adapters = Vec::new();
        for (idx, d) in dxgi_list.iter().enumerate() {
            let vendor_info = self
                .backends
                .get(&d.vendor)
                .and_then(|b| b.enumerate_adapters().ok())
                .and_then(|v| v.into_iter().next());

            let role = if !multi_gpu {
                GpuRole::Primary
            } else if d.dedicated_vram_mb >= 2048 || d.vendor == Vendor::Nvidia {
                GpuRole::Discrete
            } else {
                GpuRole::Integrated
            };

            let (driver_version, driver_api, capabilities) = if let Some(vi) = vendor_info {
                (vi.driver_version, vi.driver_api, vi.capabilities)
            } else {
                (
                    "Windows DCH".into(),
                    match d.vendor {
                        Vendor::Intel => "DXGI (ControlLib unavailable)".into(),
                        Vendor::Nvidia => "DXGI (nvapi64 unavailable)".into(),
                        Vendor::Amd => "DXGI (atiadlxx unavailable)".into(),
                    },
                    GpuCapabilities {
                        color: true,
                        ..Default::default()
                    },
                )
            };

            adapters.push(GpuAdapterInfo {
                id: GpuId(format!("{}:{idx}", d.vendor.display_name().to_lowercase())),
                vendor: d.vendor,
                name: d.name.clone(),
                role,
                driver_version,
                driver_api,
                dedicated_vram_mb: d.dedicated_vram_mb,
                pci_vendor_id: d.pci_vendor_id,
                pci_device_id: d.pci_device_id,
                luid_low: d.luid_low,
                luid_high: d.luid_high,
                capabilities,
            });
        }

        self.adapters = adapters;
        if self.selected_gpu_idx >= self.adapters.len() {
            self.selected_gpu_idx = 0;
        }
        if self.selected_display_idx >= self.displays.len() {
            self.selected_display_idx = 0;
        }

        // Populate modes & initial color/display states
        for disp in &self.displays {
            let modes = list_supported_modes(&disp.device_name);
            self.modes_by_device.insert(disp.device_name.clone(), modes);
            self.display_settings_by_id
                .entry(disp.id.clone())
                .or_insert(DisplayModeSettings {
                    width: disp.width,
                    height: disp.height,
                    refresh_hz: disp.refresh_hz,
                    scaling: ScalingMode::AspectRatio,
                    vrr_enabled: true,
                    sharpness: Some(20),
                });

            if !self.color_by_display.contains_key(&disp.id) {
                if let Some(gpu) = self.owner_gpu_for_display(disp) {
                    if let Some(backend) = self.backends.get(&gpu.vendor) {
                        if let Ok(c) = backend.get_color(&gpu.id, &disp.id) {
                            self.color_by_display.insert(disp.id.clone(), c);
                            continue;
                        }
                    }
                }
                self.color_by_display
                    .insert(disp.id.clone(), ColorSettings::default());
            }
        }

        for gpu in &self.adapters {
            let attached: Vec<String> = self
                .displays_for_gpu(gpu)
                .into_iter()
                .map(|d| d.id.clone())
                .collect();
            self.spanning_by_gpu
                .entry(gpu.id.clone())
                .or_insert(SpanningConfig {
                    enabled: false,
                    layout: SpanningLayout::Horizontal1x2,
                    display_ids: attached,
                    bezel: Default::default(),
                });
        }

        self.reload_active_graphics_settings();
    }

    pub fn is_dark(&self) -> bool {
        match self.theme {
            ThemePreference::System => self.system_dark,
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
        }
    }

    pub fn active_gpu(&self) -> Option<&GpuAdapterInfo> {
        self.adapters.get(self.selected_gpu_idx)
    }

    pub fn owner_gpu_for_display<'a>(
        &'a self,
        disp: &DisplayOutputInfo,
    ) -> Option<&'a GpuAdapterInfo> {
        self.adapters
            .iter()
            .find(|g| g.luid_low == disp.adapter_luid_low && g.luid_high == disp.adapter_luid_high)
            .or_else(|| self.adapters.first())
    }

    pub fn displays_for_gpu<'a>(&'a self, gpu: &GpuAdapterInfo) -> Vec<&'a DisplayOutputInfo> {
        self.displays
            .iter()
            .filter(|d| d.adapter_luid_low == gpu.luid_low && d.adapter_luid_high == gpu.luid_high)
            .collect()
    }

    pub fn select_gpu(&mut self, idx: usize) {
        if idx < self.adapters.len() && idx != self.selected_gpu_idx {
            self.selected_gpu_idx = idx;
            self.reload_active_graphics_settings();
        }
    }

    pub fn reload_active_graphics_settings(&mut self) {
        self.settings_3d.clear();
        self.settings_video.clear();
        let Some(gpu) = self.active_gpu().cloned() else {
            return;
        };
        if let Some(backend) = self.backends.get(&gpu.vendor) {
            if let Ok(s3d) = backend.list_3d_settings(&gpu.id, &self.gfx_scope) {
                self.settings_3d = s3d;
            }
            if let Ok(svid) = backend.list_video_settings(&gpu.id) {
                self.settings_video = svid;
            }
        }
    }

    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now() + Duration::from_secs(2)));
    }

    pub fn handle_backend_result(&mut self, res: Result<(), BackendError>, ok_msg: &str) {
        match res {
            Ok(()) => self.set_toast(ok_msg),
            Err(BackendError::AccessDenied(what)) => {
                self.needs_admin_banner = true;
                self.set_toast(format!("Cần quyền Administrator: {what}"));
            }
            Err(e) => self.set_toast(format!("{e}")),
        }
    }

    pub fn apply_mode_with_revert(
        &mut self,
        device_name: &str,
        prev: (u32, u32, u32),
        next: (u32, u32, u32),
    ) {
        let res = apply_display_mode(device_name, next.0, next.1, next.2);
        match res {
            Ok(()) => {
                self.pending_revert = Some(PendingDisplayRevert {
                    deadline: Instant::now() + Duration::from_secs(15),
                    kind: RevertKind::Mode {
                        device_name: device_name.to_string(),
                        prev_width: prev.0,
                        prev_height: prev.1,
                        prev_hz: prev.2,
                    },
                });
                self.refresh_hardware();
            }
            Err(e) => self.set_toast(format!("{e}")),
        }
    }

    pub fn apply_spanning_with_revert(&mut self, gpu_id: GpuId, vendor: Vendor, cfg: SpanningConfig) {
        if let Some(backend) = self.backends.get(&vendor).cloned() {
            let res = backend.apply_spanning(&gpu_id, &cfg);
            if res.is_ok() {
                self.spanning_by_gpu.insert(gpu_id.clone(), cfg);
                self.pending_revert = Some(PendingDisplayRevert {
                    deadline: Instant::now() + Duration::from_secs(15),
                    kind: RevertKind::Spanning { gpu_id },
                });
            }
            self.handle_backend_result(res, "Đã bật ghép màn hình (tự hoàn tác sau 15s nếu chưa xác nhận)");
        }
    }

    pub fn tick_revert(&mut self) {
        let expired = self
            .pending_revert
            .as_ref()
            .is_some_and(|p| Instant::now() >= p.deadline);
        if expired {
            self.revert_pending();
        }
    }

    pub fn keep_pending(&mut self) {
        self.pending_revert = None;
        self.set_toast(match self.language {
            Language::Vi => "Đã giữ cấu hình hiển thị mới",
            Language::En => "Display configuration kept",
        });
    }

    pub fn revert_pending(&mut self) {
        let Some(pending) = self.pending_revert.take() else {
            return;
        };
        match pending.kind {
            RevertKind::Mode {
                device_name,
                prev_width,
                prev_height,
                prev_hz,
            } => {
                let _ = apply_display_mode(&device_name, prev_width, prev_height, prev_hz);
                self.refresh_hardware();
            }
            RevertKind::Spanning { gpu_id } => {
                if let Some(gpu) = self.adapters.iter().find(|g| g.id == gpu_id) {
                    if let Some(backend) = self.backends.get(&gpu.vendor) {
                        let _ = backend.disable_spanning(&gpu_id);
                    }
                }
                if let Some(cfg) = self.spanning_by_gpu.get_mut(&gpu_id) {
                    cfg.enabled = false;
                }
            }
        }
        self.set_toast(match self.language {
            Language::Vi => "Đã hoàn tác về cấu hình hiển thị trước đó",
            Language::En => "Reverted to previous display configuration",
        });
    }

    pub fn save_preset(&mut self) {
        let bundle = PresetBundle {
            language: self.language,
            theme: self.theme,
            display_colors: self.color_by_display.clone(),
        };
        match PresetStore::save(&self.preset_path, &bundle) {
            Ok(()) => self.set_toast(format!("Đã lưu {}", self.preset_path.display())),
            Err(e) => self.set_toast(format!("Lỗi lưu preset: {e}")),
        }
    }

    pub fn apply_saved_preset(&mut self) {
        match PresetStore::load(&self.preset_path) {
            Ok(bundle) => {
                self.language = bundle.language;
                self.theme = bundle.theme;
                for (disp_id, color) in bundle.display_colors {
                    if let Some(disp) = self.displays.iter().find(|d| d.id == disp_id).cloned() {
                        if let Some(gpu) = self.owner_gpu_for_display(&disp).cloned() {
                            if let Some(backend) = self.backends.get(&gpu.vendor) {
                                let _ = backend.set_color(&gpu.id, &disp_id, &color);
                            }
                        }
                    }
                    self.color_by_display.insert(disp_id, color);
                }
                self.set_toast(match self.language {
                    Language::Vi => "Đã áp dụng Preset từ file JSON",
                    Language::En => "Applied Preset from JSON file",
                });
            }
            Err(e) => self.set_toast(format!("Chưa có file preset: {e}")),
        }
    }
}
