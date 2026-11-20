# `puwvga` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `puwvga`, a single portable Pure Rust Windows 10/11 GPU control panel (with statically linked VC++ CRT, zero WebView, zero shell spawning, zero telemetry/polling) that auto-detects installed Intel, NVIDIA, and AMD GPUs (single PC GPU or dual laptop iGPU + dGPU) and controls Color/Brightness, Display/Spanning/MUX, and Vendor-Native 3D/Video through official driver DLLs.

**Architecture:** Cargo workspace with 6 focused crates: `puwvga-core` (shared models, traits, i18n VI/EN, portable JSON presets), `puwvga-win32` (DXGI GPU enum, Win32 CCD display routing/resolution/Hz, Windows theme & DWM dark titlebar), 3 dynamic-FFI vendor crates (`puwvga-intel` via `ControlLib.dll`, `puwvga-nvidia` via `nvapi64.dll` + DRS, `puwvga-amd` via `atiadlxx.dll`), and `puwvga-ui` (Pure Rust `eframe`/`egui` modeled 1:1 on the approved Lenovo Legion Toolkit mockup).

**Tech Stack:** Rust 1.98+ (`x86_64-pc-windows-msvc` + `-C target-feature=+crt-static`), `libloading`, `windows` crate, `eframe` / `egui`, `serde` / `serde_json`, `thiserror`.

