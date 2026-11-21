use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Vendor {
    Intel,
    Nvidia,
    Amd,
}

impl Vendor {
    pub fn from_pci_vendor_id(vendor_id: u32) -> Option<Self> {
        match vendor_id {
            0x8086 => Some(Self::Intel),
            0x10DE => Some(Self::Nvidia),
            0x1002 | 0x1022 => Some(Self::Amd),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Intel => "Intel",
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GpuId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuRole {
    Integrated,
    Discrete,
    Primary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuCapabilities {
    pub color: bool,
    pub wire_format: bool,
    pub scaling: bool,
    pub sharpness: bool,
    pub lace: bool,
    pub vrr: bool,
    pub mux: bool,
    pub spanning: bool,
    pub per_app_3d: bool,
    pub video_processing: bool,
}

/// Static hardware & driver metadata read on demand (no telemetry / sensor polling).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdapterInfo {
    pub id: GpuId,
    pub vendor: Vendor,
    pub name: String,
    pub role: GpuRole,
    pub driver_version: String,
    pub driver_api: String,
    pub dedicated_vram_mb: u64,
    pub pci_vendor_id: u32,
    pub pci_device_id: u32,
    pub luid_low: u32,
    pub luid_high: i32,
    pub capabilities: GpuCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayOutputInfo {
    pub id: String,
    pub device_name: String,
    /// `DISPLAYCONFIG_PATH_TARGET_INFO.id`, matches Intel IGCL's Windows display encoder id.
    #[serde(default)]
    pub target_id: u32,
    pub friendly_name: String,
    pub adapter_luid_low: u32,
    pub adapter_luid_high: i32,
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub bits_per_color: u32,
    pub hdr_active: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorDepth {
    Bpc6,
    Bpc8,
    Bpc10,
    Bpc12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputColorFormat {
    Rgb,
    YCbCr444,
    YCbCr422,
    YCbCr420,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynamicRange {
    Full,
    Limited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFormat {
    pub depth: ColorDepth,
    pub format: OutputColorFormat,
    pub dynamic_range: DynamicRange,
}

impl Default for WireFormat {
    fn default() -> Self {
        Self {
            depth: ColorDepth::Bpc8,
            format: OutputColorFormat::Rgb,
            dynamic_range: DynamicRange::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorSettings {
    /// 0.0 ..= 100.0 (50.0 default)
    pub brightness: f32,
    /// 0.0 ..= 100.0 (50.0 default)
    pub contrast: f32,
    /// 0.5 ..= 2.5 (1.0 default)
    pub gamma: f32,
    /// Digital Vibrance (NVIDIA) / Saturation (Intel & AMD), 0.0 ..= 100.0 (50.0 default)
    pub vibrance_or_saturation: f32,
    /// -180.0 ..= 180.0 (0.0 default)
    pub hue: f32,
    /// Optional Intel LACE / Dynamic Contrast Enhancement
    pub lace_enabled: Option<bool>,
    /// Optional AMD Color Temperature in Kelvin (4000..=10000)
    pub color_temp_kelvin: Option<u32>,
    pub wire_format: WireFormat,
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            brightness: 50.0,
            contrast: 50.0,
            gamma: 1.0,
            vibrance_or_saturation: 50.0,
            hue: 0.0,
            lace_enabled: None,
            color_temp_kelvin: None,
            wire_format: WireFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingMode {
    AspectRatio,
    FullScreen,
    Center,
    Integer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayModeSettings {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub scaling: ScalingMode,
    pub vrr_enabled: bool,
    /// Optional Intel IGCL sharpness (0..=100)
    pub sharpness: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuxMode {
    Automatic,
    HybridOptimus,
    DiscreteGpuOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanningLayout {
    Horizontal1x2,
    Vertical2x1,
    Horizontal1x3,
    Grid2x2,
    Custom { rows: u32, cols: u32 },
}

impl SpanningLayout {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Horizontal1x2 => (1, 2),
            Self::Vertical2x1 => (2, 1),
            Self::Horizontal1x3 => (1, 3),
            Self::Grid2x2 => (2, 2),
            Self::Custom { rows, cols } => (rows.max(1), cols.max(1)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BezelOffset {
    pub horizontal_px: u32,
    pub vertical_px: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanningConfig {
    pub enabled: bool,
    pub layout: SpanningLayout,
    pub display_ids: Vec<String>,
    pub bezel: BezelOffset,
}

impl SpanningConfig {
    pub fn total_resolution(&self, base_w: u32, base_h: u32) -> (u32, u32) {
        let (rows, cols) = self.layout.dimensions();
        let total_w = cols
            .saturating_mul(base_w)
            .saturating_add(cols.saturating_sub(1).saturating_mul(self.bezel.horizontal_px));
        let total_h = rows
            .saturating_mul(base_h)
            .saturating_add(rows.saturating_sub(1).saturating_mul(self.bezel.vertical_px));
        (total_w, total_h)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettingScope {
    Global,
    PerApp(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingOption {
    pub id: u32,
    pub label_vi: String,
    pub label_en: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettingControl {
    Toggle,
    Enum(Vec<SettingOption>),
    Slider {
        min: f32,
        max: f32,
        step: f32,
        unit: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettingValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    EnumId(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingLockReason {
    pub vi: String,
    pub en: String,
}

/// Vendor-native capability-driven descriptor for 3D and Video settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingDescriptor {
    pub id: String,
    pub section: String,
    pub is_legacy: bool,
    pub label_vi: String,
    pub label_en: String,
    pub desc_vi: String,
    pub desc_en: String,
    pub control: SettingControl,
    pub value: SettingValue,
    pub default_value: SettingValue,
    pub supported: bool,
    pub lock_reason: Option<SettingLockReason>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_pci_vendor_ids() {
        assert_eq!(Vendor::from_pci_vendor_id(0x8086), Some(Vendor::Intel));
        assert_eq!(Vendor::from_pci_vendor_id(0x10DE), Some(Vendor::Nvidia));
        assert_eq!(Vendor::from_pci_vendor_id(0x1002), Some(Vendor::Amd));
        assert_eq!(Vendor::from_pci_vendor_id(0x1414), None);
    }

    #[test]
    fn calculates_spanning_resolution_with_bezel() {
        let cfg = SpanningConfig {
            enabled: true,
            layout: SpanningLayout::Horizontal1x2,
            display_ids: vec!["d1".into(), "d2".into()],
            bezel: BezelOffset {
                horizontal_px: 40,
                vertical_px: 20,
            },
        };
        assert_eq!(cfg.total_resolution(2560, 1440), (5160, 1440));

        let grid = SpanningConfig {
            enabled: true,
            layout: SpanningLayout::Grid2x2,
            display_ids: vec!["d1".into(), "d2".into(), "d3".into(), "d4".into()],
            bezel: BezelOffset {
                horizontal_px: 30,
                vertical_px: 20,
            },
        };
        assert_eq!(grid.total_resolution(1920, 1080), (3870, 2180));
    }
}
