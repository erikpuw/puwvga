use crate::state::AppState;
use crate::theme::{vendor_color, Palette};
use crate::widgets::{
    fluent_slider, section_header, setting_card, setting_card_with_icon, spring_toggle, vendor_dot,
    IconKind,
};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use puwvga_core::{
    tr, ColorDepth, ColorSettings, DynamicRange, Language, OutputColorFormat, TextKey, Vendor,
};
use puwvga_win32::apply_gamma_ramp;

pub fn render(ui: &mut Ui, state: &mut AppState, p: Palette) {
    let lang = state.language;
    let is_vi = lang == Language::Vi;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(tr(lang, TextKey::ColorTitle))
                    .size(21.0)
                    .strong()
                    .color(p.text_primary),
            );
            ui.label(
                RichText::new(tr(lang, TextKey::ColorSubtitle))
                    .size(12.5)
                    .color(p.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr(lang, TextKey::ResetDefaults)).clicked() {
                if let Some(disp) = state.displays.get(state.selected_display_idx).cloned() {
                    let def = ColorSettings::default();
                    let _ = apply_gamma_ramp(&disp.device_name, def.brightness, def.contrast, def.gamma);
                    if let Some(gpu) = state.owner_gpu_for_display(&disp).cloned() {
                        if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
                            let res = b.set_color(&gpu.id, &disp.id, &def);
                            state.handle_backend_result(res, "Đã khôi phục màu mặc định");
                        }
                    }
                    state.color_by_display.insert(disp.id, def);
                }
            }
        });
    });

    ui.add_space(10.0);

    if state.displays.is_empty() {
        return;
    }

    // Display selector strip showing owning GPU for each monitor
    let displays = state.displays.clone();
    ui.horizontal_wrapped(|ui| {
        for (idx, disp) in displays.iter().enumerate() {
            let active = idx == state.selected_display_idx;
            let owner = state.owner_gpu_for_display(disp).cloned();
            let frame = Frame::new()
                .fill(if active { p.accent_soft } else { p.bg_card })
                .stroke(Stroke::new(
                    1.0_f32,
                    if active { p.accent } else { p.border_subtle },
                ))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 8));
            let resp = frame
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if let Some(ref g) = owner {
                                vendor_dot(ui, vendor_color(g.vendor));
                            }
                            ui.label(
                                RichText::new(&disp.friendly_name)
                                    .size(13.0)
                                    .strong()
                                    .color(p.text_primary),
                            );
                        });
                        let owner_name = owner
                            .as_ref()
                            .map(|g| g.name.as_str())
                            .unwrap_or("GPU");
                        ui.label(
                            RichText::new(format!(
                                "{} · {}×{} @ {}Hz",
                                owner_name, disp.width, disp.height, disp.refresh_hz
                            ))
                            .size(11.5)
                            .color(p.text_secondary),
                        );
                    });
                })
                .response
                .interact(egui::Sense::click());
            if resp.clicked() {
                state.selected_display_idx = idx;
            }
        }
    });

    let disp = displays[state.selected_display_idx].clone();
    let owner = state.owner_gpu_for_display(&disp).cloned();
    let vendor = owner.as_ref().map(|g| g.vendor).unwrap_or(Vendor::Intel);
    let mut color = state
        .color_by_display
        .get(&disp.id)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;

    section_header(
        ui,
        p,
        &format!(
            "{} · {}",
            if is_vi {
                "Điều chỉnh hình ảnh"
            } else {
                "Color Adjustments"
            },
            owner.as_ref().map(|g| g.name.as_str()).unwrap_or("GPU")
        ),
    );

    let mut gamma_live_changed = false;

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Brightness",
        if is_vi {
            "Độ sáng hiển thị đầu ra của driver"
        } else {
            "Driver output brightness"
        },
        None,
        |ui| {
            let r = fluent_slider(ui, p, &mut color.brightness, 0.0..=100.0, 1.0, "%", 0);
            if r.changed() {
                gamma_live_changed = true;
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed = true;
            }
        },
    );

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Contrast",
        if is_vi {
            "Độ tương phản giữa vùng sáng và vùng tối"
        } else {
            "Contrast ratio adjustment"
        },
        None,
        |ui| {
            let r = fluent_slider(ui, p, &mut color.contrast, 0.0..=100.0, 1.0, "%", 0);
            if r.changed() {
                gamma_live_changed = true;
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed = true;
            }
        },
    );

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Gamma",
        if is_vi {
            "Đường cong sáng trung gian (mặc định 1.00)"
        } else {
            "Mid-tone gamma curve (default 1.00)"
        },
        None,
        |ui| {
            let r = fluent_slider(ui, p, &mut color.gamma, 0.5..=2.5, 0.05, "", 2);
            if r.changed() {
                gamma_live_changed = true;
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed = true;
            }
        },
    );

    if gamma_live_changed {
        let _ = apply_gamma_ramp(&disp.device_name, color.brightness, color.contrast, color.gamma);
    }

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        if vendor == Vendor::Nvidia {
            "Digital Vibrance"
        } else {
            "Saturation"
        },
        if is_vi {
            "Độ rực màu sắc hiển thị"
        } else {
            "Color saturation / vibrance intensity"
        },
        None,
        |ui| {
            let r = fluent_slider(
                ui,
                p,
                &mut color.vibrance_or_saturation,
                0.0..=100.0,
                1.0,
                "%",
                0,
            );
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed = true;
            }
        },
    );

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Hue",
        if is_vi {
            "Dịch chuyển tông màu (-180° đến +180°)"
        } else {
            "Color hue angle (-180° to +180°)"
        },
        None,
        |ui| {
            let r = fluent_slider(ui, p, &mut color.hue, -180.0..=180.0, 1.0, "°", 0);
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                changed = true;
            }
        },
    );

    if vendor == Vendor::Intel {
        let mut lace = color.lace_enabled.unwrap_or(true);
        setting_card_with_icon(
            ui,
            p,
            IconKind::Toggle,
            "Dynamic Contrast Enhancement (LACE)",
            if is_vi {
                "Tự cân bằng tương phản cục bộ trên Intel IGCL"
            } else {
                "Intel Local Adaptive Contrast Enhancement"
            },
            None,
            |ui| {
                if spring_toggle(ui, p, &mut lace).changed() {
                    color.lace_enabled = Some(lace);
                    changed = true;
                }
            },
        );
    }

    if vendor == Vendor::Amd {
        let mut k = color.color_temp_kelvin.unwrap_or(6500) as f32;
        setting_card_with_icon(
            ui,
            p,
            IconKind::Sliders,
            "Color Temperature",
            if is_vi {
                "Nhiệt độ màu (Kelvin) trên AMD ADLX"
            } else {
                "Display color temperature (Kelvin)"
            },
            None,
            |ui| {
                let r = fluent_slider(ui, p, &mut k, 4000.0..=10000.0, 100.0, " K", 0);
                if r.drag_stopped() || (r.changed() && !r.dragged()) {
                    color.color_temp_kelvin = Some(k.round() as u32);
                    changed = true;
                }
            },
        );
    }

    section_header(
        ui,
        p,
        if is_vi {
            "Định dạng tín hiệu màu (Wire Format)"
        } else {
            "Output Color Format"
        },
    );

    let prev_wf = color.wire_format;
    setting_card(
        ui,
        p,
        "Output Color Depth",
        if is_vi {
            "Độ sâu màu mỗi kênh (bpc)"
        } else {
            "Bits per color channel"
        },
        None,
        |ui| {
            let depth_label = match color.wire_format.depth {
                ColorDepth::Bpc6 => "6 bpc",
                ColorDepth::Bpc8 => "8 bpc",
                ColorDepth::Bpc10 => "10 bpc",
                ColorDepth::Bpc12 => "12 bpc",
            };
            egui::ComboBox::from_id_salt("wf_depth")
                .selected_text(depth_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut color.wire_format.depth, ColorDepth::Bpc8, "8 bpc");
                    ui.selectable_value(&mut color.wire_format.depth, ColorDepth::Bpc10, "10 bpc");
                    ui.selectable_value(&mut color.wire_format.depth, ColorDepth::Bpc12, "12 bpc");
                });
        },
    );

    setting_card(
        ui,
        p,
        "Output Color Format",
        if is_vi {
            "Không gian màu xuất ra màn hình"
        } else {
            "Pixel format sent to display"
        },
        None,
        |ui| {
            let fmt_label = match color.wire_format.format {
                OutputColorFormat::Rgb => "RGB",
                OutputColorFormat::YCbCr444 => "YCbCr444",
                OutputColorFormat::YCbCr422 => "YCbCr422",
                OutputColorFormat::YCbCr420 => "YCbCr420",
            };
            egui::ComboBox::from_id_salt("wf_fmt")
                .selected_text(fmt_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut color.wire_format.format,
                        OutputColorFormat::Rgb,
                        "RGB",
                    );
                    ui.selectable_value(
                        &mut color.wire_format.format,
                        OutputColorFormat::YCbCr444,
                        "YCbCr444",
                    );
                    ui.selectable_value(
                        &mut color.wire_format.format,
                        OutputColorFormat::YCbCr422,
                        "YCbCr422",
                    );
                    ui.selectable_value(
                        &mut color.wire_format.format,
                        OutputColorFormat::YCbCr420,
                        "YCbCr420",
                    );
                });
        },
    );

    setting_card(
        ui,
        p,
        "Output Dynamic Range",
        if is_vi {
            "Full (0–255) cho màn hình PC, Limited (16–235) cho TV"
        } else {
            "Full (0-255) for monitors, Limited (16-235) for TVs"
        },
        None,
        |ui| {
            egui::ComboBox::from_id_salt("wf_range")
                .selected_text(match color.wire_format.dynamic_range {
                    DynamicRange::Full => "Full (0-255)",
                    DynamicRange::Limited => "Limited (16-235)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut color.wire_format.dynamic_range,
                        DynamicRange::Full,
                        "Full (0-255)",
                    );
                    ui.selectable_value(
                        &mut color.wire_format.dynamic_range,
                        DynamicRange::Limited,
                        "Limited (16-235)",
                    );
                });
        },
    );

    if color.wire_format != prev_wf {
        changed = true;
    }

    state.color_by_display.insert(disp.id.clone(), color.clone());
    if changed {
        if let Some(gpu) = owner {
            if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
                let res = b.set_color(&gpu.id, &disp.id, &color);
                state.handle_backend_result(
                    res,
                    if is_vi {
                        "Đã áp dụng cấu hình màu vào driver"
                    } else {
                        "Applied color settings to driver"
                    },
                );
            }
        }
    }
}
