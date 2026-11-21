use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Vi,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    AppSubtitle,
    NavDashboard,
    NavColor,
    NavDisplay,
    NavGraphics,
    NavSettings,
    DashboardTitle,
    DashboardSubtitle,
    ColorTitle,
    ColorSubtitle,
    DisplayTitle,
    DisplaySubtitle,
    GraphicsTitle,
    GraphicsSubtitle,
    SettingsTitle,
    SettingsSubtitle,
    Refresh,
    ResetDefaults,
    GlobalScope,
    PerAppScope,
    ShowLegacy,
    HideLegacy,
    ThemeLabel,
    LanguageLabel,
    SavePreset,
    ApplyPreset,
    EnableSpanning,
    DisableSpanning,
    KeepChanges,
    RevertNow,
    NeedTwoDisplays,
    RelaunchAdmin,
}

pub fn tr(lang: Language, key: TextKey) -> &'static str {
    use Language::{En, Vi};
    use TextKey::*;
    match (lang, key) {
        (Vi, AppSubtitle) => "Điều khiển VGA thuần Rust · Không chạy nền",
        (En, AppSubtitle) => "Pure Rust GPU Control · Zero Background Overhead",
        (Vi, NavDashboard) => "Tổng quan",
        (En, NavDashboard) => "Dashboard",
        (Vi, NavColor) => "Màu & Độ sáng",
        (En, NavColor) => "Color & Brightness",
        (Vi, NavDisplay) => "Màn hình & MUX",
        (En, NavDisplay) => "Display & MUX",
        (Vi, NavGraphics) => "3D & Video",
        (En, NavGraphics) => "3D & Video",
        (Vi, NavSettings) => "Cài đặt",
        (En, NavSettings) => "Settings",
        (Vi, DashboardTitle) => "Tổng quan phần cứng",
        (En, DashboardTitle) => "Hardware Overview",
        (Vi, DashboardSubtitle) => "Thông tin phần cứng & driver tĩnh · Không telemetry, không chạy nền",
        (En, DashboardSubtitle) => "Static GPU & driver info only · Zero telemetry, zero polling",
        (Vi, ColorTitle) => "Màu sắc & Độ sáng",
        (En, ColorTitle) => "Color & Brightness",
        (Vi, ColorSubtitle) => "Chỉnh trực tiếp trên màn hình đang nối với GPU chủ quản",
        (En, ColorSubtitle) => "Adjust directly on the display owned by its active GPU driver",
        (Vi, DisplayTitle) => "Màn hình, Ghép màn hình & MUX",
        (En, DisplayTitle) => "Display, Spanning & MUX",
        (Vi, DisplaySubtitle) => "Độ phân giải, tần số quét, Scaling, VRR, Ghép màn hình và MUX",
        (En, DisplaySubtitle) => "Resolution, Refresh Rate, Scaling, VRR, Multi-Display Spanning & MUX",
        (Vi, GraphicsTitle) => "Cấu hình 3D & Video",
        (En, GraphicsTitle) => "3D & Video Settings",
        (Vi, GraphicsSubtitle) => "Bộ cấu hình gốc riêng cho từng hãng (NVAPI DRS / Intel IGCL / AMD ADLX)",
        (En, GraphicsSubtitle) => "Vendor-native driver configuration (NVAPI DRS / Intel IGCL / AMD ADLX)",
        (Vi, SettingsTitle) => "Cài đặt ứng dụng",
        (En, SettingsTitle) => "Application Settings",
        (Vi, SettingsSubtitle) => "Giao diện Sáng/Tối, song ngữ Anh/Việt và Preset JSON",
        (En, SettingsSubtitle) => "Dark/Light theme, Vietnamese/English language, and JSON presets",
        (Vi, Refresh) => "Làm mới",
        (En, Refresh) => "Refresh",
        (Vi, ResetDefaults) => "Khôi phục mặc định",
        (En, ResetDefaults) => "Restore Defaults",
        (Vi, GlobalScope) => "Toàn hệ thống (Global)",
        (En, GlobalScope) => "Global Settings",
        (Vi, PerAppScope) => "Theo ứng dụng (Per-App)",
        (En, PerAppScope) => "Per-App Profiles",
        (Vi, ShowLegacy) => "Hiện cài đặt cũ",
        (En, ShowLegacy) => "Show Legacy Settings",
        (Vi, HideLegacy) => "Ẩn cài đặt cũ",
        (En, HideLegacy) => "Hide Legacy Settings",
        (Vi, ThemeLabel) => "Chủ đề giao diện (Theme)",
        (En, ThemeLabel) => "Appearance Theme",
        (Vi, LanguageLabel) => "Ngôn ngữ / Language",
        (En, LanguageLabel) => "Language / Ngôn ngữ",
        (Vi, SavePreset) => "Lưu Preset",
        (En, SavePreset) => "Save Preset",
        (Vi, ApplyPreset) => "Áp dụng",
        (En, ApplyPreset) => "Apply",
        (Vi, EnableSpanning) => "Bật ghép màn hình",
        (En, EnableSpanning) => "Enable Spanning",
        (Vi, DisableSpanning) => "Tách màn hình",
        (En, DisableSpanning) => "Disable Spanning",
        (Vi, KeepChanges) => "Giữ thay đổi",
        (En, KeepChanges) => "Keep Changes",
        (Vi, RevertNow) => "Hoàn tác ngay",
        (En, RevertNow) => "Revert Now",
        (Vi, NeedTwoDisplays) => "Cần tối thiểu 2 màn hình nối trực tiếp vào cùng GPU",
        (En, NeedTwoDisplays) => "Requires at least 2 displays attached to the same GPU",
        (Vi, RelaunchAdmin) => "Chạy lại với quyền Administrator",
        (En, RelaunchAdmin) => "Relaunch as Administrator",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_keys_non_empty_in_both_languages() {
        let keys = [
            TextKey::AppSubtitle,
            TextKey::NavDashboard,
            TextKey::NavColor,
            TextKey::NavDisplay,
            TextKey::NavGraphics,
            TextKey::NavSettings,
            TextKey::DashboardTitle,
            TextKey::DashboardSubtitle,
            TextKey::ColorTitle,
            TextKey::ColorSubtitle,
            TextKey::DisplayTitle,
            TextKey::DisplaySubtitle,
            TextKey::GraphicsTitle,
            TextKey::GraphicsSubtitle,
            TextKey::SettingsTitle,
            TextKey::SettingsSubtitle,
            TextKey::Refresh,
            TextKey::ResetDefaults,
            TextKey::GlobalScope,
            TextKey::PerAppScope,
            TextKey::ShowLegacy,
            TextKey::HideLegacy,
            TextKey::ThemeLabel,
            TextKey::LanguageLabel,
            TextKey::SavePreset,
            TextKey::ApplyPreset,
            TextKey::EnableSpanning,
            TextKey::DisableSpanning,
            TextKey::KeepChanges,
            TextKey::RevertNow,
            TextKey::NeedTwoDisplays,
            TextKey::RelaunchAdmin,
        ];
        for k in keys {
            assert!(!tr(Language::Vi, k).trim().is_empty());
            assert!(!tr(Language::En, k).trim().is_empty());
        }
    }
}
