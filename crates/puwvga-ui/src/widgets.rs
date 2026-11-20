use crate::theme::Palette;
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2};

pub fn vendor_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.2, color);
}

pub fn section_header(ui: &mut Ui, p: Palette, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.5)
            .strong()
            .color(p.text_secondary),
    );
    ui.add_space(4.0);
}

/// Lenovo Legion Toolkit horizontal setting card.
pub fn setting_card(
    ui: &mut Ui,
    p: Palette,
    title: &str,
    description: &str,
    lock_reason: Option<&str>,
    add_control: impl FnOnce(&mut Ui),
) {
    Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 230.0).max(180.0));
                    ui.label(
                        RichText::new(title)
                            .size(13.5)
                            .strong()
                            .color(if lock_reason.is_some() {
                                p.text_secondary
                            } else {
                                p.text_primary
                            }),
                    );
                    if !description.is_empty() {
                        ui.label(RichText::new(description).size(12.0).color(p.text_secondary));
                    }
                    if let Some(reason) = lock_reason {
                        ui.label(RichText::new(reason).size(11.5).color(p.warn));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(lock_reason.is_none(), add_control);
                });
            });
        });
    ui.add_space(5.0);
}

/// Critically-damped smooth toggle switch inspired by Fluent / Apple fluid response.
pub fn spring_toggle(ui: &mut Ui, p: Palette, on: &mut bool) -> egui::Response {
    let desired_size = Vec2::new(40.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_with_time(response.id, *on, 0.18);
        let radius = rect.height() * 0.5;
        let bg = if *on { p.accent } else { p.bg_control };
        let border = if *on { p.accent } else { p.border_strong };
        ui.painter().rect(
            rect,
            CornerRadius::same(radius as u8),
            bg,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let knob_center = egui::pos2(knob_x, rect.center().y);
        let knob_color = if *on { p.accent_fg } else { p.text_secondary };
        ui.painter()
            .circle_filled(knob_center, radius - 4.0, knob_color);
    }

    response
}

pub fn metric_box(ui: &mut Ui, p: Palette, label: &str, value: &str) {
    Frame::new()
        .fill(p.bg_window)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(11.0).color(p.text_muted));
            ui.label(
                RichText::new(value)
                    .size(13.5)
                    .strong()
                    .color(p.text_primary),
            );
        });
}
