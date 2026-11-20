# puwvga

Bảng điều khiển cấu hình VGA tập trung cho Windows 10/11, hỗ trợ cả ba hãng **Intel**, **NVIDIA** và **AMD** trong một file thực thi duy nhất.

Ứng dụng đọc và ghi trực tiếp thông qua các thư viện chính thức có sẵn trong driver (`ControlLib.dll`, `nvapi64.dll`, `atiadlxx.dll` cùng Win32 DXGI/CCD). Phần giao diện viết thuần bằng Rust (`eframe`/`egui`), không dùng WebView, không gọi terminal hay PowerShell, và link tĩnh VC++ Runtime để chạy ngay trên máy mới cài Windows.

## Tính năng chính

- **Tự nhận phần cứng thực tế:** Tự động hiển thị đúng các GPU đang có trên máy (PC chạy 1 VGA hoặc laptop có cả iGPU + dGPU) và ánh xạ từng màn hình tới đúng GPU chủ quản.
- **Màu sắc & Độ sáng:** Điều chỉnh Brightness, Contrast, Gamma, Digital Vibrance / Saturation, Hue, Intel LACE, AMD Color Temperature, cùng định dạng tín hiệu màu (Color Depth, RGB/YCbCr, Dynamic Range Full/Limited).
- **Màn hình, Ghép màn hình & MUX:** Đổi độ phân giải, tần số quét, Scaling/Retro Scaling, Sharpness (Intel), VRR/G-SYNC/FreeSync, chuyển chế độ xuất hình MUX/Optimus trên laptop, và ghép nhiều màn hình (NVIDIA Mosaic/Surround, Intel Combined Display, AMD Eyefinity SLS) có đếm ngược tự hoàn tác 15 giây nếu chưa xác nhận.
- **3D & Video tách riêng theo từng hãng:**
  - **NVIDIA (`nvapi64.dll` + DRS):** Hỗ trợ Global & Per-App, gồm 20 mục Driver Settings (DLSS Override, Smooth Motion, Low Latency Mode, DSR, Image Scaling, RTX Dynamic Vibrance, RTX HDR...), 11 mục Legacy Settings thu gọn được, và 3 mục NVIDIA Video.
  - **Intel (`ControlLib.dll` IGCL):** Đọc động cờ hỗ trợ của phần cứng cho 3D Features (Flip Mode/VSync, Low Latency, Frame Limit, CMAA, Adaptive Tessellation, Sharpening, Endurance Gaming) và Video Processing (Super Resolution, Film Mode, Denoise, Adaptive Sharpening).
  - **AMD (`atiadlxx.dll` ADLX):** Global & Per-App cho 3D Gaming (Radeon Anti-Lag, Chill, Boost, RIS, Enhanced Sync, Vertical Refresh, FRTC, Texture Quality, Surface Format Optimization, Tessellation) và AMD Video.
- **Song ngữ & Portable:** Chuyển đổi tức thì giữa Tiếng Việt và English, giao diện Tối/Sáng theo Windows, lưu Preset JSON ngay cạnh file `.exe` (tự chuyển sang `%APPDATA%\puwvga\` nếu thư mục chỉ đọc).

## Những thứ bị loại bỏ có chủ đích

`puwvga` không tích hợp telemetry hoặc đọc cảm biến thời gian thực (không nạp `nvml.dll`, không polling nhiệt độ/xung nhịp/quạt chạy nền), không có overlay, không quay màn hình, không đăng nhập tài khoản, không ép xung (OC) và không tự sửa file cấu hình game. Đóng cửa sổ là tắt hẳn tiến trình.

## Cấu trúc mã nguồn

- `crates/puwvga-core`: Mô hình dữ liệu chung, trait `GpuBackend`, schema `SettingDescriptor`, song ngữ VI/EN và lưu Preset JSON.
- `crates/puwvga-win32`: Liệt kê GPU qua DXGI, ánh xạ màn hình & đổi độ phân giải/Hz qua Win32 CCD, đọc theme Windows.
- `crates/puwvga-intel`: FFI động nạp `ControlLib.dll` (Intel IGCL).
- `crates/puwvga-nvidia`: FFI động nạp `nvapi64.dll` + DRS `nvdrsdb.bin`.
- `crates/puwvga-amd`: FFI động nạp `atiadlxx.dll` (AMD ADL/ADLX).
- `crates/puwvga-ui`: Giao diện thuần Rust `eframe`/`egui` thiết kế theo bố cục thẻ cài đặt tối giản.

## Build và chạy

Yêu cầu Rust toolchain target `x86_64-pc-windows-msvc`. File `.cargo/config.toml` đã bật sẵn `-C target-feature=+crt-static` để tích hợp tĩnh VC++ Runtime vào binary.

```bash
# Chạy toàn bộ kiểm thử
cargo test --workspace

# Build bản phát hành portable duy nhất
cargo build --release
```

File thành phẩm nằm tại `target/release/puwvga.exe`.
