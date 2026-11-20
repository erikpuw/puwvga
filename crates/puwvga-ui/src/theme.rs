use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Visuals};
use puwvga_core::Vendor;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg_window: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_control: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_fg: Color32,
    pub accent_soft: Color32,
    pub warn: Color32,
    pub ok: Color32,
}

impl Palette {
    pub fn for_mode(dark: bool) -> Self {
        if dark {
            Self {
                dark: true,
                bg_window: Color32::from_rgb(0x1C, 0x1C, 0x1E),
                bg_sidebar: Color32::from_rgb(0x18, 0x18, 0x1A),
                bg_card: Color32::from_rgb(0x25, 0x25, 0x28),
                bg_card_hover: Color32::from_rgb(0x2C, 0x2C, 0x30),
                bg_control: Color32::from_rgb(0x32, 0x32, 0x36),
                border_subtle: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                border_strong: Color32::from_rgba_unmultiplied(255, 255, 255, 36),
                text_primary: Color32::from_rgb(0xF3, 0xF3, 0xF5),
                text_secondary: Color32::from_rgb(0xA1, 0xA1, 0xAA),
                text_muted: Color32::from_rgb(0x71, 0x71, 0x7A),
                accent: Color32::from_rgb(0x4C, 0xC2, 0xFF),
                accent_fg: Color32::from_rgb(0x0B, 0x14, 0x1A),
                accent_soft: Color32::from_rgba_unmultiplied(0x4C, 0xC2, 0xFF, 34),
                warn: Color32::from_rgb(0xFB, 0xBF, 0x24),
                ok: Color32::from_rgb(0x34, 0xD3, 0x99),
            }
        } else {
            Self {
                dark: false,
                bg_window: Color32::from_rgb(0xF3, 0xF3, 0xF3),
                bg_sidebar: Color32::from_rgb(0xEC, 0xEC, 0xEF),
                bg_card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                bg_card_hover: Color32::from_rgb(0xFA, 0xFA, 0xFA),
                bg_control: Color32::from_rgb(0xF0, 0xF0, 0xF3),
                border_subtle: Color32::from_rgba_unmultiplied(0, 0, 0, 20),
                border_strong: Color32::from_rgba_unmultiplied(0, 0, 0, 38),
                text_primary: Color32::from_rgb(0x18, 0x18, 0x1B),
                text_secondary: Color32::from_rgb(0x52, 0x52, 0x5B),
                text_muted: Color32::from_rgb(0x71, 0x71, 0x7A),
                accent: Color32::from_rgb(0x00, 0x67, 0xC0),
                accent_fg: Color32::WHITE,
                accent_soft: Color32::from_rgba_unmultiplied(0x00, 0x67, 0xC0, 26),
                warn: Color32::from_rgb(0xB4, 0x53, 0x09),
                ok: Color32::from_rgb(0x05, 0x96, 0x69),
            }
        }
    }
}

pub fn vendor_color(vendor: Vendor) -> Color32 {
    match vendor {
        Vendor::Nvidia => Color32::from_rgb(0x76, 0xB9, 0x00),
        Vendor::Intel => Color32::from_rgb(0x00, 0x71, 0xC5),
        Vendor::Amd => Color32::from_rgb(0xED, 0x1C, 0x24),
    }
}

/// Loads Windows `Segoe UI` so Vietnamese diacritics render cleanly like native Lenovo Legion Toolkit.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let candidates = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("segoe_ui".into(), FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "segoe_ui".into());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

pub fn apply_fluent_visuals(ctx: &egui::Context, p: Palette) {
    let mut visuals = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = p.bg_window;
    visuals.window_fill = p.bg_card;
    visuals.extreme_bg_color = p.bg_control;
    visuals.faint_bg_color = p.bg_card;
    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0, p.accent_fg);

    visuals.widgets.noninteractive.bg_fill = p.bg_card;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border_subtle);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = p.bg_control;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border_strong);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = p.bg_card_hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = p.accent_soft;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, p.text_primary);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    ctx.set_visuals(visuals);
}
