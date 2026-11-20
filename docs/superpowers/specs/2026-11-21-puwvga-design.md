# Thiết kế kiến trúc: `puwvga` — Bảng điều khiển cấu hình VGA tập trung (Intel / NVIDIA / AMD)

**Ngày:** 2026-11-21  
**Workspace:** `e:\workspace\puwvga`  
**Trạng thái:** Chờ phê duyệt trước khi lập kế hoạch triển khai

---

## 1. Mục tiêu & Lý do (Intent & Why)

Các phần mềm quản lý GPU chính hãng hiện tại (NVIDIA App, NVIDIA Control Panel, Intel Graphics Software, AMD Adrenalin) bị phân mảnh, cồng kềnh (CEF/WebView nặng RAM), cài kèm nhiều tính năng thừa (overlay, quay màn hình, đăng nhập, tự tải driver, auto-optimize game, telemetry) và hay phát sinh lỗi. Trên máy PC thường chỉ chạy 1 VGA (NVIDIA/AMD/Intel), còn laptop thường chạy song song iGPU + dGPU (ví dụ Intel UHD + NVIDIA RTX 4060 Laptop).

**`puwvga`** giải quyết vấn đề này bằng một ứng dụng Windows duy nhất:
- Nhẹ, mở tức thì, đóng gói **1 file `.exe` portable** tích hợp sẵn VC++ Runtime tĩnh (`+crt-static`), không cần cài đặt, không WebView.
- Đọc và điều chỉnh trực tiếp cấu hình driver của cả 3 hãng **Intel, NVIDIA, AMD** qua các thư viện DLL chính thức có sẵn trong driver Windows.
- Tập trung toàn bộ thao tác chỉnh màu, độ sáng, màn hình/MUX, 3D/Video và xem thông số vào một giao diện tối giản, gọn đẹp lấy cảm hứng trực tiếp từ **Lenovo Legion Toolkit** (Windows 11 Fluent / WPF-UI).

---

## 2. Phạm vi: Goals, Non-Goals & Anti-Goals

### Goals (Phải làm)
1. **Tự động phát hiện phần cứng & driver thực tế:**
   - Nhận diện tất cả GPU đang hoạt động (PC 1 VGA hoặc laptop iGPU + dGPU) qua DXGI + dynamic loading DLL của từng hãng.
   - Chỉ hiển thị các GPU và tính năng mà driver trên máy thực sự hỗ trợ; tính năng không hỗ trợ được ẩn hoặc hiển thị trạng thái không khả dụng rõ ràng (không crash, không báo lỗi giả).
