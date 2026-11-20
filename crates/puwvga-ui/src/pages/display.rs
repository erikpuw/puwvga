use crate::state::AppState;
use crate::theme::{vendor_color, Palette};
use crate::widgets::{section_header, setting_card, setting_card_with_icon, spring_toggle, vendor_dot, IconKind};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use puwvga_core::{tr, Language, ScalingMode, SpanningLayout, TextKey, Vendor};
use std::time::Instant;

pub fn render(ui: &mut Ui, state: &mut AppState, p: Palette) {
    let lang = state.language;
    let is_vi = lang == Language::Vi;

    ui.label(
        RichText::new(tr(lang, TextKey::DisplayTitle))
            .size(21.0)
            .strong()
            .color(p.text_primary),
    );
    ui.label(
        RichText::new(tr(lang, TextKey::DisplaySubtitle))
            .size(12.5)
            .color(p.text_secondary),
    );
    ui.add_space(10.0);

    // 15-second auto-revert confirmation banner
    if let Some(pending) = state.pending_revert.clone() {
        let secs = pending
            .deadline
            .saturating_duration_since(Instant::now())
            .as_secs();
        Frame::new()
            .fill(p.accent_soft)
            .stroke(Stroke::new(1.0_f32, p.accent))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let msg = if is_vi {
                        format!(
                            "Đang áp dụng cấu hình hiển thị mới. Tự hoàn tác sau {secs}s nếu chưa xác nhận."
                        )
                    } else {
                        format!(
                            "Display change applied. Auto-reverting in {secs}s unless confirmed."
                        )
                    };
                    ui.label(RichText::new(msg).size(12.5).strong().color(p.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(lang, TextKey::KeepChanges)).clicked() {
                            state.keep_pending();
                        }
                        if ui.button(tr(lang, TextKey::RevertNow)).clicked() {
                            state.revert_pending();
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    // Inline GPU selector when >1 GPU
    if state.adapters.len() > 1 {
        let adapters = state.adapters.clone();
        ui.horizontal_wrapped(|ui| {
            for (idx, g) in adapters.iter().enumerate() {
                let active = idx == state.selected_gpu_idx;
                let attached = state.displays_for_gpu(g).len();
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
                                RichText::new(format!(
                                    "{} {}",
                                    attached,
                                    if is_vi { "màn hình" } else { "display(s)" }
                                ))
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

    let Some(gpu) = state.active_gpu().cloned() else {
        return;
    };

    section_header(
        ui,
        p,
        &format!(
            "{} · {}",
            if is_vi {
                "Chế độ hiển thị"
            } else {
                "Display Mode"
            },
            gpu.name
        ),
    );

    let attached_displays: Vec<_> = state
        .displays_for_gpu(&gpu)
        .into_iter()
        .cloned()
        .collect();

    if let Some(disp) = attached_displays.first() {
        let modes = state
            .modes_by_device
            .get(&disp.device_name)
            .cloned()
            .unwrap_or_default();
        let current_mode = (disp.width, disp.height, disp.refresh_hz);
        let mut chosen_mode = current_mode;

        setting_card(
            ui,
            p,
            if is_vi {
                "Độ phân giải & Tần số quét"
            } else {
                "Resolution & Refresh Rate"
            },
            &format!("{} ({})", disp.friendly_name, disp.device_name),
            None,
            |ui| {
                egui::ComboBox::from_id_salt("disp_mode_combo")
                    .selected_text(format!(
                        "{} × {} @ {} Hz",
                        chosen_mode.0, chosen_mode.1, chosen_mode.2
                    ))
                    .show_ui(ui, |ui| {
                        for &(w, h, hz) in modes.iter().take(24) {
                            ui.selectable_value(
                                &mut chosen_mode,
                                (w, h, hz),
                                format!("{w} × {h} @ {hz} Hz"),
                            );
                        }
                    });
            },
        );

        if chosen_mode != current_mode {
            state.apply_mode_with_revert(&disp.device_name, current_mode, chosen_mode);
        }

        let mut disp_changed = false;
        if let Some(ds) = state.display_settings_by_id.get_mut(&disp.id) {
            let prev_ds = ds.clone();
            let scaling_text = match ds.scaling {
                ScalingMode::AspectRatio => "Aspect Ratio",
                ScalingMode::FullScreen => "Full-screen",
                ScalingMode::Center => "Center",
                ScalingMode::Integer => {
                    if gpu.vendor == Vendor::Intel {
                        "Retro Scaling (Integer)"
                    } else {
                        "Integer Scaling"
                    }
                }
            };
            setting_card(
                ui,
                p,
                "Scaling Mode",
                if is_vi {
                    "Cách kéo giãn hình ảnh ở độ phân giải thấp hơn native"
                } else {
                    "How non-native resolutions are scaled"
                },
                None,
                |ui| {
                    egui::ComboBox::from_id_salt("disp_scaling")
                        .selected_text(scaling_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut ds.scaling,
                                ScalingMode::AspectRatio,
                                "Aspect Ratio",
                            );
                            ui.selectable_value(
                                &mut ds.scaling,
                                ScalingMode::FullScreen,
                                "Full-screen",
                            );
                            ui.selectable_value(&mut ds.scaling, ScalingMode::Center, "Center");
                            ui.selectable_value(
                                &mut ds.scaling,
                                ScalingMode::Integer,
                                if gpu.vendor == Vendor::Intel {
                                    "Retro Scaling (Integer)"
                                } else {
                                    "Integer Scaling"
                                },
                            );
                        });
                },
            );

            let vrr_label = match gpu.vendor {
                Vendor::Nvidia => "G-SYNC / Adaptive Sync",
                Vendor::Intel => "Variable Refresh Rate (VRR)",
                Vendor::Amd => "AMD FreeSync",
            };
            setting_card_with_icon(
                ui,
                p,
                IconKind::Toggle,
                vrr_label,
                if is_vi {
                    "Đồng bộ tần số quét chống xé hình"
                } else {
                    "Eliminate screen tearing via adaptive sync"
                },
                None,
                |ui| {
                    spring_toggle(ui, p, &mut ds.vrr_enabled);
                },
            );

            if gpu.vendor == Vendor::Intel {
                let mut sharp = ds.sharpness.unwrap_or(20) as f32;
                setting_card_with_icon(
                    ui,
                    p,
                    IconKind::Sliders,
                    "Display Sharpness",
                    if is_vi {
                        "Độ sắc nét hình ảnh qua Intel IGCL"
                    } else {
                        "Intel IGCL display sharpening"
                    },
                    None,
                    |ui| {
                        let r = crate::widgets::fluent_slider(ui, p, &mut sharp, 0.0..=100.0, 1.0, "%", 0);
                        if r.changed() {
                            ds.sharpness = Some(sharp.round() as u32);
                        }
                    },
                );
            }
            if *ds != prev_ds {
                disp_changed = true;
            }
        }
        if disp_changed {
            if let Some(ds_copy) = state.display_settings_by_id.get(&disp.id).cloned() {
                state
                    .display_settings_by_id
                    .insert(disp.device_name.clone(), ds_copy);
            }
            state.auto_save_preset();
        }
    } else {
        setting_card(
            ui,
            p,
            if is_vi {
                "Không có màn hình nào đang nối trực tiếp vào GPU này"
            } else {
                "No displays currently connected directly to this GPU"
            },
            if is_vi {
                "Trên laptop chạy Optimus, màn hình nội bộ gắn vào iGPU; cắm màn hình rời qua HDMI/USB-C hoặc chuyển MUX sang dGPU để chỉnh trực tiếp."
            } else {
                "On Optimus laptops, the built-in panel is wired to the iGPU; attach an external monitor or switch MUX to dGPU to adjust display modes here."
            },
            None,
            |_| {},
        );
    }

    // Multi-Display Spanning section (NVIDIA Mosaic/Surround · Intel Combined Display · AMD Eyefinity)
    let span_title = match gpu.vendor {
        Vendor::Nvidia => "NVIDIA Mosaic / Surround (nvapi64.dll · NvAPI_Mosaic_SetDisplayGrids)",
        Vendor::Intel => "Intel Combined Display / Collage (ControlLib.dll · ctlGetSetCombinedDisplay)",
        Vendor::Amd => "AMD Eyefinity SLS (atiadlxx.dll · ADLX Eyefinity)",
    };
    section_header(
        ui,
        p,
        &format!(
            "{} · {}",
            if is_vi {
                "Ghép màn hình"
            } else {
                "Multi-Display Spanning"
            },
            span_title
        ),
    );

    if attached_displays.len() < 2 {
        setting_card_with_icon(
            ui,
            p,
            IconKind::Span,
            tr(lang, TextKey::NeedTwoDisplays),
            &if is_vi {
                format!(
                    "Hiện có {} màn hình nối trực tiếp vào {}.",
                    attached_displays.len(),
                    gpu.name
                )
            } else {
                format!(
                    "Currently {} display(s) directly attached to {}.",
                    attached_displays.len(),
                    gpu.name
                )
            },
            None,
            |ui| {
                ui.label(
                    RichText::new(if is_vi { "Cần ≥ 2 màn" } else { "Need ≥ 2" })
                        .size(12.0)
                        .color(p.text_muted),
                );
            },
        );
        return;
    }

    let mut cfg = state
        .spanning_by_gpu
        .get(&gpu.id)
        .cloned()
        .unwrap_or_else(|| puwvga_core::SpanningConfig {
            enabled: false,
            layout: SpanningLayout::Horizontal1x2,
            display_ids: attached_displays.iter().map(|d| d.id.clone()).collect(),
            bezel: Default::default(),
        });

    let base_w = attached_displays[0].width.max(1920);
    let base_h = attached_displays[0].height.max(1080);
    let base_hz = attached_displays[0].refresh_hz.max(60);
    let (total_w, total_h) = cfg.total_resolution(base_w, base_h);
    let (rows, cols) = cfg.layout.dimensions();

    // Visual grid preview
    Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_vi {
                        "Sơ đồ lưới ghép màn hình"
                    } else {
                        "Spanning Topology Grid"
                    })
                    .size(13.0)
                    .strong()
                    .color(p.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{total_w} × {total_h} @ {base_hz} Hz"))
                            .size(12.5)
                            .strong()
                            .color(if cfg.enabled { p.accent } else { p.text_secondary }),
                    );
                });
            });
            ui.add_space(8.0);
            for r in 0..rows {
                ui.columns(cols as usize, |columns| {
                    for c in 0..cols {
                        let slot = (r * cols + c) as usize;
                        let tile_name = attached_displays
                            .get(slot)
                            .map(|d| d.friendly_name.as_str())
                            .unwrap_or(if is_vi { "Chưa nối màn hình" } else { "Unconnected" });
                        Frame::new()
                            .fill(if cfg.enabled { p.accent_soft } else { p.bg_window })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if cfg.enabled { p.accent } else { p.border_strong },
                            ))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::same(10))
                            .show(&mut columns[c as usize], |ui| {
                                ui.label(
                                    RichText::new(format!("[{}] {}", slot + 1, tile_name))
                                        .size(12.0)
                                        .strong()
                                        .color(p.text_primary),
                                );
                            });
                    }
                });
                ui.add_space(6.0);
            }
        });
    ui.add_space(6.0);

    setting_card(
        ui,
        p,
        if is_vi {
            "Bố cục lưới ghép (Spanning Layout)"
        } else {
            "Spanning Grid Layout"
        },
        if is_vi {
            "Sắp xếp hàng × cột các màn hình nối trực tiếp vào cùng GPU"
        } else {
            "Rows × Columns topology for attached monitors"
        },
        None,
        |ui| {
            egui::ComboBox::from_id_salt("span_layout_combo")
                .selected_text(layout_label(cfg.layout))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut cfg.layout, SpanningLayout::Horizontal1x2, "1 × 2");
                    ui.selectable_value(&mut cfg.layout, SpanningLayout::Vertical2x1, "2 × 1");
                    ui.selectable_value(&mut cfg.layout, SpanningLayout::Horizontal1x3, "1 × 3");
                    ui.selectable_value(&mut cfg.layout, SpanningLayout::Grid2x2, "2 × 2");
                });
        },
    );

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Bezel Compensation X",
        if is_vi {
            "Bù độ dày viền ngang giữa các màn hình (px)"
        } else {
            "Horizontal bezel gap compensation (px)"
        },
        None,
        |ui| {
            let mut bx = cfg.bezel.horizontal_px as f32;
            if crate::widgets::fluent_slider(ui, p, &mut bx, 0.0..=120.0, 1.0, " px", 0).changed() {
                cfg.bezel.horizontal_px = bx.round() as u32;
            }
        },
    );

    setting_card_with_icon(
        ui,
        p,
        IconKind::Sliders,
        "Bezel Compensation Y",
        if is_vi {
            "Bù độ dày viền dọc giữa các màn hình (px)"
        } else {
            "Vertical bezel gap compensation (px)"
        },
        None,
        |ui| {
            let mut by = cfg.bezel.vertical_px as f32;
            if crate::widgets::fluent_slider(ui, p, &mut by, 0.0..=120.0, 1.0, " px", 0).changed() {
                cfg.bezel.vertical_px = by.round() as u32;
            }
        },
    );

    let mut trigger_enable = false;
    let mut trigger_disable = false;
    setting_card_with_icon(
        ui,
        p,
        IconKind::Span,
        if cfg.enabled {
            if is_vi {
                "Đang bật ghép màn hình"
            } else {
                "Display Spanning Active"
            }
        } else if is_vi {
            "Kích hoạt ghép màn hình"
        } else {
            "Enable Display Spanning"
        },
        if is_vi {
            "Hỗ trợ tự động hoàn tác sau 15 giây nếu mất hình"
        } else {
            "Includes 15-second auto-revert safety fallback"
        },
        None,
        |ui| {
            if cfg.enabled {
                if ui.button(tr(lang, TextKey::DisableSpanning)).clicked() {
                    trigger_disable = true;
                }
            } else if ui.button(tr(lang, TextKey::EnableSpanning)).clicked() {
                trigger_enable = true;
            }
        },
    );

    state.spanning_by_gpu.insert(gpu.id.clone(), cfg.clone());
    if trigger_enable {
        cfg.enabled = true;
        state.apply_spanning_with_revert(gpu.id, gpu.vendor, cfg);
    } else if trigger_disable {
        if let Some(b) = state.backends.get(&gpu.vendor).cloned() {
            let res = b.disable_spanning(&gpu.id);
            if res.is_ok() {
                if let Some(c) = state.spanning_by_gpu.get_mut(&gpu.id) {
                    c.enabled = false;
                }
            }
            state.handle_backend_result(res, "Đã tách màn hình về chế độ độc lập");
        }
    }
}

fn layout_label(l: SpanningLayout) -> &'static str {
    match l {
        SpanningLayout::Horizontal1x2 => "1 × 2",
        SpanningLayout::Vertical2x1 => "2 × 1",
        SpanningLayout::Horizontal1x3 => "1 × 3",
        SpanningLayout::Grid2x2 => "2 × 2",
        SpanningLayout::Custom { .. } => "Custom",
    }
}
