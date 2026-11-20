use crate::theme::Palette;
use egui::{pos2, Color32, CornerRadius, Frame, Margin, Painter, Rect, RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Dashboard,
    Color,
    Display,
    Graphics,
    Settings,
    Sliders,
    List,
    Toggle,
    Span,
    Info,
}

pub fn draw_icon(painter: &Painter, center: egui::Pos2, size: f32, color: Color32, kind: IconKind) {
    let s = size * 0.5;
    let stroke = Stroke::new(1.5_f32, color);
    match kind {
        IconKind::Dashboard => {
            let g = 1.5_f32;
            let r1 = Rect::from_min_max(pos2(center.x - s, center.y - s), pos2(center.x - g, center.y + 0.5));
            let r2 = Rect::from_min_max(pos2(center.x + g, center.y - s), pos2(center.x + s, center.y - 1.5));
            let r3 = Rect::from_min_max(pos2(center.x + g, center.y + 1.5), pos2(center.x + s, center.y + s));
            let r4 = Rect::from_min_max(pos2(center.x - s, center.y + 3.0), pos2(center.x - g, center.y + s));
            for r in [r1, r2, r3, r4] {
                painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            }
        }
        IconKind::Color => {
            painter.circle_stroke(center, s * 0.85, stroke);
            painter.line_segment([pos2(center.x, center.y - s * 0.85), pos2(center.x, center.y + s * 0.85)], stroke);
            painter.line_segment([center, pos2(center.x + s * 0.72, center.y - s * 0.42)], stroke);
        }
        IconKind::Display => {
            let mon = Rect::from_min_max(pos2(center.x - s, center.y - s * 0.75), pos2(center.x + s, center.y + s * 0.45));
            painter.rect_stroke(mon, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            painter.line_segment([pos2(center.x, center.y + s * 0.45), pos2(center.x, center.y + s * 0.88)], stroke);
            painter.line_segment([pos2(center.x - s * 0.45, center.y + s * 0.88), pos2(center.x + s * 0.45, center.y + s * 0.88)], stroke);
        }
        IconKind::Graphics => {
            let top = pos2(center.x, center.y - s * 0.9);
            let right = pos2(center.x + s * 0.85, center.y - s * 0.42);
            let br = pos2(center.x + s * 0.85, center.y + s * 0.45);
            let bot = pos2(center.x, center.y + s * 0.92);
            let bl = pos2(center.x - s * 0.85, center.y + s * 0.45);
            let left = pos2(center.x - s * 0.85, center.y - s * 0.42);
            let pts = [top, right, br, bot, bl, left, top];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            painter.line_segment([center, right], stroke);
            painter.line_segment([center, left], stroke);
            painter.line_segment([center, bot], stroke);
        }
        IconKind::Settings => {
            painter.circle_stroke(center, s * 0.35, stroke);
            painter.circle_stroke(center, s * 0.78, stroke);
        }
        IconKind::Sliders => {
            for (dy, kx) in [(-s * 0.55, -0.25), (0.0, 0.35), (s * 0.55, -0.1)] {
                let y = center.y + dy;
                painter.line_segment([pos2(center.x - s * 0.85, y), pos2(center.x + s * 0.85, y)], stroke);
                painter.circle_filled(pos2(center.x + s * kx, y), 2.3, color);
            }
        }
        IconKind::List => {
            painter.line_segment([pos2(center.x - s * 0.8, center.y - s * 0.5), pos2(center.x + s * 0.8, center.y - s * 0.5)], stroke);
            painter.line_segment([pos2(center.x - s * 0.8, center.y), pos2(center.x + s * 0.8, center.y)], stroke);
            painter.line_segment([pos2(center.x - s * 0.8, center.y + s * 0.5), pos2(center.x + s * 0.25, center.y + s * 0.5)], stroke);
        }
        IconKind::Toggle => {
            let r = Rect::from_center_size(center, Vec2::new(size * 0.95, size * 0.56));
            painter.rect_stroke(r, CornerRadius::same(8), stroke, egui::StrokeKind::Inside);
            painter.circle_filled(pos2(center.x + s * 0.35, center.y), 2.5, color);
        }
        IconKind::Span => {
            let r1 = Rect::from_min_max(pos2(center.x - s, center.y - s * 0.65), pos2(center.x - 1.2, center.y + s * 0.65));
            let r2 = Rect::from_min_max(pos2(center.x + 1.2, center.y - s * 0.65), pos2(center.x + s, center.y + s * 0.65));
            painter.rect_stroke(r1, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            painter.rect_stroke(r2, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
        }
        IconKind::Info => {
            painter.circle_stroke(center, s * 0.85, stroke);
            painter.line_segment([pos2(center.x, center.y - 1.0), pos2(center.x, center.y + s * 0.45)], stroke);
            painter.circle_filled(pos2(center.x, center.y - s * 0.42), 1.2, color);
        }
    }
}

pub fn vendor_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.2, color);
}

pub fn section_header(ui: &mut Ui, p: Palette, title: &str) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.5)
            .strong()
            .color(p.text_secondary),
    );
    ui.add_space(5.0);
}

pub fn pill_badge(ui: &mut Ui, text: &str, text_color: Color32, bg: Color32) {
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(99))
        .inner_margin(Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(text_color));
        });
}

