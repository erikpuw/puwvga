#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;
mod shortcut;

use eframe::egui;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Frame, Margin, RichText, Stroke, Vec2};
use install::{
    default_install_dir, launch_installed_app, perform_install, perform_uninstall, InstallOptions,
    APP_NAME, APP_VERSION,
};
use puwvga_win32::is_windows_dark_mode;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Install,
    Uninstall,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Ready,
    Done,
    Failed,
}

struct InstallerApp {
    mode: Mode,
    is_vi: bool,
    dark: bool,
    install_dir_text: String,
    create_start_menu: bool,
    create_desktop: bool,
    launch_after: bool,
    status: Status,
    message: String,
    installed_exe: Option<PathBuf>,
}

impl InstallerApp {
    fn new(cc: &eframe::CreationContext<'_>, mode: Mode) -> Self {
        configure_fonts(&cc.egui_ctx);
        let dark = is_windows_dark_mode();
        apply_visuals(&cc.egui_ctx, dark);
        let dir = if mode == Mode::Uninstall {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(default_install_dir)
        } else {
            default_install_dir()
        };
        Self {
            mode,
            is_vi: true,
            dark,
            install_dir_text: dir.display().to_string(),
            create_start_menu: true,
            create_desktop: true,
            launch_after: true,
            status: Status::Ready,
            message: String::new(),
            installed_exe: None,
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (bg, card, border, text_pri, text_sec, accent) = if self.dark {
            (
                Color32::from_rgb(24, 24, 24),
                Color32::from_rgb(35, 35, 35),
                Color32::from_rgb(56, 56, 56),
                Color32::from_rgb(243, 243, 243),
                Color32::from_rgb(168, 168, 168),
                Color32::from_rgb(76, 194, 255),
            )
        } else {
            (
                Color32::from_rgb(243, 243, 243),
                Color32::from_rgb(255, 255, 255),
                Color32::from_rgb(224, 224, 224),
                Color32::from_rgb(24, 24, 24),
                Color32::from_rgb(96, 96, 96),
                Color32::from_rgb(0, 103, 192),
            )
        };

        egui::CentralPanel::default()
            .frame(Frame::new().fill(bg).inner_margin(Margin::same(22)))
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("{APP_NAME} v{APP_VERSION} Setup"))
                                .size(20.0)
                                .strong()
                                .color(text_pri),
                        );
                        ui.label(
                            RichText::new(if self.is_vi {
                                "Trình cài đặt bảng điều khiển VGA (Intel, NVIDIA, AMD)"
                            } else {
                                "Unified GPU Control Panel Installer (Intel, NVIDIA, AMD)"
                            })
                            .size(12.0)
                            .color(text_sec),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        let lang_btn = if self.is_vi { "EN" } else { "VI" };
                        if ui.button(lang_btn).clicked() {
                            self.is_vi = !self.is_vi;
                        }
                    });
                });

                ui.add_space(14.0);

                // Notice card (unsigned build transparency)
                Frame::new()
                    .fill(card)
                    .stroke(Stroke::new(1.0_f32, border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match self.mode {
                            Mode::Install => {
                                ui.label(
                                    RichText::new(if self.is_vi {
                                        "Thư mục cài đặt"
                                    } else {
                                        "Installation folder"
                                    })
                                    .size(12.5)
                                    .strong()
                                    .color(text_pri),
                                );
                                ui.add_space(4.0);
                                ui.add_sized(
                                    [ui.available_width(), 28.0],
                                    egui::TextEdit::singleline(&mut self.install_dir_text),
                                );
                                ui.add_space(10.0);
                                ui.checkbox(
                                    &mut self.create_start_menu,
                                    if self.is_vi {
                                        "Tạo lối tắt trong Start Menu"
                                    } else {
                                        "Create Start Menu shortcut"
                                    },
                                );
                                ui.checkbox(
                                    &mut self.create_desktop,
                                    if self.is_vi {
                                        "Tạo lối tắt ngoài Desktop"
                                    } else {
                                        "Create Desktop shortcut"
                                    },
                                );
                                ui.checkbox(
                                    &mut self.launch_after,
                                    if self.is_vi {
                                        "Mở puwvga sau khi cài xong"
                                    } else {
                                        "Launch puwvga after installation"
                                    },
                                );
                            }
                            Mode::Uninstall => {
                                ui.label(
                                    RichText::new(if self.is_vi {
                                        "Gỡ cài đặt puwvga khỏi máy tính này"
                                    } else {
                                        "Uninstall puwvga from this computer"
                                    })
                                    .size(13.0)
                                    .strong()
                                    .color(text_pri),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        if self.is_vi { "Thư mục" } else { "Folder" },
                                        self.install_dir_text
                                    ))
                                    .size(12.0)
                                    .color(text_sec),
                                );
                            }
                        }
                    });

                ui.add_space(10.0);

                ui.label(
                    RichText::new(if self.is_vi {
                        "Giấy phép: Apache License 2.0 · Bản cài đặt nội bộ chưa có chữ ký số (Unsigned)"
                    } else {
                        "License: Apache License 2.0 · Unsigned installer build"
                    })
                    .size(11.0)
                    .color(text_sec),
                );

                if !self.message.is_empty() {
                    ui.add_space(10.0);
                    let msg_col = match self.status {
                        Status::Done => accent,
                        Status::Failed => Color32::from_rgb(255, 110, 110),
                        Status::Ready => text_sec,
                    };
                    ui.label(RichText::new(&self.message).size(12.5).color(msg_col));
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.horizontal(|ui| {
                        if self.status == Status::Done {
                            let finish_txt = if self.is_vi { "Hoàn tất" } else { "Finish" };
                            if ui.add_sized([110.0, 32.0], egui::Button::new(finish_txt)).clicked() {
                                if self.mode == Mode::Install && self.launch_after {
                                    if let Some(exe) = &self.installed_exe {
                                        launch_installed_app(exe);
                                    }
                                }
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        } else {
                            let primary_txt = match (self.mode, self.is_vi) {
                                (Mode::Install, true) => "Cài đặt",
                                (Mode::Install, false) => "Install",
                                (Mode::Uninstall, true) => "Gỡ cài đặt",
                                (Mode::Uninstall, false) => "Uninstall",
                            };
                            if ui
                                .add_sized([120.0, 32.0], egui::Button::new(primary_txt))
                                .clicked()
                            {
                                match self.mode {
                                    Mode::Install => {
                                        let opts = InstallOptions {
                                            install_dir: PathBuf::from(self.install_dir_text.trim()),
                                            create_start_menu: self.create_start_menu,
                                            create_desktop: self.create_desktop,
                                        };
                                        match perform_install(&opts) {
                                            Ok(exe) => {
                                                self.installed_exe = Some(exe);
                                                self.status = Status::Done;
                                                self.message = if self.is_vi {
                                                    "Đã cài đặt thành công puwvga v1.0.1!".into()
                                                } else {
                                                    "puwvga v1.0.1 installed successfully!".into()
                                                };
                                            }
                                            Err(err) => {
                                                self.status = Status::Failed;
                                                self.message = err;
                                            }
                                        }
                                    }
                                    Mode::Uninstall => {
                                        let dir = PathBuf::from(self.install_dir_text.trim());
                                        match perform_uninstall(&dir) {
                                            Ok(()) => {
                                                self.status = Status::Done;
                                                self.message = if self.is_vi {
                                                    "Đã gỡ cài đặt puwvga khỏi máy.".into()
                                                } else {
                                                    "puwvga has been uninstalled.".into()
                                                };
                                            }
                                            Err(err) => {
                                                self.status = Status::Failed;
                                                self.message = err;
                                            }
                                        }
                                    }
                                }
                            }

                            let cancel_txt = if self.is_vi { "Đóng" } else { "Cancel" };
                            if ui.add_sized([90.0, 32.0], egui::Button::new(cancel_txt)).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        }
                    });
                });
            });
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoeui".to_owned(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoeui".to_owned());
        ctx.set_fonts(fonts);
    }
}