2. **3 nhóm cấu hình cốt lõi + Tổng quan phần cứng tĩnh:**
   - **Màu & Độ sáng (Color & Brightness):**
     - *NVIDIA:* Brightness, Contrast, Gamma, Digital Vibrance, Hue; Color Depth (bpc), Output Color Format (RGB / YCbCr444 / YCbCr422 / YCbCr420), Output Dynamic Range (Full / Limited).
     - *Intel:* Brightness, Contrast, Gamma, Hue, Saturation, 3D LUT / CSC (Pixel Transformation), Dynamic Contrast Enhancement (LACE).
     - *AMD:* Brightness, Contrast, Hue, Saturation, Color Temperature; Pixel Format, Color Depth.
   - **Màn hình & MUX (Display & MUX):**
     - Độ phân giải, tần số quét (Hz), xoay màn hình, chế độ đa màn hình (kết hợp Win32 CCD API `SetDisplayConfig` / `EnumDisplaySettingsEx` + vendor API).
     - *NVIDIA:* Scaling (Aspect Ratio / Full-screen / No scaling, GPU vs Display), G-SYNC, chuyển chế độ xuất hình Optimus / dGPU (MUX / Advanced Optimus nếu driver hỗ trợ), ghép màn hình **Mosaic / Surround** qua `NvAPI_Mosaic_*` trong `nvapi64.dll`.
     - *Intel:* Custom Mode, Scaling & Retro Scaling, Sharpness, Wire Format (RGB/YCbCr, bpc, Full/Limited), VRR / Endurance Gaming, ghép màn hình **Combined Display (Collage)** qua `ctlGetSetCombinedDisplay` trong `ControlLib.dll`.
     - *AMD:* GPU Scaling, Virtual Super Resolution (VSR), FreeSync / Adaptive Sync, Integer Scaling, ghép màn hình **Eyefinity (SLS — Single Large Surface)** qua `ADL2_Display_SLSMapConfig_*` / ADLX Eyefinity trong `atiadlxx.dll`.
     - *Cơ chế an toàn ghép màn hình:* Chỉ cho phép kích hoạt khi có ≥ 2 màn hình nối trực tiếp vào cùng 1 GPU; lưu snapshot topology trước khi áp dụng và hiển thị hộp xác nhận **tự hoàn tác sau 15 giây (Auto-Revert)** để tránh mất hình.
   - **3D & Video Settings (Tách riêng theo từng hãng — Vendor-Native Capability Schema):**
     Không ép 3 hãng dùng chung 1 bảng cấu hình phẳng; mỗi backend tự khai báo và render đúng bộ tính năng gốc của driver đang cài:
     - *NVIDIA (`nvapi64.dll` + DRS `nvdrsdb.bin`, hỗ trợ Global & Per-App, có nút Restore Defaults & thu gọn Legacy Settings):*
       - **Driver Settings chính:** DLSS Override (Model Presets, Frame Generation Mode, Super Resolution Mode), Smooth Motion, Low Latency Mode, Automatic Display Switching (laptop), BatteryBoost (laptop), CUDA Memory Fallback, DSR - Factors, GPU App Assignment, Image Scaling (NIS), Max Frame Rate, OpenGL GDI Compatibility, Power Management Mode, RTX Dynamic Vibrance, RTX HDR, Shader Cache, Vertical Sync, VR - Variable Rate Super Sampling, Vulkan/OpenGL Present Method.
       - **Legacy Settings (thu gọn/mở rộng):** Anisotropic Filtering, Antialiasing - FXAA, Antialiasing - Transparency, Background Application Max Frame Rate, MFAA, PhysX, Texture Filtering (Anisotropic Sample Optimization, Negative LOD Bias, Quality, Trilinear Optimization), Threaded Optimization.
       - **Video:** RTX Video Super Resolution (`Off` / `Auto` / `Quality 1–4`), RTX Video HDR, Video Dynamic Range (`Limited` / `Full`).
     - *Intel (`ControlLib.dll` IGCL, đọc động từ `ctlGetSupported3DCapabilities` & `ctlGetSupportedVideoProcessingCapabilities`):* Flip Mode / VSync (`Application Default` / `Always Off` / `Always On` / `Speed Sync` / `Smooth Sync`), Low Latency Mode, Frame Limit, Anisotropic Filtering, CMAA, Adaptive Tessellation, Sharpening Filter, Endurance Gaming; Video: Intel Video Super Resolution, Film Mode Detection, Video Denoise, Adaptive Video Sharpening.
     - *AMD (`atiadlxx.dll` / ADLX, hỗ trợ Global & Per-App):* Radeon Anti-Lag, Radeon Chill, Radeon Boost, Radeon Image Sharpening (RIS), Radeon Enhanced Sync, Wait for Vertical Refresh, Frame Rate Target Control (FRTC), Texture Filtering Quality, Surface Format Optimization, Tessellation Mode; Video: Video Profile, AMD Video Upscale, Video Sharpening.
   - **Tổng quan phần cứng tĩnh (Static Hardware Info — Không Telemetry):**
     - Chỉ đọc 1 lần khi mở app / khi bấm Làm mới: Tên GPU, phiên bản Driver, API Driver đang dùng, tổng VRAM (DXGI), PCI ID và danh sách màn hình nối trực tiếp. Không đọc cảm biến thời gian thực, không polling chạy nền.
3. **Thuần Rust & Không gọi shell:**
   - 100% logic gọi trực tiếp FFI DLL / Win32 API. **Tuyệt đối không** tạo tiến trình `cmd.exe`, `powershell.exe`, hay script ngoài.
4. **Portable hoàn toàn:**
   - Link tĩnh VC++ CRT (`-C target-feature=+crt-static`), không phụ thuộc WebView2 hay VC++ Redistributable.

### Non-Goals (Không ưu tiên / Không làm ở phạm vi này)
- Hỗ trợ Linux/macOS (chỉ tập trung Windows 10/11 x64).
- Tự động tải hoặc cài đặt driver GPU.
- Lưu trữ/đồng bộ cấu hình lên đám mây.

