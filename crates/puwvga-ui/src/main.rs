#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pages;
mod state;
mod theme;
mod widgets;

use eframe::egui;
use egui::{CornerRadius, Frame, Margin, RichText, Sense, Stroke, Vec2};
use puwvga_core::{tr, TextKey};
use puwvga_win32::relaunch_as_administrator;
use state::{AppState, Page};
use theme::{apply_fluent_visuals, configure_fonts, vendor_color, Palette};
use widgets::vendor_dot;
use std::time::{Duration, Instant};

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

        // Only wake up once per second while a 15-second display revert countdown is active.
        if self.state.pending_revert.is_some() {
            self.state.tick_revert();
            ctx.request_repaint_after(Duration::from_millis(500));
        }

        // Clear expired toast
        if let Some((_, until)) = &self.state.toast {
            if Instant::now() >= *until {
                self.state.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }

        let lang = self.state.language;

        // Left Sidebar (Lenovo Legion Toolkit style)
        egui::SidePanel::left("puwvga_sidebar")
            .exact_width(228.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(p.bg_sidebar)
                    .stroke(Stroke::new(1.0, p.border_subtle))
                    .inner_margin(Margin::symmetric(10, 12)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("puwvga")
                                .size(16.0)
                                .strong()
                                .color(p.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("Portable")
                                    .size(10.5)
                                    .color(p.text_muted),
                            );
                        });
                    });
                    ui.add_space(10.0);

                    // Detected GPUs card (auto-adapts to 1 or 2+ GPUs)
                    Frame::new()
                        .fill(p.bg_card)
                        .stroke(Stroke::new(1.0, p.border_subtle))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
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
                            ui.add_space(4.0);
                            let adapters = self.state.adapters.clone();
                            for (idx, g) in adapters.iter().enumerate() {
                                let active = idx == self.state.selected_gpu_idx;
                                let resp = Frame::new()
                                    .fill(if active {
                                        p.bg_control
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    })
                                    .stroke(Stroke::new(
                                        1.0,
                                        if active {
                                            p.border_strong
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        },
                                    ))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            vendor_dot(ui, vendor_color(g.vendor));
                                            ui.label(
                                                RichText::new(&g.name)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(if active {
                                                        p.text_primary
                                                    } else {
                                                        p.text_secondary
                                                    }),
                                            );
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());
                                if resp.clicked() {
                                    self.state.select_gpu(idx);
                                }
                                ui.add_space(2.0);
                            }
                        });

                    ui.add_space(12.0);

                    nav_button(
                        ui,
                        p,
                        tr(lang, TextKey::NavDashboard),
                        self.state.page == Page::Dashboard,
                        || self.state.page = Page::Dashboard,
                    );
                    nav_button(
                        ui,
                        p,
                        tr(lang, TextKey::NavColor),
                        self.state.page == Page::Color,
                        || self.state.page = Page::Color,
                    );
                    nav_button(
                        ui,
                        p,
                        tr(lang, TextKey::NavDisplay),
                        self.state.page == Page::Display,
                        || self.state.page = Page::Display,
                    );
                    nav_button(
                        ui,
                        p,
                        tr(lang, TextKey::NavGraphics),
                        self.state.page == Page::Graphics,
                        || self.state.page = Page::Graphics,
                    );
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    nav_button(
                        ui,
                        p,
                        tr(lang, TextKey::NavSettings),
                        self.state.page == Page::Settings,
                        || self.state.page = Page::Settings,
                    );
                });
            });

        // Main Scrollable Area
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(p.bg_window)
                    .inner_margin(Margin::symmetric(24, 18)),
            )
            .show(ctx, |ui| {
                if self.state.needs_admin_banner {
                    Frame::new()
                        .fill(p.accent_soft)
                        .stroke(Stroke::new(1.0, p.warn))
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

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match self.state.page {
                        Page::Dashboard => pages::dashboard::render(ui, &mut self.state, p),
                        Page::Color => pages::color::render(ui, &mut self.state, p),
                        Page::Display => pages::display::render(ui, &mut self.state, p),
                        Page::Graphics => pages::graphics::render(ui, &mut self.state, p),
                        Page::Settings => pages::settings::render(ui, &mut self.state, p),
                    });

                if let Some((msg, _)) = &self.state.toast {
                    egui::Area::new(egui::Id::new("puwvga_toast"))
                        .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-18.0, -16.0))
                        .show(ctx, |ui| {
                            Frame::new()
                                .fill(p.bg_card)
                                .stroke(Stroke::new(1.0, p.accent))
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
    label: &str,
    active: bool,
    on_click: impl FnOnce(),
) {
    let desired = Vec2::new(ui.available_width(), 34.0);
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

        ui.painter().text(
            egui::pos2(rect.left() + 16.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            if active {
                p.text_primary
            } else {
                p.text_secondary
            },
        );
    }
    ui.add_space(3.0);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("puwvga")
            .with_inner_size([1080.0, 680.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "puwvga",
        options,
        Box::new(|cc| Ok(Box::new(PuwvgaApp::new(cc)))),
    )
}