fn apply_visuals(ctx: &egui::Context, dark: bool) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
}

fn load_window_icon() -> egui::IconData {
    const ICON_RGBA: &[u8] = include_bytes!("../../../assets/puwvga-icon.rgba");
    egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 64,
        height: 64,
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let self_is_uninstaller = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()))
        .map(|n| n.contains("uninstall"))
        .unwrap_or(false);

    if args.iter().any(|a| a == "--silent-install" || a == "/S") {
        let opts = InstallOptions {
            install_dir: default_install_dir(),
            create_start_menu: true,
            create_desktop: true,
        };
        let _ = perform_install(&opts);
        return Ok(());
    }
    if args.iter().any(|a| a == "--silent-uninstall") {
        let dir = default_install_dir();
        let _ = perform_uninstall(&dir);
        return Ok(());
    }

    let mode = if self_is_uninstaller || args.iter().any(|a| a == "--uninstall") {
        Mode::Uninstall
    } else {
        Mode::Install
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{APP_NAME} v{APP_VERSION} Setup"))
            .with_icon(Arc::new(load_window_icon()))
            .with_inner_size(Vec2::new(520.0, 340.0))
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "puwvga Setup",
        options,
        Box::new(move |cc| Ok(Box::new(InstallerApp::new(cc, mode)))),
    )
}
