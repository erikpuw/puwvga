#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;
mod shortcut;

use install::{
    default_install_dir, launch_installed_app, perform_install, perform_uninstall, InstallOptions,
    APP_NAME, APP_VERSION,
};
use puwvga_win32::is_windows_dark_mode;
use std::ffi::c_void;
use std::path::PathBuf;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetMonitorInfoW,
    InvalidateRect, MonitorFromPoint, Polyline, RoundRect, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, FF_DONTCARE, HBRUSH, HDC, HFONT,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, SRCCOPY,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE, VK_RETURN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, LoadCursorW, LoadIconW,
    MoveWindow, PostQuitMessage, RegisterClassExW, SendMessageW, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW,
    ES_AUTOHSCROLL, GWLP_USERDATA, HMENU, IDC_ARROW, IDC_HAND, MSG, SW_SHOW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_CHAR, WM_COMMAND, WM_CREATE, WM_CTLCOLOREDIT, WM_DESTROY, WM_DPICHANGED,
    WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE,
    WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETFONT, WNDCLASSEXW, WS_CAPTION, WS_CHILD,
    WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

const LOGICAL_W: i32 = 540;
const LOGICAL_H_INSTALL: i32 = 376;
const LOGICAL_H_UNINSTALL: i32 = 276;
const ID_EDIT_DIR: usize = 101;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum HitId {
    LangToggle,
    CheckStartMenu,
    CheckDesktop,
    CheckStartup,
    CheckLaunch,
    PrimaryBtn,
    CancelBtn,
}

#[derive(Clone, Copy)]
struct Palette {
    bg: COLORREF,
    card: COLORREF,
    card_border: COLORREF,
    input_bg: COLORREF,
    input_border: COLORREF,
    text_pri: COLORREF,
    text_sec: COLORREF,
    accent: COLORREF,
    accent_hover: COLORREF,
    accent_text: COLORREF,
    btn_bg: COLORREF,
    btn_hover: COLORREF,
    btn_border: COLORREF,
    check_border: COLORREF,
    err: COLORREF,
}

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

impl Palette {
    fn for_mode(dark: bool) -> Self {
        if dark {
            Self {
                bg: rgb(24, 24, 24),
                card: rgb(35, 35, 35),
                card_border: rgb(56, 56, 56),
                input_bg: rgb(28, 28, 28),
                input_border: rgb(74, 74, 74),
                text_pri: rgb(243, 243, 243),
                text_sec: rgb(168, 168, 168),
                accent: rgb(76, 194, 255),
                accent_hover: rgb(104, 206, 255),
                accent_text: rgb(12, 16, 20),
                btn_bg: rgb(46, 46, 46),
                btn_hover: rgb(58, 58, 58),
                btn_border: rgb(72, 72, 72),
                check_border: rgb(128, 128, 128),
                err: rgb(255, 110, 110),
            }
        } else {
            Self {
                bg: rgb(243, 243, 243),
                card: rgb(255, 255, 255),
                card_border: rgb(224, 224, 224),
                input_bg: rgb(255, 255, 255),
                input_border: rgb(196, 196, 196),
                text_pri: rgb(24, 24, 24),
                text_sec: rgb(96, 96, 96),
                accent: rgb(0, 103, 192),
                accent_hover: rgb(0, 120, 212),
                accent_text: rgb(255, 255, 255),
                btn_bg: rgb(251, 251, 251),
                btn_hover: rgb(240, 240, 240),
                btn_border: rgb(206, 206, 206),
                check_border: rgb(134, 134, 134),
                err: rgb(196, 43, 28),
            }
        }
    }
}

struct Fonts {
    title: HFONT,
    subtitle: HFONT,
    label_bold: HFONT,
    body: HFONT,
    notice: HFONT,
    button: HFONT,
}