### Anti-Goals (Cấm tuyệt đối — Làm là thất bại)
- **Không** Telemetry / giám sát cảm biến thời gian thực (không polling nhiệt độ, xung nhịp, điện năng, quạt; không nạp `nvml.dll`).
- **Không** Overlay in-game.
- **Không** quay màn hình / chụp ảnh màn hình / replay buffer.
- **Không** đăng nhập tài khoản / gửi telemetry ra mạng.
- **Không** Overclock, chỉnh điện áp, chỉnh Power Limit / ép xung quạt.
- **Không** Auto-Optimize game (tự sửa file config của game).
- **Không** dùng WebView / Electron / CEF / Tauri.
- **Không** spawn terminal hoặc PowerShell trong runtime.

---

## 3. Hiện trạng phần cứng kiểm chứng (Reference Machine)

Đã kiểm tra trực tiếp trên máy phát triển (Lenovo Laptop):
- **iGPU:** `Intel(R) UHD Graphics` — Driver `32.0.101.6881`
  - DLL sẵn có: `C:\Windows\System32\ControlLib.dll` (IGCL — 125 hàm export).
- **dGPU:** `NVIDIA GeForce RTX 4060 Laptop GPU` — Driver `32.0.16.1692`
  - DLL sẵn có: `C:\Windows\System32\nvapi64.dll` (`nvapi_QueryInterface`), `C:\Windows\System32\nvml.dll` (430 hàm export), `nvdrsdb.bin` (DRS 3D profile DB).
- **AMD:** Không có trên máy dev → backend AMD (`atiadlxx.dll` / ADLX) sẽ được thiết kế theo đúng chuẩn ABI/FFI động, có unit test mock đầy đủ và tự ẩn sạch sẽ khi không tìm thấy GPU/DLL AMD.

---

## 4. Kiến trúc kỹ thuật (Technical Architecture)

### 4.1. Cấu trúc Cargo Workspace (Đề xuất của bé)
Tách rõ lõi FFI/Domain khỏi giao diện để dễ test, không đụng chạm vòng tròn:

```text
puwvga/
├── .cargo/config.toml         # +crt-static cho x86_64-pc-windows-msvc
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── puwvga-core/           # Model chung, trait GpuBackend, cấu hình cục bộ, capability flags
│   ├── puwvga-win32/          # DXGI adapter enum, Win32 CCD Display API, Windows Theme/Accent, DWM Dark Titlebar
│   ├── puwvga-nvidia/         # Dynamic FFI: nvapi64.dll (Display, Color, Mosaic/Surround, DRS 3D, Video, MUX)
│   ├── puwvga-intel/          # Dynamic FFI: ControlLib.dll (IGCL: Color/LUT/LACE, Display/Combined Display/Scaling/Sharpness, 3D, Video)
│   ├── puwvga-amd/            # Dynamic FFI: atiadlxx.dll / ADLX (Color, Display/Eyefinity, 3D, Video)
│   └── puwvga-ui/             # Pure Rust GUI (eframe/egui) dựng lại hệ thống component & layout của Lenovo Legion Toolkit
```

### 4.2. Chiến lược FFI an toàn (Safe Dynamic Loading)
- **Dynamic Loading (`libloading`):** Không link tĩnh `.lib` của hãng để app không bao giờ lỗi thiếu DLL trên máy chỉ có 1 hãng VGA. Mỗi backend tự `LoadLibrary("ControlLib.dll")` / `LoadLibrary("nvapi64.dll")` / `LoadLibrary("atiadlxx.dll")`.
- **Không phụ thuộc header C nặng khi build:** Định nghĩa struct `#[repr(C)]` và function pointer chuẩn ABI ngay trong Rust (đối chiếu chính thức với Intel IGCL spec, NVAPI open headers / query interface IDs, AMD ADL/ADLX spec). Điều này giúp `cargo build` sạch sẽ 100% mà không cần cài SDK rời của 3 hãng.
- **Vendor-Native Capability Descriptor Schema (cho 3D & Video):** Vì 3D & Video của Intel, NVIDIA và AMD có tên gọi, giá trị và phạm vi áp dụng (Global vs Per-App) khác hẳn nhau, `puwvga-core` định nghĩa mô hình `SettingDescriptor` (`id`, `section`, `control`: Toggle/Enum/Slider, `value`, `default_value`, `supported`). Mỗi backend trả về đúng danh sách tính năng thật của hãng đó (chỉ các cờ driver báo hỗ trợ), UI tự động render giao diện tương ứng mà không ép chung cấu trúc phẳng.
- **Cô lập lỗi (Fault Isolation):** Mọi lời gọi FFI trả về `Result<T, BackendError>` kèm mã lỗi driver gốc. Một tính năng hoặc một hãng lỗi không làm crash toàn bộ app.
- **Không chạy nền / 0% CPU khi idle:** Không có vòng lặp polling telemetry. Chỉ gọi driver khi mở app, bấm Làm mới, nhận sự kiện cắm/rút màn hình, hoặc khi người dùng thay đổi cấu hình.

