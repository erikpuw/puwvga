use crate::state::AppState;
use crate::theme::{vendor_color, Palette};
use crate::widgets::{section_header, segmented_control, setting_card, spring_toggle, vendor_dot};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use puwvga_core::{
    tr, Language, SettingControl, SettingDescriptor, SettingScope, SettingValue, TextKey, Vendor,
};

pub fn render(ui: &mut Ui, state: &mut AppState, p: Palette) {
    let lang = state.language;
    let is_vi = lang == Language::Vi;

    let Some(gpu) = state.active_gpu().cloned() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(tr(lang, TextKey::GraphicsTitle))
                    .size(21.0)
                    .strong()
                    .color(p.text_primary),
            );
            ui.label(
                RichText::new(tr(lang, TextKey::GraphicsSubtitle))
                    .size(12.5)
                    .color(p.text_secondary),
            );
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if gpu.capabilities.per_app_3d {
                let mut is_global = matches!(state.gfx_scope, SettingScope::Global);
                if segmented_control(
                    ui,
                    p,
                    &mut is_global,
                    &[
                        (true, tr(lang, TextKey::GlobalScope)),
                        (false, tr(lang, TextKey::PerAppScope)),
                    ],
                ) {
                    state.gfx_scope = if is_global {
                        SettingScope::Global
                    } else {
                        SettingScope::PerApp(state.per_app_name.clone())
                    };
                    state.reload_active_graphics_settings();
                }
            } else {
                ui.label(
                    RichText::new("IGCL · ctlGetSupported3DCapabilities")
                        .size(11.5)
                        .color(p.text_secondary),
                );
            }
        });
    });

    ui.add_space(10.0);

    // Inline GPU switcher when machine has >1 GPU
    if state.adapters.len() > 1 {
        let adapters = state.adapters.clone();
        ui.horizontal_wrapped(|ui| {
            for (idx, g) in adapters.iter().enumerate() {
                let active = idx == state.selected_gpu_idx;
                let resp = Frame::new()
                    .fill(if active { p.accent_soft } else { p.bg_card })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if active { p.accent } else { p.border_subtle },
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                vendor_dot(ui, vendor_color(g.vendor));
                                ui.label(
                                    RichText::new(&g.name)
                                        .size(13.0)
                                        .strong()
                                        .color(p.text_primary),
                                );
                            });
                            ui.label(
                                RichText::new(&g.driver_api)
                                    .size(11.5)
                                    .color(p.text_secondary),
                            );
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    state.select_gpu(idx);
                }
            }
        });
        ui.add_space(6.0);
    }

    if gpu.capabilities.per_app_3d && matches!(state.gfx_scope, SettingScope::PerApp(_)) {
        setting_card(
            ui,
            p,
            if is_vi {
                "Ứng dụng mục tiêu (.exe)"
            } else {
                "Target Application (.exe)"
            },
            if is_vi {
                "Ghi đè profile 3D trực tiếp vào driver (không phụ thuộc whitelist hãng)"
            } else {
                "Override 3D profile directly in driver DB (no app whitelist restriction)"
            },
            None,
            |ui| {
                if ui.text_edit_singleline(&mut state.per_app_name).lost_focus() {
                    state.gfx_scope = SettingScope::PerApp(state.per_app_name.clone());
                    state.reload_active_graphics_settings();
                }
            },
        );
    }

    // Header + Restore Defaults button
    let main_section = state
        .settings_3d
        .first()
        .map(|s| s.section.clone())
        .unwrap_or_else(|| format!("{} 3D Settings", gpu.vendor.display_name()));

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(main_section.to_uppercase())
                .size(11.5)
                .strong()
                .color(p.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr(lang, TextKey::ResetDefaults)).clicked() {
                if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
                    let res = b.restore_3d_defaults(&gpu.id, &state.gfx_scope);
                    state.handle_backend_result(res, "Đã khôi phục mặc định 3D");
                    state.reload_active_graphics_settings();
                }
            }
        });
    });
    ui.add_space(6.0);

    // Main (non-legacy) 3D settings
    let mut updates_3d: Vec<(String, SettingValue)> = Vec::new();
    for item in state.settings_3d.iter_mut().filter(|s| !s.is_legacy) {
        if let Some(new_val) = render_descriptor_card(ui, p, lang, item) {
            updates_3d.push((item.id.clone(), new_val));
        }
    }

    // Collapsible Legacy Settings (NVIDIA DRS)
    let legacy_count = state.settings_3d.iter().filter(|s| s.is_legacy).count();
    if legacy_count > 0 {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            section_header(ui, p, "Legacy Settings (NVIDIA DRS)");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn_label = if state.show_legacy_nv {
                    tr(lang, TextKey::HideLegacy).to_string()
                } else {
                    format!("{} ({legacy_count})", tr(lang, TextKey::ShowLegacy))
                };
                if ui.button(btn_label).clicked() {
                    state.show_legacy_nv = !state.show_legacy_nv;
                }
            });
        });

        if state.show_legacy_nv {
            for item in state.settings_3d.iter_mut().filter(|s| s.is_legacy) {
                if let Some(new_val) = render_descriptor_card(ui, p, lang, item) {
                    updates_3d.push((item.id.clone(), new_val));
                }
            }
        }
    }

    for (id, val) in updates_3d {
        if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
            let res = b.apply_3d_setting(&gpu.id, &state.gfx_scope, &id, &val);
            state.handle_backend_result(
                res,
                if is_vi {
                    "Đã áp dụng trực tiếp vào driver"
                } else {
                    "Applied directly to driver"
                },
            );
        }
    }

    // Video section
    let video_section = state
        .settings_video
        .first()
        .map(|s| s.section.clone())
        .unwrap_or_else(|| match gpu.vendor {
            Vendor::Nvidia => "NVIDIA Video (nvapi64.dll)".into(),
            Vendor::Intel => "Intel Video Processing (IGCL)".into(),
            Vendor::Amd => "AMD Video (ADLX)".into(),
        });
    section_header(ui, p, &video_section);

    let mut updates_vid: Vec<(String, SettingValue)> = Vec::new();
    for item in state.settings_video.iter_mut() {
        if let Some(new_val) = render_descriptor_card(ui, p, lang, item) {
            updates_vid.push((item.id.clone(), new_val));
        }
    }
    for (id, val) in updates_vid {
        if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
            let res = b.apply_video_setting(&gpu.id, &id, &val);
            state.handle_backend_result(
                res,
                if is_vi {
                    "Đã áp dụng cấu hình Video"
                } else {
                    "Applied video setting"
                },
            );
        }
    }
}

