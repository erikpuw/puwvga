use crate::state::AppState;
use crate::theme::{vendor_color, Palette};
use crate::widgets::{metric_box, section_header, setting_card, vendor_dot};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use puwvga_core::{tr, Language, MuxMode, TextKey, Vendor};

pub fn render(ui: &mut Ui, state: &mut AppState, p: Palette) {
    let lang = state.language;
    let is_vi = lang == Language::Vi;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(tr(lang, TextKey::DashboardTitle))
                    .size(21.0)
                    .strong()
                    .color(p.text_primary),
            );
            ui.label(
                RichText::new(tr(lang, TextKey::DashboardSubtitle))
                    .size(12.5)
                    .color(p.text_secondary),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr(lang, TextKey::Refresh)).clicked() {
                state.refresh_hardware();
                state.set_toast(if is_vi {
                    "Đã làm mới danh sách GPU & màn hình"
                } else {
                    "Refreshed GPU & display list"
                });
            }
        });
    });

    ui.add_space(12.0);

    let adapters = state.adapters.clone();
    if adapters.is_empty() {
        setting_card(
            ui,
            p,
            if is_vi {
                "Không phát hiện VGA Intel / NVIDIA / AMD"
            } else {
                "No Intel / NVIDIA / AMD GPU detected"
            },
            "",
            None,
            |_| {},
        );
        return;
    }

    for gpu in &adapters {
        let attached = state.displays_for_gpu(gpu).len();
        let vram_text = if gpu.dedicated_vram_mb >= 1024 {
            format!("{:.1} GB", gpu.dedicated_vram_mb as f64 / 1024.0)
        } else if gpu.dedicated_vram_mb > 0 {
            format!("{} MB", gpu.dedicated_vram_mb)
        } else {
            "Shared (DXGI)".into()
        };
        let pci_text = format!(
            "VEN_{:04X} · DEV_{:04X}",
            gpu.pci_vendor_id, gpu.pci_device_id
        );

        Frame::new()
            .fill(p.bg_card)
            .stroke(Stroke::new(1.0, p.border_subtle))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    vendor_dot(ui, vendor_color(gpu.vendor));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&gpu.name)
                                .size(14.5)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("{} · {}", gpu.driver_api, gpu.driver_version))
                                .size(11.5)
                                .color(p.text_secondary),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Active").size(11.5).strong().color(p.ok));
                    });
                });

                ui.add_space(10.0);
                ui.columns(4, |cols| {
                    metric_box(&mut cols[0], p, "Driver", &gpu.driver_version);
                    metric_box(&mut cols[1], p, "VRAM", &vram_text);
                    metric_box(&mut cols[2], p, "PCI ID", &pci_text);
                    metric_box(
                        &mut cols[3],
                        p,
                        if is_vi { "Màn hình nối" } else { "Displays" },
                        &attached.to_string(),
                    );
                });
            });
        ui.add_space(8.0);
    }

    section_header(
        ui,
        p,
        if is_vi {
            "Trạng thái xuất hình"
        } else {
            "Output Status"
        },
    );

    if adapters.len() > 1 {
        let mut selected = state.mux_mode;
        setting_card(
            ui,
            p,
            "Display Mode (MUX / Optimus)",
            if is_vi {
                "Chuyển chế độ xuất hình giữa iGPU tiết kiệm điện và dGPU trực tiếp"
            } else {
                "Switch display routing between hybrid Optimus and dGPU-only"
            },
            None,
            |ui| {
                egui::ComboBox::from_id_salt("dash_mux")
                    .selected_text(mux_label(selected))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut selected,
                            MuxMode::Automatic,
                            mux_label(MuxMode::Automatic),
                        );
                        ui.selectable_value(
                            &mut selected,
                            MuxMode::HybridOptimus,
                            mux_label(MuxMode::HybridOptimus),
                        );
                        ui.selectable_value(
                            &mut selected,
                            MuxMode::DiscreteGpuOnly,
                            mux_label(MuxMode::DiscreteGpuOnly),
                        );
                    });
            },
        );
        if selected != state.mux_mode {
            state.mux_mode = selected;
            if let Some(nv_gpu) = state.adapters.iter().find(|g| g.vendor == Vendor::Nvidia).cloned() {
                if let Some(b) = state.backends.get(&Vendor::Nvidia).cloned() {
                    let res = b.set_mux(&nv_gpu.id, selected);
                    state.handle_backend_result(res, "Đã cập nhật Display Mode (MUX)");
                }
            }
        }
    } else if let Some(gpu) = adapters.first() {
        let disp_desc = state
            .displays
            .first()
            .map(|d| format!("{} ({}×{} @ {}Hz)", d.friendly_name, d.width, d.height, d.refresh_hz))
            .unwrap_or_else(|| gpu.name.clone());
        setting_card(
            ui,
            p,
            if is_vi {
                "Chế độ đơn GPU (PC)"
            } else {
                "Single GPU Mode (Desktop)"
            },
            &disp_desc,
            None,
            |ui| {
                ui.label(RichText::new("Direct Output").size(12.0).color(p.accent));
            },
        );
    }
}

fn mux_label(mode: MuxMode) -> &'static str {
    match mode {
        MuxMode::Automatic => "Automatic Select (Advanced Optimus)",
        MuxMode::HybridOptimus => "Optimus (iGPU + dGPU)",
        MuxMode::DiscreteGpuOnly => "Discrete GPU Only",
    }
}