### 4.3. Quản lý iGPU + dGPU & Màn hình
- Dùng **DXGI (`EnumAdapters1`) + Win32 CCD (`QueryDisplayConfig`)** làm nguồn chân lý duy nhất để ánh xạ: **Màn hình nào đang nối trực tiếp vào GPU nào** (ví dụ trên laptop Optimus: màn hình tích hợp gắn vào Intel iGPU, cổng HDMI/USB-C có thể gắn vào NVIDIA dGPU; khi bật dGPU-only MUX thì màn hình tích hợp chuyển sang NVIDIA).
- Giao diện hiển thị rõ nhãn GPU chủ quản cho từng màn hình (`[Intel UHD]` / `[NVIDIA RTX 4060]` / `[AMD Radeon]`), tránh tình trạng người dùng chỉnh màu ở hãng này nhưng màn hình lại đang do hãng kia xuất hình.

### 4.4. Thiết kế Giao diện — Phong cách Lenovo Legion Toolkit (Không WebView)
- **Engine:** Pure Rust `eframe` / `egui` (chế độ `glow` OpenGL hoặc `wgpu` nhẹ + `DwmSetWindowAttribute` cho thanh tiêu đề Windows 11 Dark/Light chuẩn hệ thống, nạp font `Segoe UI` / `Segoe UI Variable` trực tiếp từ `C:\Windows\Fonts`).
- **Bố cục chuẩn Lenovo Legion Toolkit:**
  - **Sidebar trái cố định (~200px):**
    - Trên cùng: Bộ chọn nhanh GPU (hoặc chế độ "Tất cả GPU" tự lọc theo màn hình) + logo/tên gọn `puwvga`.
    - Các mục điều hướng chính (icon mảnh + chữ, có vạch accent xanh bên trái mục đang chọn):
      1. **Tổng quan (Dashboard)** — Thẻ thông tin tĩnh gọn cho từng GPU (tên, phiên bản driver, Driver API, VRAM tổng, PCI ID, số màn hình kết nối, trạng thái MUX/Hybrid — không telemetry).
      2. **Màu & Độ sáng (Color)** — Chọn màn hình → danh sách Setting Card cho Brightness, Contrast, Gamma, Vibrance/Saturation, Hue, Color Depth, Color Format, Dynamic Range, LACE/3D LUT; có nút "Khôi phục mặc định".
      3. **Màn hình & MUX (Display)** — Chọn GPU/màn hình → Độ phân giải, Hz, Scaling/Retro Scaling, Sharpness, VRR/G-SYNC/FreeSync, Custom Mode (Intel), chế độ xuất hình MUX; kèm khu vực **Ghép màn hình (NVIDIA Mosaic/Surround · Intel Combined Display/Collage · AMD Eyefinity)** có sơ đồ lưới trực quan, bù viền (Bezel Compensation X/Y) và tự hoàn tác sau 15 giây nếu chưa xác nhận.
      4. **3D & Video (Graphics)** — Khi có 2 GPU, hiển thị thanh chọn GPU ngay đầu trang để chuyển nhanh giữa các hãng; mỗi hãng render đúng bộ setting 3D & Video gốc từ `SettingDescriptor` (có chuyển Global / Per-App ở NVIDIA DRS & AMD ADLX; đọc động capability flags trên Intel IGCL).
      5. **Cài đặt app (Settings)** — Giao diện Sáng/Tối/Theo Windows, chuyển đổi ngôn ngữ **Tiếng Việt / English**, Lưu/Áp dụng Preset cục bộ (JSON), tùy chọn chạy nền/tắt hẳn.
  - **Ngôn ngữ thiết kế Card (LLT Style):**
    - Nền chính `#1C1C1C` (Dark) / `#F3F3F3` (Light); Card `#272727` (Dark) / `#FFFFFF` (Light), viền mảnh `1px`, bo góc `8px`, padding `14px–16px`.
    - Mỗi hàng setting là 1 Card ngang: `[Icon trái] — [Tiêu đề đậm + Mô tả phụ màu xám] — [Control phải: Toggle / Slider+Số / ComboBox]`.
    - Không hiệu ứng màu mè, không gradient gaming, không quảng cáo.