fn render_descriptor_card(
    ui: &mut Ui,
    p: Palette,
    lang: Language,
    item: &mut SettingDescriptor,
) -> Option<SettingValue> {
    let label = if lang == Language::Vi {
        &item.label_vi
    } else {
        &item.label_en
    };
    let desc = if lang == Language::Vi {
        &item.desc_vi
    } else {
        &item.desc_en
    };
    let lock = item.lock_reason.as_ref().map(|r| {
        if lang == Language::Vi {
            r.vi.as_str()
        } else {
            r.en.as_str()
        }
    });

    let mut changed_value: Option<SettingValue> = None;

    let icon = match item.control {
        SettingControl::Toggle => crate::widgets::IconKind::Toggle,
        SettingControl::Slider { .. } => crate::widgets::IconKind::Sliders,
        SettingControl::Enum(_) => crate::widgets::IconKind::List,
    };

    crate::widgets::setting_card_with_icon(ui, p, icon, label, desc, lock, |ui| match &item.control {
        SettingControl::Toggle => {
            let mut b = matches!(item.value, SettingValue::Bool(true));
            if spring_toggle(ui, p, &mut b).changed() {
                item.value = SettingValue::Bool(b);
                changed_value = Some(SettingValue::Bool(b));
            }
        }
        SettingControl::Enum(options) => {
            let mut selected_id = match item.value {
                SettingValue::EnumId(id) => id,
                _ => options.first().map(|o| o.id).unwrap_or(0),
            };
            let current_text = options
                .iter()
                .find(|o| o.id == selected_id)
                .map(|o| {
                    if lang == Language::Vi {
                        o.label_vi.as_str()
                    } else {
                        o.label_en.as_str()
                    }
                })
                .unwrap_or("—");

            let prev = selected_id;
            egui::ComboBox::from_id_salt(&item.id)
                .selected_text(current_text)
                .show_ui(ui, |ui| {
                    for o in options {
                        let text = if lang == Language::Vi {
                            o.label_vi.as_str()
                        } else {
                            o.label_en.as_str()
                        };
                        ui.selectable_value(&mut selected_id, o.id, text);
                    }
                });
            if selected_id != prev {
                item.value = SettingValue::EnumId(selected_id);
                changed_value = Some(SettingValue::EnumId(selected_id));
            }
        }
        SettingControl::Slider {
            min,
            max,
            step,
            unit,
        } => {
            let mut val = match item.value {
                SettingValue::Float(f) => f,
                SettingValue::Int(i) => i as f32,
                _ => *min,
            };
            let r = crate::widgets::fluent_slider(ui, p, &mut val, *min..=*max, *step, unit.as_str(), 0);
            item.value = SettingValue::Float(val);
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed_value = Some(SettingValue::Float(val));
            }
        }
    });

    changed_value
}