/// Fluent segmented control matching mockup (subtle dark pill, active card fill, no harsh cyan block).
pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    p: Palette,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(p.bg_control)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for &(val, label) in options {
                    let active = *current == val;
                    let resp = Frame::new()
                        .fill(if active { p.bg_card } else { Color32::TRANSPARENT })
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(Margin::symmetric(11, 5))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(label)
                                    .size(12.5)
                                    .strong()
                                    .color(if active {
                                        p.text_primary
                                    } else {
                                        p.text_secondary
                                    }),
                            );
                        })
                        .response
                        .interact(Sense::click());
                    if resp.clicked() && !active {
                        *current = val;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Lenovo Legion Toolkit horizontal setting card with 32x32 vector icon badge on the left.
pub fn setting_card_with_icon(
    ui: &mut Ui,
    p: Palette,
    icon: IconKind,
    title: &str,
    description: &str,
    lock_reason: Option<&str>,
    add_control: impl FnOnce(&mut Ui),
) {
    Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(15, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // 32x32 Fluent Icon Box
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                ui.painter().rect_filled(icon_rect, CornerRadius::same(7), p.bg_control);
                draw_icon(
                    ui.painter(),
                    icon_rect.center(),
                    15.0,
                    if lock_reason.is_some() { p.text_muted } else { p.text_primary },
                    icon,
                );
                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 235.0).max(180.0));
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
    ui.add_space(6.0);
}

pub fn setting_card(
    ui: &mut Ui,
    p: Palette,
    title: &str,
    description: &str,
    lock_reason: Option<&str>,
    add_control: impl FnOnce(&mut Ui),
) {
    setting_card_with_icon(ui, p, IconKind::List, title, description, lock_reason, add_control);
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
            Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let knob_center = pos2(knob_x, rect.center().y);
        let knob_color = if *on { p.accent_fg } else { p.text_secondary };
        ui.painter()
            .circle_filled(knob_center, radius - 4.0, knob_color);
    }

    response
}

/// Sleek Fluent / Apple-style slider matching the mockup:
/// 4px rounded track with accent fill on the left, compact 14px thumb, crisp right-aligned value readout.
pub fn fluent_slider(
    ui: &mut Ui,
    p: Palette,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    suffix: &str,
    decimals: usize,
) -> egui::Response {
    let min = *range.start();
    let max = *range.end();
    let span = (max - min).max(f32::EPSILON);

    let mut slider_resp = None;
    ui.horizontal(|ui| {
        // Right-to-left parent layout: allocate value text first so it sits on the right, then slider track on the left.
        let val_text = if decimals == 0 {
            format!("{}{}", value.round() as i32, suffix)
        } else {
            format!("{:.prec$}{}", *value, suffix, prec = decimals)
        };
        ui.allocate_ui_with_layout(
            Vec2::new(48.0, 22.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(
                    RichText::new(val_text)
                        .size(12.5)
                        .strong()
                        .color(p.text_primary),
                );
            },
        );
        ui.add_space(6.0);

        let track_size = Vec2::new(165.0, 22.0);
        let (rect, mut resp) = ui.allocate_exact_size(track_size, Sense::click_and_drag());

        let pad = 7.0_f32;
        let left = rect.left() + pad;
        let right = rect.right() - pad;
        let usable = (right - left).max(1.0);

        if (resp.clicked() || resp.dragged()) && resp.interact_pointer_pos().is_some() {
            let px = resp.interact_pointer_pos().unwrap().x;
            let t = ((px - left) / usable).clamp(0.0, 1.0);
            let mut next = min + t * span;
            if step > 0.0 {
                next = ((next - min) / step).round() * step + min;
            }
            next = next.clamp(min, max);
            if (*value - next).abs() > 1e-5 {
                *value = next;
                resp.mark_changed();
            }
        }

        if ui.is_rect_visible(rect) {
            let t = ((*value - min) / span).clamp(0.0, 1.0);
            let cy = rect.center().y;
            let thumb_x = left + t * usable;

            // Inactive background track (4px tall)
            let bg_rect = Rect::from_min_max(pos2(left, cy - 2.0), pos2(right, cy + 2.0));
            ui.painter()
                .rect_filled(bg_rect, CornerRadius::same(2), p.border_strong);

            // Active accent fill on left
            let fill_rect = Rect::from_min_max(pos2(left, cy - 2.0), pos2(thumb_x, cy + 2.0));
            ui.painter()
                .rect_filled(fill_rect, CornerRadius::same(2), p.accent);

            // Compact 14px thumb (grows slightly on hover/drag)
            let r = if resp.dragged() || resp.hovered() {
                7.5_f32
            } else {
                6.5_f32
            };
            ui.painter().circle_filled(pos2(thumb_x, cy), r, p.accent);
            ui.painter().circle_stroke(
                pos2(thumb_x, cy),
                r,
                Stroke::new(1.0_f32, p.bg_card),
            );
        }

        slider_resp = Some(resp);
    });

    slider_resp.unwrap()
}

pub fn metric_box(ui: &mut Ui, p: Palette, label: &str, value: &str) {
    Frame::new()
        .fill(p.bg_window)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(11.0).color(p.text_muted));
            ui.add_space(2.0);
            ui.label(
                RichText::new(value)
                    .size(13.5)
                    .strong()
                    .color(p.text_primary),
            );
        });
}