---

## 5. Build, Đóng gói & Kiểm thử

- **Static VC++ Runtime:** `.cargo/config.toml` bật `rustflags = ["-C", "target-feature=+crt-static"]` cho `x86_64-pc-windows-msvc`.
- **Tối ưu dung lượng release:** `lto = true`, `codegen-units = 1`, `opt-level = "s"` (hoặc `3`), `strip = true`, subsystem Windows (`#![windows_subsystem = "windows"]` trên bản release để không bao giờ hiện cửa sổ console đen).
- **Kiểm thử (Testing):**
  - Unit tests cho `puwvga-core`, serialization preset, capability matrix, và mock ABI cho 3 backend (`nvidia`, `intel`, `amd`).
  - Hardware integration smoke test (read-only) chạy trực tiếp trên máy thật của anh để xác nhận đọc được đúng: Intel IGCL (`ControlLib.dll`), NVIDIA NVAPI (`nvapi64.dll`) + NVML (`nvml.dll`), và Win32 Display config.
  - Kiểm tra `dumpbin /dependents puwvga.exe` (hoặc script đọc PE import table) để **chứng minh 100%** file `.exe` không phụ thuộc `VCRUNTIME140.dll` / `MSVCP140.dll` hay bất kỳ DLL ngoài Windows chuẩn nào.

---

## 6. Quyết định đã chốt vs Mặc định của bé

### Anh đã chốt
- Hỗ trợ 3 hãng Intel, NVIDIA, AMD; tự thích ứng PC 1 VGA và laptop iGPU + dGPU.
- 4 nhóm chức năng (Màu & Độ sáng, Màn hình & MUX, 3D & Video, Giám sát chỉ đọc); cấm Overlay, quay màn hình, đăng nhập, OC, auto-optimize.
- Viết bằng Rust, gọi trực tiếp DLL driver, không gọi terminal/PowerShell.
- Không dùng WebView; UI lấy phong cách dựng giao diện của Lenovo Legion Toolkit.
- Tích hợp tĩnh VC++ Runtime để không cần cài đặt.

### Mặc định bé đề xuất (Anh có thể đổi nếu muốn)
- **GUI crate:** `eframe` / `egui` (chế độ native Windows, custom Fluent theme theo Lenovo Legion Toolkit, nạp font `Segoe UI` hệ thống).
- **Hành vi đóng app:** Mặc định **đóng là tắt hẳn** (0% RAM/CPU nền); không chạy ngầm tự ghi đè cấu hình driver.
- **Lưu trữ Preset (JSON):** Chế độ Portable — lưu cạnh `puwvga.exe` nếu thư mục ghi được, tự fallback sang `%APPDATA%\puwvga\` nếu chạy từ thư mục chỉ đọc (như `Program Files`). Khi driver reset/cập nhật, UI luôn hiển thị trạng thái thực từ driver; anh bấm "Áp dụng Preset" để khôi phục khi cần.
- **Quyền Administrator (UAC):** Chạy quyền User thường để mở nhanh không hỏi UAC; chỉ hiện nút "Khởi chạy lại với quyền Admin" nếu một API đặc thù (như đổi MUX) trả về lỗi thiếu quyền.
- **Hot-plug màn hình / đổi MUX:** Tự động làm mới danh sách GPU & màn hình khi phát hiện thay đổi hiển thị (cắm/rút màn hình, đổi MUX) kèm nút làm mới thủ công.
- **Hỗ trợ song ngữ (i18n):** Chuyển đổi trực tiếp giữa **Tiếng Việt** và **English** trong trang Settings (giữ nguyên thuật ngữ kỹ thuật chuẩn như *Digital Vibrance, Low Latency Mode, G-SYNC, YCbCr444, Gamma, LACE* ở bản Tiếng Việt).
- **Phần để cho builder quyết định khi lập kế hoạch:** Danh sách chi tiết từng NVAPI interface ID / DRS setting ID triển khai theo từng phase (Phase 1: Core + Telemetry + Display/Color thực tế trên Intel & NVIDIA → Phase 2: 3D/Video + Per-App DRS + AMD backend → Phase 3: Polish UI LLT + Preset + Portable release check).