**Spec:** [2026-11-21-puwvga-design.md](file:///e:/workspace/puwvga/docs/superpowers/specs/2026-11-21-puwvga-design.md)  
**Interactive UI Mockup:** [puwvga-ui-mockup.html](file:///e:/workspace/puwvga/docs/mockups/puwvga-ui-mockup.html)  
**Design System Master:** [MASTER.md](file:///e:/workspace/puwvga/design-system/puwvga/MASTER.md)

---

## UI/UX Design Contract (from `/ui-ux-pro-max`, `/apple-design`, `/design-taste-frontend`)

**Design Read:** Native Windows 11 desktop GPU utility for power users, with a calm, utilitarian Lenovo Legion Toolkit language, leaning toward Fluent Dark/Light surfaces + Segoe UI Variable + tabular numeric telemetry + restrained spring motion.

1. **Adaptive GPU Visibility & Vendor-Native 3D/Video:**
   - Sidebar top card (`VGA phát hiện` / `Detected GPUs`) lists **only** GPUs physically detected on the machine.
   - 1 GPU (Desktop PC): shows that 1 GPU cleanly; hides MUX/Optimus cards automatically.
   - 2+ GPUs (Laptop iGPU + dGPU): shows all GPUs side-by-side on Dashboard and lets the user switch active GPU via sidebar or inline GPU strip (on Color / 3D & Video pages) to inspect and adjust its vendor-native settings.
   - **Vendor-Native 3D & Video:** Never force Intel, NVIDIA, and AMD into a single shared 3D/Video form; each backend yields its own `SettingDescriptor` list and scope support (`Global` / `Per-App` on NVIDIA DRS & AMD ADLX; dynamic capability flags via `ctlGetSupported3DCapabilities` on Intel IGCL).
2. **Surface & Color Tokens (WCAG ≥ 4.5:1):**
   - Dark: Window `#1C1C1E`, Sidebar `#18181A`, Card `#252528` (hover `#2C2C30`), Control `#323236`, Border `rgba(255,255,255,0.075)`, Text `#F3F3F5` / `#A1A1AA`, Accent `#4CC2FF`.
   - Light: Window `#F3F3F3`, Sidebar `#ECECEF`, Card `#FFFFFF` (hover `#FAFAFA`), Control `#F0F0F3`, Border `rgba(0,0,0,0.075)`, Text `#18181B` / `#52525B`, Accent `#0067C0`.
   - Vendor dots: NVIDIA `#76B900`, Intel `#0071C5`, AMD `#ED1C24`. No AI purple gradients, no gaming RGB, no emoji icons.
3. **Typography & Layout (Lenovo Legion Toolkit):**
   - Load `Segoe UI` (`C:\Windows\Fonts\segoeui.ttf`) into `egui` FontDefinitions (supports Vietnamese diacritics cleanly).
   - Hierarchy: Page Title `22px SemiBold`, Section Header `12px Uppercase SemiBold`, Card Label `13.5px Medium`, Card Description `12px Muted`, Telemetry values monospace/tabular-aligned so numbers never jitter at 1 Hz refresh.
   - Setting Card anatomy: `8px` radius, `1px` subtle border, `13px 16px` padding, `[32×32 Icon Box] — [Title + Subtitle] — [Right-aligned Control]`.
4. **Fluid Interaction (`/apple-design`):**
   - Zero-latency pointer-down feedback on buttons/cards; 1:1 live numeric readout while dragging sliders (apply to driver on drag-release or throttled 60ms so driver never stalls).
   - Interruptible critically damped spring interpolation (`egui::Context::animate_value_with_time` / custom spring helper, `damping = 1.0`, `response ≈ 0.22s`) on toggle switches and sidebar selection indicator.
   - Stop background telemetry polling immediately when window is minimized/unfocused or off Dashboard.

---

## Global Constraints

- **No WebView / Electron / Tauri:** 100% Pure Rust GUI (`eframe`/`egui`).
- **Static VC++ CRT:** `.cargo/config.toml` MUST set `rustflags = ["-C", "target-feature=+crt-static"]` for `x86_64-pc-windows-msvc`; release binary MUST NOT import `VCRUNTIME140.dll` or `MSVCP140.dll`.
- **Zero Shell Spawning:** No `std::process::Command` calling `cmd`, `powershell`, or external CLI tools.
- **Anti-Goals:** No Telemetry / real-time sensor polling (do NOT load `nvml.dll` or poll clocks/temps/power/fans), no Overlay, no screen recording, no login/network telemetry, no Overclocking/voltage/fan-curve editing, no game auto-optimize.
- **Bilingual i18n:** Full Vietnamese (`vi`) and English (`en`) switchable live in Settings; technical terms (*Digital Vibrance, Low Latency Mode, G-SYNC, YCbCr444, Gamma, LACE*) preserved in English in Vietnamese UI.
- **Fault Isolation:** Missing vendor DLL or unsupported driver feature returns `BackendError::NotAvailable` / `Unsupported` — never panics.

## Review Focus

1. **Missing Vendor DLL on Single-GPU PC:** Machine with only NVIDIA (no `ControlLib.dll`, no `atiadlxx.dll`) or only Intel/AMD must boot cleanly with zero errors and only show the present GPU.
2. **Display-to-GPU Ownership on Optimus Laptops:** Adjusting Color/Brightness on internal laptop panel must route to Intel IGCL when in Optimus mode and to NVIDIA NVAPI when in dGPU-only MUX mode.
3. **Slider Drag Flooding Driver API:** Rapid slider dragging must update UI at 60+ FPS without freezing the UI thread or spamming synchronous driver calls every pixel.
4. **Read-Only Directory for Portable Presets:** Running `puwvga.exe` from `C:\Program Files\...` without write permission must fall back cleanly to `%APPDATA%\puwvga\puwvga.presets.json`.
5. **UAC / Access Denied on MUX or Deep DRS:** Calling an elevated driver feature as standard user must surface a clean non-blocking prompt with "Relaunch as Administrator" instead of failing silently.
6. **Black Screen / Invalid Spanning Topology Recovery:** Applying display mode or multi-display spanning (Mosaic/Surround, Combined Display, Eyefinity) MUST snapshot the previous topology first and automatically revert after 15 seconds unless explicitly confirmed by the user.

---

## Tasks

### Task 1: Workspace Scaffold, Static VC++ Config & `puwvga-core`

**Files:**
- Create: `.cargo/config.toml`
- Create: `Cargo.toml`
- Create: `crates/puwvga-core/Cargo.toml`
- Create: `crates/puwvga-core/src/lib.rs`
- Create: `crates/puwvga-core/src/model.rs`
- Create: `crates/puwvga-core/src/backend.rs`
- Create: `crates/puwvga-core/src/i18n.rs`
- Create: `crates/puwvga-core/src/preset.rs`

**Interfaces:**
- Produces:
  - `Vendor` (`Intel`, `Nvidia`, `Amd`), `GpuId`, `GpuAdapterInfo`, `DisplayOutputInfo`
  - `ColorSettings`, `WireFormat`, `DisplayModeSettings`, `MuxMode`, `AppProfile`
  - Multi-display spanning schema:
    - `SpanningLayout` (`Horizontal1x2`, `Vertical2x1`, `Horizontal1x3`, `Grid2x2`, `Custom(u32, u32)`)
    - `BezelOffset { horizontal_px: u32, vertical_px: u32 }`
    - `SpanningConfig { enabled: bool, layout: SpanningLayout, display_ids: Vec<String>, bezel: BezelOffset }`
  - Vendor-native 3D & Video capability schema:
    - `SettingScope` (`Global`, `PerApp(String)`)
    - `SettingOption` (`id: u32`, `label_vi: &'static str`, `label_en: &'static str`)
    - `SettingControl` (`Toggle`, `Enum(Vec<SettingOption>)`, `Slider { min: f32, max: f32, step: f32, unit: &'static str }`)
    - `SettingValue` (`Bool(bool)`, `Int(i32)`, `Float(f32)`, `EnumId(u32)`)
    - `SettingDescriptor` (`id`, `section`, `label_vi`, `label_en`, `desc_vi`, `desc_en`, `control`, `value`, `default_value`, `supported`)
  - `GpuCapabilities` bitflags/struct indicating exact features supported per adapter (including `supports_per_app_3d`, `supports_spanning`)
  - `trait GpuBackend: Send + Sync` (`vendor()`, `enumerate_adapters()`, `get_color()`, `set_color()`, `get_display()`, `set_display()`, `get_spanning(adapter)`, `apply_spanning(adapter, config)`, `disable_spanning(adapter)`, `list_3d_settings(adapter, scope)`, `apply_3d_setting(adapter, scope, id, value)`, `list_video_settings(adapter)`, `apply_video_setting(adapter, id, value)`, `get_mux()`, `set_mux()`)
  - `enum BackendError` (`DllNotFound`, `Unsupported`, `AccessDenied`, `DriverError(i32, String)`)
  - `Language` (`Vi`, `En`) + `tr(lang, key) -> &'static str`
  - `PresetStore::resolve_path(exe_dir, appdata_dir)` + `save()` / `load()`

- [ ] **Step 1:** Create `.cargo/config.toml` with `+crt-static` and workspace `Cargo.toml` with release LTO/strip profile.
- [ ] **Step 2:** Write unit tests in `crates/puwvga-core/src/preset.rs` and `i18n.rs` testing portable-vs-appdata fallback, JSON round-trip, and VI/EN key parity.
- [ ] **Step 3:** Run `rtk cargo test -p puwvga-core` and verify tests fail before implementation.
- [ ] **Step 4:** Implement `model.rs`, `backend.rs`, `i18n.rs`, `preset.rs`, and `lib.rs`.
- [ ] **Step 5:** Run `rtk cargo test -p puwvga-core` — expected PASS. Commit.

---

### Task 2: `puwvga-win32` (DXGI Enumeration, Win32 CCD Display Routing & Windows Theme)

**Files:**
- Create: `crates/puwvga-win32/Cargo.toml`
- Create: `crates/puwvga-win32/src/lib.rs`
- Create: `crates/puwvga-win32/src/dxgi.rs`
- Create: `crates/puwvga-win32/src/display_ccd.rs`
- Create: `crates/puwvga-win32/src/theme.rs`

**Interfaces:**
- Consumes: `puwvga_core::model::{Vendor, GpuAdapterInfo, DisplayOutputInfo, DisplayModeSettings}`
- Produces:
  - `enumerate_dxgi_adapters() -> Result<Vec<DxgiAdapter>, BackendError>` (maps PCI Vendor ID `0x8086`=Intel, `0x10DE`=NVIDIA, `0x1002`=AMD + LUID)
  - `enumerate_active_displays() -> Result<Vec<DisplayOutputInfo>, BackendError>` (uses `QueryDisplayConfig` to map each active monitor to its owning adapter LUID, current resolution, Hz, color depth)
  - `list_supported_modes(device_name: &str) -> Vec<(u32, u32, u32)>` & `apply_display_mode(device_name: &str, width: u32, height: u32, hz: u32) -> Result<(), BackendError>`
  - `is_windows_dark_mode() -> bool` & `apply_dwm_dark_titlebar(hwnd: isize, dark: bool)`

- [ ] **Step 1:** Write unit tests for PCI vendor ID mapping (`0x8086`/`0x10DE`/`0x1002`) and mode deduplication/sorting, plus a read-only hardware smoke test (`#[test] fn smoke_enumerate_dxgi_and_displays`) verifying real adapters & displays on this machine.
- [ ] **Step 2:** Run `rtk cargo test -p puwvga-win32` and verify initial failure.
- [ ] **Step 3:** Implement `dxgi.rs`, `display_ccd.rs`, and `theme.rs` using `windows` crate (zero shell calls).
- [ ] **Step 4:** Run `rtk cargo test -p puwvga-win32` — expected PASS, finding Intel UHD + NVIDIA RTX 4060 Laptop on dev machine. Commit.

---

### Task 3: `puwvga-intel` (Dynamic FFI `ControlLib.dll` / IGCL)

**Files:**
- Create: `crates/puwvga-intel/Cargo.toml`
- Create: `crates/puwvga-intel/src/lib.rs`
- Create: `crates/puwvga-intel/src/igcl_abi.rs`
- Create: `crates/puwvga-intel/src/backend.rs`

**Interfaces:**
- Consumes: `puwvga_core::backend::{GpuBackend, BackendError}`, `puwvga_core::model::*`
- Produces: `IntelBackend::try_load() -> Result<IntelBackend, BackendError>` implementing `GpuBackend` via dynamic `libloading::Library::new("ControlLib.dll")`.
- Covers IGCL APIs verified on dev machine:
  - Init/Enum: `ctlInit`, `ctlClose`, `ctlEnumerateDevices`, `ctlGetDeviceProperties`, `ctlEnumerateDisplayOutputs`, `ctlGetDisplayProperties`
  - Color & Brightness: `ctlGet/SetBrightnessSetting`, `ctlPixelTransformationGet/SetConfig` (Brightness/Contrast/Gamma/Hue/Saturation/3D LUT/CSC), `ctlGet/SetLACEConfig`, `ctlGetSetDynamicContrastEnhancement`
  - Display & Combined Display (Collage): `ctlGetCurrentScaling`, `ctlGetSetRetroScaling`, `ctlGetCurrentSharpness`, `ctlGetSetWireFormat`, `ctlGetSetCustomMode`, `ctlGetSetDisplaySettings`, `ctlGetSetCombinedDisplay` (mapping `ctl_combined_display_args_t`, child display rects, and bezel overlap)
  - 3D & Video (`SettingDescriptor` mapped dynamically from `ctlGetSupported3DCapabilities` / `ctlGetSet3DFeature` and `ctlGetSupportedVideoProcessingCapabilities` / `ctlGetSetVideoProcessingFeature`):
    - 3D: Flip Mode / VSync (`Application Default`, `Always Off`, `Always On`, `Speed Sync`, `Smooth Sync`), Low Latency Mode (`Off`, `On`, `On + Boost`), Frame Limit, Anisotropic Filtering, CMAA, Adaptive Tessellation, Sharpening Filter (Slider %), Endurance Gaming.
    - Video: Intel Video Super Resolution, Film Mode Detection, Video Denoise (Slider %), Adaptive Video Sharpening (Slider %).

- [ ] **Step 1:** Write unit tests for ABI struct sizes/alignment and `BackendError::DllNotFound` handling, plus read-only integration smoke test `smoke_intel_igcl_read` that initializes IGCL on dev machine and reads adapter/display properties and supported 3D capabilities without mutating user settings.
- [ ] **Step 2:** Run `rtk cargo test -p puwvga-intel` (expect fail).
- [ ] **Step 3:** Implement `igcl_abi.rs` and `backend.rs` with strict `Drop` cleanup (`ctlClose`).
- [ ] **Step 4:** Run `rtk cargo test -p puwvga-intel` — expected PASS on real Intel UHD Graphics. Commit.

---

### Task 4: `puwvga-nvidia` (Dynamic FFI `nvapi64.dll` + DRS)

**Files:**
- Create: `crates/puwvga-nvidia/Cargo.toml`
- Create: `crates/puwvga-nvidia/src/lib.rs`
- Create: `crates/puwvga-nvidia/src/nvapi_abi.rs`
- Create: `crates/puwvga-nvidia/src/drs.rs`
- Create: `crates/puwvga-nvidia/src/backend.rs`

**Interfaces:**
- Consumes: `puwvga_core::backend::{GpuBackend, BackendError}`, `puwvga_core::model::*`
- Produces: `NvidiaBackend::try_load() -> Result<NvidiaBackend, BackendError>` implementing `GpuBackend`.
- Covers APIs verified on dev machine (no `nvml.dll`, no telemetry):
  - `nvapi64.dll` (`nvapi_QueryInterface`): `NvAPI_Initialize`, `NvAPI_Unload`, `NvAPI_EnumPhysicalGPUs`, `NvAPI_EnumNvidiaDisplayHandle`, `NvAPI_Disp_ColorControl`, `NvAPI_GetDVCInfoEx` / `NvAPI_SetDVCLevelEx` (Digital Vibrance), `NvAPI_GetHUEInfo` / `NvAPI_SetHUEAngle`, `NvAPI_Disp_GetOutputIdFromDisplayId`, scaling/G-SYNC/MUX queries, and Mosaic/Surround (`NvAPI_Mosaic_GetSupportedTopoInfo`, `NvAPI_Mosaic_EnumDisplayGrids`, `NvAPI_Mosaic_SetDisplayGrids`).
  - DRS & Video (`drs.rs` + `SettingDescriptor` for Global & Per-App, with Restore Defaults & collapsible Legacy Settings): `NvAPI_DRS_CreateSession`, `LoadSettings`, `GetBaseProfile`, `FindProfileByName`, `GetSetting` / `SetSetting`, `SaveSettings`.
    - Driver Settings (20 items): DLSS Override (Model Presets, Frame Generation Mode, Super Resolution Mode), Smooth Motion, Low Latency Mode, Automatic Display Switching, BatteryBoost, CUDA Memory Fallback, DSR - Factors, GPU App Assignment, Image Scaling, Max Frame Rate, OpenGL GDI Compatibility, Power Management Mode, RTX Dynamic Vibrance, RTX HDR, Shader Cache, Vertical Sync, Virtual Reality - Variable Rate Super Sampling, Vulkan/OpenGL Present Method.
    - Legacy Settings (11 items, collapsible): Anisotropic Filtering, Antialiasing - FXAA, Antialiasing - Transparency, Background Application Max Frame Rate, MFAA, PhysX, Texture Filtering (Anisotropic Sample Optimization, Negative LOD Bias, Quality, Trilinear Optimization), Threaded Optimization.
    - Video: RTX Video Super Resolution (`Off` / `Auto` / `Quality 1–4`), RTX Video HDR, Video Dynamic Range (`Limited` / `Full`).

- [ ] **Step 1:** Write unit tests for NVAPI interface ID table, DRS setting enum mapping, and read-only smoke test `smoke_nvidia_nvapi_read` reading RTX 4060 Laptop physical GPU info & global DRS profile without mutating settings.
- [ ] **Step 2:** Run `rtk cargo test -p puwvga-nvidia` (expect fail).
- [ ] **Step 3:** Implement `nvapi_abi.rs`, `drs.rs`, and `backend.rs` with clean `Drop` (`NvAPI_Unload`).
- [ ] **Step 4:** Run `rtk cargo test -p puwvga-nvidia` — expected PASS on real RTX 4060 Laptop. Commit.

---

### Task 5: `puwvga-amd` (Dynamic FFI `atiadlxx.dll` / ADLX)

**Files:**
- Create: `crates/puwvga-amd/Cargo.toml`
- Create: `crates/puwvga-amd/src/lib.rs`
- Create: `crates/puwvga-amd/src/adl_abi.rs`
- Create: `crates/puwvga-amd/src/backend.rs`

**Interfaces:**
- Consumes: `puwvga_core::backend::{GpuBackend, BackendError}`, `puwvga_core::model::*`
- Produces: `AmdBackend::try_load() -> Result<AmdBackend, BackendError>` implementing `GpuBackend` via dynamic `libloading::Library::new("atiadlxx.dll")`.
- Covers: Adapter enum, Display Color (Brightness, Contrast, Hue, Saturation, Color Temp), Pixel Format & Color Depth, Scaling/FreeSync, Eyefinity SLS (`ADL2_Display_SLSMapConfig_Get/Create/Set/Delete`, `ADL2_Display_BezelOffset_Set`), and AMD-native 3D & Video (`SettingDescriptor` supporting Global & Per-App):
  - 3D: Radeon Anti-Lag, Radeon Chill, Radeon Boost, Radeon Image Sharpening (RIS), Radeon Enhanced Sync, Wait for Vertical Refresh, Frame Rate Target Control (FRTC), Texture Filtering Quality, Surface Format Optimization, Tessellation Mode.
  - Video: Video Profile (`Default`, `Cinema Classic`, `Enhanced`, `Vivid`, `Custom`), AMD Video Upscale, Video Sharpening.

- [ ] **Step 1:** Write unit tests verifying `AmdBackend::try_load()` cleanly returns `Err(BackendError::DllNotFound)` on non-AMD machines and mock-adapter tests for Color/Display/Spanning conversion.
- [ ] **Step 2:** Run `rtk cargo test -p puwvga-amd` (expect fail).
- [ ] **Step 3:** Implement `adl_abi.rs` and `backend.rs`.
- [ ] **Step 4:** Run `rtk cargo test -p puwvga-amd` — expected PASS. Commit.

---

### Task 6: `puwvga-ui` (Pure Rust `eframe`/`egui` — Lenovo Legion Toolkit Design)

**Files:**
- Create: `crates/puwvga-ui/Cargo.toml`
- Create: `crates/puwvga-ui/src/main.rs` (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`)
- Create: `crates/puwvga-ui/src/theme.rs` (Fluent Dark/Light tokens + Segoe UI loader)
- Create: `crates/puwvga-ui/src/widgets.rs` (LLT `setting_card`, `spring_toggle`, `fluent_slider` with reset & throttle, `fluent_combo`, `metric_box`, vector icons)
- Create: `crates/puwvga-ui/src/state.rs` (Multi-GPU manager, static hardware info refresh on demand / hot-plug, zero background polling, UAC elevation prompt helper)
- Create: `crates/puwvga-ui/src/pages/dashboard.rs`
- Create: `crates/puwvga-ui/src/pages/color.rs`
- Create: `crates/puwvga-ui/src/pages/display.rs`
- Create: `crates/puwvga-ui/src/pages/graphics.rs`
- Create: `crates/puwvga-ui/src/pages/settings.rs`

**Interfaces:**
- Consumes: `puwvga-core`, `puwvga-win32`, `puwvga-intel`, `puwvga-nvidia`, `puwvga-amd`.
- Produces: Single Windows executable `puwvga.exe`.

- [ ] **Step 1:** Write unit tests in `state.rs` for single-GPU vs dual-GPU adapter selection, display-to-GPU ownership routing, and slider debounce/commit logic.
- [ ] **Step 2:** Implement `theme.rs` and `widgets.rs` matching the exact visual tokens and spring feel of [puwvga-ui-mockup.html](file:///e:/workspace/puwvga/docs/mockups/puwvga-ui-mockup.html).
- [ ] **Step 3:** Implement `state.rs`, all 5 pages (`dashboard`, `color`, `display` including inline GPU strip, multi-display spanning grid preview, ≥ 2 same-GPU display guard, Bezel X/Y sliders, and 15-second auto-revert confirmation countdown, `graphics` with inline GPU strip when >1 GPU, Global/Per-App switcher for NVIDIA & AMD or IGCL dynamic capability badge for Intel, rendering directly from `Vec<SettingDescriptor>`, and `settings`), and `main.rs`.
- [ ] **Step 4:** Run `rtk cargo test --workspace` and `rtk cargo build --release`. Expected: 0 warnings, 0 errors, all tests PASS. Commit.

---

### Task 7: Verification, Static VC++ Proof & Documentation

**Files:**
- Create: `README.md` (Vietnamese + English summary, written cleanly using `humanizer`)

- [ ] **Step 1:** Run full test suite: `rtk cargo test --workspace`.
- [ ] **Step 2:** Build release binary: `rtk cargo build --release`.
- [ ] **Step 3:** Inspect PE Import Table of `target/release/puwvga.exe` via script to **prove** zero dependency on `VCRUNTIME140.dll`, `MSVCP140.dll`, or WebView2, and verify binary size.
- [ ] **Step 4:** Write concise, natural `README.md` documenting features, supported GPUs, build steps, and portable usage.
- [ ] **Step 5:** Final `rtk git status` & commit.
