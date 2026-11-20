use crate::model::{
    ColorSettings, DisplayModeSettings, GpuAdapterInfo, GpuId, MuxMode, SettingDescriptor,
    SettingScope, SettingValue, SpanningConfig, Vendor,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("driver library not found: {0}")]
    DllNotFound(&'static str),
    #[error("feature not supported by active GPU/driver: {0}")]
    Unsupported(String),
    #[error("administrator privileges required for: {0}")]
    AccessDenied(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("driver call failed (code {code}): {message}")]
    DriverError { code: i32, message: String },
}

/// Unified GPU driver backend contract (zero telemetry, zero shell spawning).
pub trait GpuBackend: Send + Sync {
    fn vendor(&self) -> Vendor;

    fn enumerate_adapters(&self) -> Result<Vec<GpuAdapterInfo>, BackendError>;

    fn get_color(&self, adapter: &GpuId, display_id: &str) -> Result<ColorSettings, BackendError>;

    fn set_color(
        &self,
        adapter: &GpuId,
        display_id: &str,
        settings: &ColorSettings,
    ) -> Result<(), BackendError>;

    fn get_display(
        &self,
        adapter: &GpuId,
        display_id: &str,
    ) -> Result<DisplayModeSettings, BackendError>;

    fn set_display(
        &self,
        adapter: &GpuId,
        display_id: &str,
        settings: &DisplayModeSettings,
    ) -> Result<(), BackendError>;

    fn get_spanning(&self, adapter: &GpuId) -> Result<SpanningConfig, BackendError>;

    fn apply_spanning(
        &self,
        adapter: &GpuId,
        config: &SpanningConfig,
    ) -> Result<(), BackendError>;

    fn disable_spanning(&self, adapter: &GpuId) -> Result<(), BackendError>;

    fn list_3d_settings(
        &self,
        adapter: &GpuId,
        scope: &SettingScope,
    ) -> Result<Vec<SettingDescriptor>, BackendError>;

    fn apply_3d_setting(
        &self,
        adapter: &GpuId,
        scope: &SettingScope,
        setting_id: &str,
        value: &SettingValue,
    ) -> Result<(), BackendError>;

    fn restore_3d_defaults(
        &self,
        adapter: &GpuId,
        scope: &SettingScope,
    ) -> Result<(), BackendError>;

    fn list_video_settings(&self, adapter: &GpuId) -> Result<Vec<SettingDescriptor>, BackendError>;

    fn apply_video_setting(
        &self,
        adapter: &GpuId,
        setting_id: &str,
        value: &SettingValue,
    ) -> Result<(), BackendError>;

    fn get_mux(&self, adapter: &GpuId) -> Result<MuxMode, BackendError> {
        let _ = adapter;
        Err(BackendError::Unsupported("MUX".into()))
    }

    fn set_mux(&self, adapter: &GpuId, mode: MuxMode) -> Result<(), BackendError> {
        let _ = (adapter, mode);
        Err(BackendError::Unsupported("MUX".into()))
    }
}