impl Fonts {
    fn create(dpi: u32) -> Self {
        Self {
            title: make_font(22, 700, dpi),
            subtitle: make_font(13, 400, dpi),
            label_bold: make_font(14, 600, dpi),
            body: make_font(13, 400, dpi),
            notice: make_font(12, 400, dpi),
            button: make_font(13, 600, dpi),
        }
    }

    fn destroy(&mut self) {
        unsafe {
            for f in [
                self.title,
                self.subtitle,
                self.label_bold,
                self.body,
                self.notice,
                self.button,
            ] {
                if !f.0.is_null() {
                    let _ = DeleteObject(f);
                }
            }
        }
        self.title = HFONT::default();
        self.subtitle = HFONT::default();
        self.label_bold = HFONT::default();
        self.body = HFONT::default();
        self.notice = HFONT::default();
        self.button = HFONT::default();
    }
}

fn make_font(px_at_96: i32, weight: i32, dpi: u32) -> HFONT {
    let height = -scale(px_at_96, dpi);
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            0,
            CLEARTYPE_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Segoe UI"),
        )
    }
}

#[inline]
fn scale(px: i32, dpi: u32) -> i32 {
    (px * dpi as i32 + 48) / 96
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

fn pt_in_rect(r: RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

struct Layout {
    lang_btn: RECT,
    card: RECT,
    edit_box: RECT,
    edit_inner: RECT,
    checks: [RECT; 4],
    notice: RECT,
    message: RECT,
    primary_btn: RECT,
    cancel_btn: RECT,
}

struct UiState {
    mode: Mode,
    is_vi: bool,
    dark: bool,
    dpi: u32,
    install_dir_text: String,
    create_start_menu: bool,
    create_desktop: bool,
    enable_startup_task: bool,
    launch_after: bool,
    status: Status,
    message: String,
    installed_exe: Option<PathBuf>,
    hovered: Option<HitId>,
    pressed: Option<HitId>,
    tracking_mouse: bool,
    edit_hwnd: HWND,
    fonts: Fonts,
    input_brush: HBRUSH,
}

impl Drop for UiState {
    fn drop(&mut self) {
        self.fonts.destroy();
        if !self.input_brush.0.is_null() {
            unsafe {
                let _ = DeleteObject(self.input_brush);
            }
        }
    }
}

impl UiState {
    fn new(mode: Mode, dpi: u32) -> Self {
        let dark = is_windows_dark_mode();
        let pal = Palette::for_mode(dark);
        let dir = if mode == Mode::Uninstall {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(default_install_dir)
        } else {
            default_install_dir()
        };
        let input_brush = unsafe { CreateSolidBrush(pal.input_bg) };
        Self {
            mode,
            is_vi: true,
            dark,
            dpi,
            install_dir_text: dir.display().to_string(),
            create_start_menu: true,
            create_desktop: true,
            enable_startup_task: true,
            launch_after: true,
            status: Status::Ready,
            message: String::new(),
            installed_exe: None,
            hovered: None,
            pressed: None,
            tracking_mouse: false,
            edit_hwnd: HWND::default(),
            fonts: Fonts::create(dpi),
            input_brush,
        }
    }

    fn layout(&self, cw: i32, ch: i32) -> Layout {
        let s = |v: i32| scale(v, self.dpi);
        let pad = s(22);
        let content_w = cw - pad * 2;

        let lang_w = s(38);
        let lang_h = s(26);
        let lang_btn = rect(cw - pad - lang_w, pad, lang_w, lang_h);

        let card_top = pad + s(54);
        let card_h = match self.mode {
            Mode::Install => s(196),
            Mode::Uninstall => s(96),
        };
        let card = rect(pad, card_top, content_w, card_h);

        let inner_x = card.left + s(14);
        let inner_w = content_w - s(28);
        let edit_box = rect(inner_x, card.top + s(38), inner_w, s(30));
        let edit_inner = rect(
            edit_box.left + s(8),
            edit_box.top + s(6),
            (edit_box.right - edit_box.left - s(16)).max(40),
            (edit_box.bottom - edit_box.top - s(10)).max(16),
        );

        let row_h = s(26);
        let row_y0 = edit_box.bottom + s(12);
        let checks = [
            rect(inner_x, row_y0, inner_w, row_h),
            rect(inner_x, row_y0 + row_h, inner_w, row_h),
            rect(inner_x, row_y0 + row_h * 2, inner_w, row_h),
            rect(inner_x, row_y0 + row_h * 3, inner_w, row_h),
        ];

        let notice = rect(pad, card.bottom + s(10), content_w, s(20));
        let message = rect(pad, notice.bottom + s(4), content_w, s(22));

        let btn_h = s(32);
        let btn_y = ch - pad - btn_h;
        let primary_w = if self.status == Status::Done { s(112) } else { s(120) };
        let cancel_w = s(90);
        let gap = s(8);

        let primary_btn = if self.status == Status::Done {
            rect(cw - pad - primary_w, btn_y, primary_w, btn_h)
        } else {
            rect(cw - pad - cancel_w - gap - primary_w, btn_y, primary_w, btn_h)
        };
        let cancel_btn = rect(cw - pad - cancel_w, btn_y, cancel_w, btn_h);

        Layout {
            lang_btn,
            card,
            edit_box,
            edit_inner,
            checks,
            notice,
            message,
            primary_btn,
            cancel_btn,
        }
    }

    fn hit_test(&self, cw: i32, ch: i32, x: i32, y: i32) -> Option<HitId> {
        let l = self.layout(cw, ch);
        if pt_in_rect(l.lang_btn, x, y) {
            return Some(HitId::LangToggle);
        }
        if self.mode == Mode::Install && self.status != Status::Done {
            let ids = [
                HitId::CheckStartMenu,
                HitId::CheckDesktop,
                HitId::CheckStartup,
                HitId::CheckLaunch,
            ];
            for (r, id) in l.checks.iter().zip(ids) {
                if pt_in_rect(*r, x, y) {
                    return Some(id);
                }
            }
        }
        if pt_in_rect(l.primary_btn, x, y) {
            return Some(HitId::PrimaryBtn);
        }
        if self.status != Status::Done && pt_in_rect(l.cancel_btn, x, y) {
            return Some(HitId::CancelBtn);
        }
        None
    }

    fn sync_dir_from_edit(&mut self) {
        if self.edit_hwnd.0.is_null() {
            return;
        }
        unsafe {
            let len = GetWindowTextLengthW(self.edit_hwnd);
            if len >= 0 {
                let mut buf = vec![0u16; (len as usize) + 1];
                let copied = GetWindowTextW(self.edit_hwnd, &mut buf);
                self.install_dir_text = String::from_utf16_lossy(&buf[..copied as usize]);
            }
        }
    }

    fn position_edit(&self, cw: i32, ch: i32) {
        if self.edit_hwnd.0.is_null() {
            return;
        }
        let l = self.layout(cw, ch);
        let r = l.edit_inner;
        unsafe {
            let _ = MoveWindow(
                self.edit_hwnd,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                true,
            );
            SendMessageW(
                self.edit_hwnd,
                WM_SETFONT,
                WPARAM(self.fonts.body.0 as usize),
                LPARAM(1),
            );
        }
    }

    fn handle_primary(&mut self, hwnd: HWND) {
        if self.status == Status::Done {
            if self.mode == Mode::Install && self.launch_after {
                if let Some(exe) = &self.installed_exe {
                    launch_installed_app(exe);
                }
            }
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            return;
        }

        self.sync_dir_from_edit();
        let trimmed = self.install_dir_text.trim();
        let dir = if trimmed.is_empty() {
            default_install_dir()
        } else {
            PathBuf::from(trimmed)
        };

        match self.mode {
            Mode::Install => {
                let opts = InstallOptions {
                    install_dir: dir,
                    create_start_menu: self.create_start_menu,
                    create_desktop: self.create_desktop,
                    enable_startup_task: self.enable_startup_task,
                };
                match perform_install(&opts) {
                    Ok(exe) => {
                        self.installed_exe = Some(exe);
                        self.status = Status::Done;
                        self.message = if self.is_vi {
                            format!("Đã cài đặt thành công {APP_NAME} v{APP_VERSION}!")
                        } else {
                            format!("{APP_NAME} v{APP_VERSION} installed successfully!")
                        };
                    }
                    Err(err) => {
                        self.status = Status::Failed;
                        self.message = err;
                    }
                }
            }
            Mode::Uninstall => match perform_uninstall(&dir) {
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
            },
        }
    }

    fn paint(&self, hdc: HDC, cw: i32, ch: i32) {
        let pal = Palette::for_mode(self.dark);
        let s = |v: i32| scale(v, self.dpi);
        let l = self.layout(cw, ch);

        unsafe {
            SetBkMode(hdc, TRANSPARENT);

            // Background
            let bg_brush = CreateSolidBrush(pal.bg);
            let full = rect(0, 0, cw, ch);
            FillRect(hdc, &full, bg_brush);
            let _ = DeleteObject(bg_brush);

            // Header
            let pad = s(22);
            let title_str = format!("{APP_NAME} v{APP_VERSION} Setup");
            draw_text(
                hdc,
                &title_str,
                rect(pad, pad - s(2), cw - pad * 2 - s(48), s(28)),
                self.fonts.title,
                pal.text_pri,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            let subtitle_str = if self.is_vi {
                "Trình cài đặt bảng điều khiển VGA (Intel, NVIDIA, AMD)"
            } else {
                "Unified GPU Control Panel Installer (Intel, NVIDIA, AMD)"
            };
            draw_text(
                hdc,
                subtitle_str,
                rect(pad, pad + s(27), cw - pad * 2 - s(48), s(20)),
                self.fonts.subtitle,
                pal.text_sec,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Language toggle button
            let lang_label = if self.is_vi { "EN" } else { "VI" };
            let lang_bg = if self.hovered == Some(HitId::LangToggle) {
                pal.btn_hover
            } else {
                pal.btn_bg
            };
            draw_round_box(hdc, l.lang_btn, s(6), lang_bg, pal.btn_border);
            draw_text(
                hdc,
                lang_label,
                l.lang_btn,
                self.fonts.button,
                pal.text_pri,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Main card
            draw_round_box(hdc, l.card, s(8), pal.card, pal.card_border);

            match self.mode {
                Mode::Install => {
                    let folder_lbl = if self.is_vi {
                        "Thư mục cài đặt"
                    } else {
                        "Installation folder"
                    };
                    draw_text(
                        hdc,
                        folder_lbl,
                        rect(l.card.left + s(14), l.card.top + s(13), l.card.right - l.card.left - s(28), s(20)),
                        self.fonts.label_bold,
                        pal.text_pri,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );

                    // Rounded border around native EDIT control
                    draw_round_box(hdc, l.edit_box, s(6), pal.input_bg, pal.input_border);

                    let items = [
                        (
                            self.create_start_menu,
                            HitId::CheckStartMenu,
                            if self.is_vi {
                                "Tạo lối tắt trong Start Menu"
                            } else {
                                "Create Start Menu shortcut"
                            },
                        ),
                        (
                            self.create_desktop,
                            HitId::CheckDesktop,
                            if self.is_vi {
                                "Tạo lối tắt ngoài Desktop"
                            } else {
                                "Create Desktop shortcut"
                            },
                        ),
                        (
                            self.enable_startup_task,
                            HitId::CheckStartup,
                            if self.is_vi {
                                "Tự áp màu cùng Windows (Task Scheduler, không chạy nền)"
                            } else {
                                "Auto-apply color with Windows (Task Scheduler, no background process)"
                            },
                        ),
                        (
                            self.launch_after,
                            HitId::CheckLaunch,
                            if self.is_vi {
                                "Mở puwvga sau khi cài xong"
                            } else {
                                "Launch puwvga after installation"
                            },
                        ),
                    ];
                    for (idx, (checked, id, label)) in items.iter().enumerate() {
                        draw_checkbox(
                            hdc,
                            l.checks[idx],
                            *checked,
                            self.hovered == Some(*id),
                            label,
                            self.fonts.body,
                            pal,
                            self.dpi,
                        );
                    }
                }
                Mode::Uninstall => {
                    let heading = if self.is_vi {
                        "Gỡ cài đặt puwvga khỏi máy tính này"
                    } else {
                        "Uninstall puwvga from this computer"
                    };
                    draw_text(
                        hdc,
                        heading,
                        rect(l.card.left + s(14), l.card.top + s(16), l.card.right - l.card.left - s(28), s(22)),
                        self.fonts.label_bold,
                        pal.text_pri,
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );
                    let folder_line = format!(
                        "{}: {}",
                        if self.is_vi { "Thư mục" } else { "Folder" },
                        self.install_dir_text
                    );
                    draw_text(
                        hdc,
                        &folder_line,
                        rect(l.card.left + s(14), l.card.top + s(44), l.card.right - l.card.left - s(28), s(38)),
                        self.fonts.body,
                        pal.text_sec,
                        DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
                    );
                }
            }

            // Official GitHub notice
            let notice_str = if self.is_vi {
                "Bản cài đặt chưa có chữ ký số (Unsigned) · Chỉ tải từ github.com/erikpuw/puwvga"
            } else {
                "Unsigned installer build · Download only from github.com/erikpuw/puwvga"
            };
            draw_text(
                hdc,
                notice_str,
                l.notice,
                self.fonts.notice,
                pal.text_sec,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Status message
            if !self.message.is_empty() {
                let msg_col = match self.status {
                    Status::Done => pal.accent,
                    Status::Failed => pal.err,
                    Status::Ready => pal.text_sec,
                };
                draw_text(
                    hdc,
                    &self.message,
                    l.message,
                    self.fonts.label_bold,
                    msg_col,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }

            // Bottom buttons
            if self.status == Status::Done {
                let finish_txt = if self.is_vi { "Hoàn tất" } else { "Finish" };
                let bg = if self.hovered == Some(HitId::PrimaryBtn) {
                    pal.accent_hover
                } else {
                    pal.accent
                };
                draw_round_box(hdc, l.primary_btn, s(6), bg, bg);
                draw_text(
                    hdc,
                    finish_txt,
                    l.primary_btn,
                    self.fonts.button,
                    pal.accent_text,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            } else {
                let primary_txt = match (self.mode, self.is_vi) {
                    (Mode::Install, true) => "Cài đặt",
                    (Mode::Install, false) => "Install",
                    (Mode::Uninstall, true) => "Gỡ cài đặt",
                    (Mode::Uninstall, false) => "Uninstall",
                };
                let p_bg = if self.hovered == Some(HitId::PrimaryBtn) {
                    pal.accent_hover
                } else {
                    pal.accent
                };
                draw_round_box(hdc, l.primary_btn, s(6), p_bg, p_bg);
                draw_text(
                    hdc,
                    primary_txt,
                    l.primary_btn,
                    self.fonts.button,
                    pal.accent_text,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );

                let cancel_txt = if self.is_vi { "Đóng" } else { "Cancel" };
                let c_bg = if self.hovered == Some(HitId::CancelBtn) {
                    pal.btn_hover
                } else {
                    pal.btn_bg
                };
                draw_round_box(hdc, l.cancel_btn, s(6), c_bg, pal.btn_border);
                draw_text(
                    hdc,
                    cancel_txt,
                    l.cancel_btn,
                    self.fonts.button,
                    pal.text_pri,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
            }
        }
    }
}

unsafe fn draw_round_box(hdc: HDC, r: RECT, radius: i32, fill: COLORREF, border: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, border);
        let old_b = SelectObject(hdc, brush);
        let old_p = SelectObject(hdc, pen);
        let _ = RoundRect(hdc, r.left, r.top, r.right, r.bottom, radius * 2, radius * 2);
        SelectObject(hdc, old_b);
        SelectObject(hdc, old_p);
        let _ = DeleteObject(brush);
        let _ = DeleteObject(pen);
    }
}

unsafe fn draw_text(
    hdc: HDC,
    text: &str,
    mut r: RECT,
    font: HFONT,
    color: COLORREF,
    flags: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
) {
    unsafe {
        let old_f = SelectObject(hdc, font);
        SetTextColor(hdc, color);
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(hdc, &mut wide, &mut r, flags);
        SelectObject(hdc, old_f);
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn draw_checkbox(
    hdc: HDC,
    row: RECT,
    checked: bool,
    hovered: bool,
    label: &str,
    font: HFONT,
    pal: Palette,
    dpi: u32,
) {
    unsafe {
        let s = |v: i32| scale(v, dpi);
        let box_sz = s(16);
        let cy = (row.top + row.bottom) / 2;
        let bx = row.left;
        let by = cy - box_sz / 2;
        let box_rect = rect(bx, by, box_sz, box_sz);

        let (fill, border) = if checked {
            let col = if hovered { pal.accent_hover } else { pal.accent };
            (col, col)
        } else {
            let bg = if hovered { pal.btn_hover } else { pal.input_bg };
            (bg, pal.check_border)
        };
        draw_round_box(hdc, box_rect, s(4), fill, border);

        if checked {
            let pen_w = s(2).max(1);
            let pen = CreatePen(PS_SOLID, pen_w, pal.accent_text);
            let old_p = SelectObject(hdc, pen);
            let pts = [
                POINT {
                    x: bx + s(4),
                    y: by + s(8),
                },
                POINT {
                    x: bx + s(7),
                    y: by + s(11),
                },
                POINT {
                    x: bx + s(12),
                    y: by + s(5),
                },
            ];
            let _ = Polyline(hdc, &pts);
            SelectObject(hdc, old_p);
            let _ = DeleteObject(pen);
        }

        let text_r = RECT {
            left: bx + box_sz + s(9),
            top: row.top,
            right: row.right,
            bottom: row.bottom,
        };
        draw_text(
            hdc,
            label,
            text_r,
            font,
            pal.text_pri,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

#[inline]
fn lo_word(l: LPARAM) -> i32 {
    (l.0 & 0xFFFF) as i16 as i32
}

#[inline]
fn hi_word(l: LPARAM) -> i32 {
    ((l.0 >> 16) & 0xFFFF) as i16 as i32
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut UiState> {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
        ptr.as_mut()
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCREATE => {
                let cs = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CREATE => {
                if let Some(st) = get_state(hwnd) {
                    if st.dark {
                        let val: i32 = 1;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_USE_IMMERSIVE_DARK_MODE,
                            &val as *const i32 as *const c_void,
                            size_of::<i32>() as u32,
                        );
                    }

                    if st.mode == Mode::Install {
                        let hinst = GetModuleHandleW(None).unwrap_or_default();
                        let text_h = HSTRING::from(&st.install_dir_text);
                        if let Ok(edit) = CreateWindowExW(
                            WINDOW_EX_STYLE(0),
                            w!("EDIT"),
                            PCWSTR(text_h.as_ptr()),
                            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                            0,
                            0,
                            100,
                            20,
                            hwnd,
                            HMENU(ID_EDIT_DIR as *mut c_void),
                            hinst,
                            None,
                        ) {
                            st.edit_hwnd = edit;
                            let mut cr = RECT::default();
                            let _ = GetClientRect(hwnd, &mut cr);
                            st.position_edit(cr.right - cr.left, cr.bottom - cr.top);
                        }
                    }
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_CTLCOLOREDIT => {
                if let Some(st) = get_state(hwnd) {
                    let pal = Palette::for_mode(st.dark);
                    let hdc = HDC(wparam.0 as *mut c_void);
                    SetTextColor(hdc, pal.text_pri);
                    SetBkColor(hdc, pal.input_bg);
                    return LRESULT(st.input_brush.0 as isize);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if let Some(st) = get_state(hwnd) {
                    let mut cr = RECT::default();
                    let _ = GetClientRect(hwnd, &mut cr);
                    let cw = (cr.right - cr.left).max(1);
                    let ch = (cr.bottom - cr.top).max(1);

                    let mem_dc = CreateCompatibleDC(hdc);
                    let mem_bmp = CreateCompatibleBitmap(hdc, cw, ch);
                    let old_bmp = SelectObject(mem_dc, mem_bmp);

                    st.paint(mem_dc, cw, ch);
                    let _ = BitBlt(hdc, 0, 0, cw, ch, mem_dc, 0, 0, SRCCOPY);

                    SelectObject(mem_dc, old_bmp);
                    let _ = DeleteObject(mem_bmp);
                    let _ = DeleteDC(mem_dc);
                }
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if let Some(st) = get_state(hwnd) {
                    if !st.tracking_mouse {
                        let mut tme = TRACKMOUSEEVENT {
                            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        let _ = TrackMouseEvent(&mut tme);
                        st.tracking_mouse = true;
                    }
                    let mut cr = RECT::default();
                    let _ = GetClientRect(hwnd, &mut cr);
                    let hit = st.hit_test(cr.right - cr.left, cr.bottom - cr.top, lo_word(lparam), hi_word(lparam));
                    if hit != st.hovered {
                        st.hovered = hit;
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                }
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                if let Some(st) = get_state(hwnd) {
                    st.tracking_mouse = false;
                    if st.hovered.is_some() || st.pressed.is_some() {
                        st.hovered = None;
                        st.pressed = None;
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                }
                LRESULT(0)
            }
            WM_SETCURSOR => {
                if let Some(st) = get_state(hwnd) {
                    if (lparam.0 & 0xFFFF) as u32 == 1 && st.hovered.is_some() {
                        if let Ok(cur) = LoadCursorW(None, IDC_HAND) {
                            windows::Win32::UI::WindowsAndMessaging::SetCursor(cur);
                            return LRESULT(1);
                        }
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_LBUTTONDOWN => {
                if let Some(st) = get_state(hwnd) {
                    let mut cr = RECT::default();
                    let _ = GetClientRect(hwnd, &mut cr);
                    st.pressed = st.hit_test(cr.right - cr.left, cr.bottom - cr.top, lo_word(lparam), hi_word(lparam));
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if let Some(st) = get_state(hwnd) {
                    let mut cr = RECT::default();
                    let _ = GetClientRect(hwnd, &mut cr);
                    let hit = st.hit_test(cr.right - cr.left, cr.bottom - cr.top, lo_word(lparam), hi_word(lparam));
                    let clicked = st.pressed.take().filter(|p| Some(*p) == hit);
                    if let Some(id) = clicked {
                        match id {
                            HitId::LangToggle => st.is_vi = !st.is_vi,
                            HitId::CheckStartMenu => st.create_start_menu = !st.create_start_menu,
                            HitId::CheckDesktop => st.create_desktop = !st.create_desktop,
                            HitId::CheckStartup => st.enable_startup_task = !st.enable_startup_task,
                            HitId::CheckLaunch => st.launch_after = !st.launch_after,
                            HitId::PrimaryBtn => st.handle_primary(hwnd),
                            HitId::CancelBtn => {
                                let _ = DestroyWindow(hwnd);
                                return LRESULT(0);
                            }
                        }
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if let Some(st) = get_state(hwnd) {
                    let vk = wparam.0 as u16;
                    if vk == VK_ESCAPE.0 {
                        let _ = DestroyWindow(hwnd);
                        return LRESULT(0);
                    } else if vk == VK_RETURN.0 {
                        st.handle_primary(hwnd);
                        let _ = InvalidateRect(hwnd, None, false);
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CHAR => DefWindowProcW(hwnd, msg, wparam, lparam),
            WM_COMMAND => {
                if let Some(st) = get_state(hwnd) {
                    if (wparam.0 & 0xFFFF) == ID_EDIT_DIR {
                        st.sync_dir_from_edit();
                    }
                }
                LRESULT(0)
            }
            WM_DPICHANGED => {
                if let Some(st) = get_state(hwnd) {
                    let new_dpi = (wparam.0 & 0xFFFF) as u32;
                    if new_dpi > 0 && new_dpi != st.dpi {
                        st.dpi = new_dpi;
                        st.fonts.destroy();
                        st.fonts = Fonts::create(new_dpi);
                        let suggested = &*(lparam.0 as *const RECT);
                        let _ = MoveWindow(
                            hwnd,
                            suggested.left,
                            suggested.top,
                            suggested.right - suggested.left,
                            suggested.bottom - suggested.top,
                            true,
                        );
                        let mut cr = RECT::default();
                        let _ = GetClientRect(hwnd, &mut cr);
                        st.position_edit(cr.right - cr.left, cr.bottom - cr.top);
                        let _ = InvalidateRect(hwnd, None, false);
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UiState;
                if !ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(ptr));
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn active_monitor_work_and_dpi() -> (RECT, u32) {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);

        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            mi.rcWork
        } else {
            rect(0, 0, 1920, 1080)
        };

        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_err() || dpi_x == 0
        {
            dpi_x = 96;
        }
        (work, dpi_x)
    }
}

fn main() {
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
            enable_startup_task: true,
        };
        let _ = perform_install(&opts);
        return;
    }
    if args.iter().any(|a| a == "--silent-uninstall") {
        let dir = default_install_dir();
        let _ = perform_uninstall(&dir);
        return;
    }

    let mode = if self_is_uninstaller || args.iter().any(|a| a == "--uninstall") {
        Mode::Uninstall
    } else {
        Mode::Install
    };

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class_name = w!("PuwvgaSetupWindow");
        let icon = LoadIconW(hinst, PCWSTR(1 as *const u16)).unwrap_or_default();
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.into(),
            hIcon: icon,
            hIconSm: icon,
            hCursor: cursor,
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return;
        }

        let (work, dpi) = active_monitor_work_and_dpi();
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let ex_style = WINDOW_EX_STYLE(0);
        let logical_h = match mode {
            Mode::Install => LOGICAL_H_INSTALL,
            Mode::Uninstall => LOGICAL_H_UNINSTALL,
        };
        let mut wr = rect(0, 0, scale(LOGICAL_W, dpi), scale(logical_h, dpi));
        let _ = AdjustWindowRectExForDpi(&mut wr, style, false, ex_style, dpi);
        let win_w = wr.right - wr.left;
        let win_h = wr.bottom - wr.top;
        let x = work.left + ((work.right - work.left) - win_w) / 2;
        let y = work.top + ((work.bottom - work.top) - win_h) / 2;

        let title = HSTRING::from(format!("{APP_NAME} v{APP_VERSION} Setup"));
        let state_ptr = Box::into_raw(Box::new(UiState::new(mode, dpi)));

        let Ok(hwnd) = CreateWindowExW(
            ex_style,
            class_name,
            PCWSTR(title.as_ptr()),
            style,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            hinst,
            Some(state_ptr as *const c_void),
        ) else {
            drop(Box::from_raw(state_ptr));
            return;
        };

        let _ = SetWindowTextW(hwnd, PCWSTR(title.as_ptr()));
        let _ = ShowWindow(hwnd, SW_SHOW);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
