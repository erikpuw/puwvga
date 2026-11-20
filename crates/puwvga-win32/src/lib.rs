pub mod display_ccd;
pub mod dxgi;
pub mod gamma_ramp;
pub mod theme;

pub use display_ccd::{
    apply_display_mode, dedup_and_sort_modes, enumerate_active_displays, list_supported_modes,
};
pub use dxgi::{enumerate_dxgi_adapters, DxgiAdapter};
pub use gamma_ramp::{apply_gamma_ramp, build_gamma_ramp, read_gamma_ramp_estimate};
pub use theme::{apply_dwm_dark_titlebar, is_windows_dark_mode, relaunch_as_administrator};
