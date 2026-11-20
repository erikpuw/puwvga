#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pages;
mod state;
mod theme;
mod widgets;

use eframe::egui;
use egui::{CornerRadius, Frame, Margin, RichText, Sense, Stroke, Vec2};
use puwvga_core::{tr, GpuRole, TextKey};
use puwvga_win32::relaunch_as_administrator;
use state::{AppState, Page};
use std::time::{Duration, Instant};
use theme::{apply_fluent_visuals, configure_fonts, vendor_color, Palette};
use widgets::{draw_icon, vendor_dot, IconKind};

struct PuwvgaApp {
    state: AppState,
    last_dark: Option<bool>,
}

impl PuwvgaApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let state = AppState::new();
        let dark = state.is_dark();
        apply_fluent_visuals(&cc.egui_ctx, Palette::for_mode(dark));
        Self {
            state,
            last_dark: Some(dark),
        }
    }
}

impl eframe::App for PuwvgaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dark = self.state.is_dark();
        let p = Palette::for_mode(dark);
        if self.last_dark != Some(dark) {
            apply_fluent_visuals(ctx, p);
            self.last_dark = Some(dark);
        }

        if self.state.pending_revert.is_some() {
            self.state.tick_revert();
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        if let Some((_, until)) = &self.state.toast {
            if Instant::now() >= *until {
                self.state.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }

        let lang = self.state.language;

        // Left Sidebar — fixed 236px, zero black gap, strict truncation so long GPU names never overflow
        egui::SidePanel::left("puwvga_sidebar")
            .exact_width(236.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(p.bg_sidebar)
                    .inner_margin(Margin::symmetric(10, 12)),
            )
            .show(ctx, |ui| {
                // Draw crisp 1px right divider line on sidebar edge
                let panel_rect = ui.max_rect().expand2(Vec2::new(10.0, 12.0));
                ui.painter().line_segment(
                    [
                        egui::pos2(panel_rect.right(), panel_rect.top()),
                        egui::pos2(panel_rect.right(), panel_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, p.border_subtle),
                );

                ui.vertical(|ui| {
                    ui.set_max_width(216.0);
                    ui.horizontal(|ui| {
                        let (mark_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                        ui.painter().rect(
                            mark_rect,
                            CornerRadius::same(5),
                            p.accent_soft,
                            Stroke::new(1.0_f32, p.accent),
                            egui::StrokeKind::Inside,
                        );
                        draw_icon(ui.painter(), mark_rect.center(), 12.0, p.accent, IconKind::Display);
                        ui.label(
                            RichText::new("puwvga")
                                .size(15.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Portable").size(10.5).color(p.text_muted));
                        });
                    });
                    ui.add_space(12.0);

                    // Detected GPUs card (strictly clamped width, truncated labels)
                    Frame::new()
                        .fill(p.bg_card)
                        .stroke(Stroke::new(1.0_f32, p.border_subtle))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(198.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tr(lang, TextKey::DetectedGpus).to_uppercase())
                                        .size(10.0)
                                        .strong()
                                        .color(p.text_muted),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(self.state.adapters.len().to_string())
                                                .size(10.5)
                                                .color(p.text_muted),
                                        );
                                    },
                                );
                            });
                            ui.add_space(5.0);
                            let adapters = self.state.adapters.clone();
                            for (idx, g) in adapters.iter().enumerate() {
                                let active = idx == self.state.selected_gpu_idx;
                                let attached = self.state.displays_for_gpu(g).len();
                                let role_str = match g.role {
                                    GpuRole::Integrated => format!("iGPU · {attached} display"),
                                    GpuRole::Discrete => format!("dGPU · {attached} display"),
                                    GpuRole::Primary => format!("Primary · {attached} display"),
                                };
                                let resp = Frame::new()
                                    .fill(if active {
                                        p.bg_control
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    })
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if active {
                                            p.border_strong
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        },
                                    ))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(180.0);
                                        ui.horizontal(|ui| {
                                            vendor_dot(ui, vendor_color(g.vendor));
                                            ui.vertical(|ui| {
                                                ui.set_max_width(156.0);
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&g.name)
                                                            .size(12.0)
                                                            .strong()
                                                            .color(if active {
                                                                p.text_primary
                                                            } else {
                                                                p.text_secondary
                                                            }),
                                                    )
                                                    .truncate(),
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(role_str)
                                                            .size(10.5)
                                                            .color(p.text_muted),
                                                    )
                                                    .truncate(),
                                                );
                                            });
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());
                                if resp.clicked() {
                                    self.state.select_gpu(idx);
                                }
                                ui.add_space(3.0);
                            }
                        });

                    ui.add_space(12.0);

                    nav_button(
                        ui,
                        p,
                        IconKind::Dashboard,
                        tr(lang, TextKey::NavDashboard),
                        self.state.page == Page::Dashboard,
                        || self.state.page = Page::Dashboard,
                    );
                    nav_button(
                        ui,
                        p,
                        IconKind::Color,
                        tr(lang, TextKey::NavColor),
                        self.state.page == Page::Color,
                        || self.state.page = Page::Color,
                    );
                    nav_button(
                        ui,
                        p,
                        IconKind::Display,
                        tr(lang, TextKey::NavDisplay),
                        self.state.page == Page::Display,
                        || self.state.page = Page::Display,
                    );
                    nav_button(
                        ui,
                        p,
                        IconKind::Graphics,
                        tr(lang, TextKey::NavGraphics),
                        self.state.page == Page::Graphics,
                        || self.state.page = Page::Graphics,
                    );
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    nav_button(
                        ui,
                        p,
                        IconKind::Settings,
                        tr(lang, TextKey::NavSettings),
                        self.state.page == Page::Settings,
                        || self.state.page = Page::Settings,
                    );
                    ui.add_space(6.0);
                    let sep = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [
                            egui::pos2(sep.left(), sep.bottom()),
                            egui::pos2(sep.right(), sep.bottom()),
                        ],
                        Stroke::new(1.0_f32, p.border_subtle),
                    );
                });
            });

        // Main Scrollable Area — zero outer margin on CentralPanel so the scrollbar docks to the window edge,
        // with generous inner padding inside the ScrollArea so cards never touch the scrollbar.
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg_window).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        Frame::new()
                            .inner_margin(Margin {
                                left: 26,
                                right: 24,
                                top: 20,
                                bottom: 24,
                            })
                            .show(ui, |ui| {
                                if self.state.needs_admin_banner {
                                    Frame::new()
                                        .fill(p.accent_soft)
                                        .stroke(Stroke::new(1.0_f32, p.warn))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("Driver yêu cầu quyền Administrator cho thao tác vừa rồi.")
                                                        .size(12.5)
                                                        .color(p.text_primary),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.button(tr(lang, TextKey::RelaunchAdmin)).clicked() {
                                                            if relaunch_as_administrator() {
                                                                ctx.send_viewport_cmd(
                                                                    egui::ViewportCommand::Close,
                                                                );
                                                            }
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(10.0);
                                }

                                match self.state.page {
                                    Page::Dashboard => pages::dashboard::render(ui, &mut self.state, p),
                                    Page::Color => pages::color::render(ui, &mut self.state, p),
                                    Page::Display => pages::display::render(ui, &mut self.state, p),
                                    Page::Graphics => pages::graphics::render(ui, &mut self.state, p),
                                    Page::Settings => pages::settings::render(ui, &mut self.state, p),
                                }
                            });
                    });

                if let Some((msg, _)) = &self.state.toast {
                    egui::Area::new(egui::Id::new("puwvga_toast"))
                        .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-18.0, -16.0))
                        .show(ctx, |ui| {
                            Frame::new()
                                .fill(p.bg_card)
                                .stroke(Stroke::new(1.0_f32, p.accent))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(14, 9))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(msg).size(12.5).color(p.text_primary),
                                    );
                                });
                        });
                }
            });
    }
}

fn nav_button(
    ui: &mut egui::Ui,
    p: Palette,
    icon: IconKind,
    label: &str,
    active: bool,
    on_click: impl FnOnce(),
) {
    let desired = Vec2::new(216.0, 36.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    if resp.clicked() {
        on_click();
    }

    if ui.is_rect_visible(rect) {
        let fill = if active || resp.hovered() {
            p.bg_card
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill);

        if active {
            let bar = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 3.0, rect.top() + 8.0),
                egui::pos2(rect.left() + 6.0, rect.bottom() - 8.0),
            );
            ui.painter()
                .rect_filled(bar, CornerRadius::same(2), p.accent);
        }

        let fg = if active {
            p.text_primary
        } else {
            p.text_secondary
        };

        draw_icon(
            ui.painter(),
            egui::pos2(rect.left() + 21.0, rect.center().y),
            14.0,
            fg,
            icon,
        );

        ui.painter().text(
            egui::pos2(rect.left() + 38.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            fg,
        );
    }
    ui.add_space(3.0);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("puwvga")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([880.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "puwvga",
        options,
        Box::new(|cc| Ok(Box::new(PuwvgaApp::new(cc)))),
    )
}
