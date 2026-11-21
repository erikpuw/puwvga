use crate::state::AppState;
use crate::theme::Palette;
use crate::widgets::{section_header, segmented_control, setting_card};
use egui::{RichText, Ui};
use puwvga_core::{tr, Language, TextKey, ThemePreference};

pub fn render(ui: &mut Ui, state: &mut AppState, p: Palette) {
    let lang = state.language;
    let is_vi = lang == Language::Vi;

    ui.label(
        RichText::new(tr(lang, TextKey::SettingsTitle))
            .size(21.0)
            .strong()
            .color(p.text_primary),
    );
    ui.label(
        RichText::new(tr(lang, TextKey::SettingsSubtitle))
            .size(12.5)
            .color(p.text_secondary),
    );
    ui.add_space(10.0);

    section_header(
        ui,
        p,
        if is_vi {
            "Giao diện & Ngôn ngữ"
        } else {
            "Appearance & Language"
        },
    );

    let mut theme_changed = false;
    setting_card(
        ui,
        p,
        tr(lang, TextKey::ThemeLabel),
        if is_vi {
            "Đồng bộ Windows 11 Fluent (Theo hệ thống / Tối / Sáng)"
        } else {
            "Windows 11 Fluent palette (System / Dark / Light)"
        },
        None,
        |ui| {
            if segmented_control(
                ui,
                p,
                &mut state.theme,
                &[
                    (ThemePreference::System, "System"),
                    (ThemePreference::Dark, "Dark"),
                    (ThemePreference::Light, "Light"),
                ],
            ) {
                theme_changed = true;
            }
        },
    );

    let mut lang_changed = false;
    setting_card(
        ui,
        p,
        tr(lang, TextKey::LanguageLabel),
        if is_vi {
            "Chuyển đổi tức thì giữa Tiếng Việt và English"
        } else {
            "Instant switch between Vietnamese and English"
        },
        None,
        |ui| {
            if segmented_control(
                ui,
                p,
                &mut state.language,
                &[(Language::Vi, "Tiếng Việt"), (Language::En, "English")],
            ) {
                lang_changed = true;
            }
        },
    );

    if theme_changed || lang_changed {
        state.auto_save_preset();
    }

    section_header(
        ui,
        p,
        if is_vi {
            "Preset cục bộ (Portable JSON)"
        } else {
            "Local Presets (Portable JSON)"
        },
    );

    let preset_desc = format!("{}", state.preset_path.display());
    let mut do_save = false;
    let mut do_apply = false;
    setting_card(
        ui,
        p,
        if is_vi {
            "Lưu / Áp dụng Preset"
        } else {
            "Save / Apply Preset"
        },
        &preset_desc,
        None,
        |ui| {
            if ui.button(tr(lang, TextKey::ApplyPreset)).clicked() {
                do_apply = true;
            }
            if ui.button(tr(lang, TextKey::SavePreset)).clicked() {
                do_save = true;
            }
        },
    );
    if do_save {
        state.save_preset();
    }
    if do_apply {
        state.apply_saved_preset();
    }

    section_header(ui, p, if is_vi { "Thông tin" } else { "About" });
    setting_card(
        ui,
        p,
        concat!("puwvga v", env!("CARGO_PKG_VERSION")),
        if is_vi {
            "Pure Rust · Static VC++ CRT · Không WebView · Không Telemetry · Không chạy nền"
        } else {
            "Pure Rust · Static VC++ CRT · Zero WebView · Zero Telemetry · Zero Background Overhead"
        },
        None,
        |ui| {
            ui.label(RichText::new("Portable").size(12.0).color(p.accent));
        },
    );
}
