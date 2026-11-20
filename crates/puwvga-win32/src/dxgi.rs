use puwvga_core::{BackendError, Vendor};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxgiAdapter {
    pub vendor: Vendor,
    pub name: String,
    pub pci_vendor_id: u32,
    pub pci_device_id: u32,
    pub dedicated_vram_mb: u64,
    pub luid_low: u32,
    pub luid_high: i32,
}

fn wide_to_string(slice: &[u16]) -> String {
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end]).trim().to_string()
}

/// Enumerates physical Intel, NVIDIA, and AMD GPUs via DXGI (excludes software adapters).
pub fn enumerate_dxgi_adapters() -> Result<Vec<DxgiAdapter>, BackendError> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.map_err(|e| {
        BackendError::DriverError {
            code: e.code().0,
            message: format!("CreateDXGIFactory1 failed: {e}"),
        }
    })?;

    let mut result = Vec::new();
    let mut index = 0u32;

    loop {
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(a) => a,
            Err(_) => break,
        };
        index += 1;

        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(d) => d,
            Err(_) => continue,
        };

        if (desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32)) != 0 {
            continue;
        }

        let Some(vendor) = Vendor::from_pci_vendor_id(desc.VendorId) else {
            continue;
        };

        let name = wide_to_string(&desc.Description);
        let dedicated_vram_mb = (desc.DedicatedVideoMemory as u64) / (1024 * 1024);

        // Deduplicate by adapter LUID
        if result
            .iter()
            .any(|a: &DxgiAdapter| a.luid_low == desc.AdapterLuid.LowPart && a.luid_high == desc.AdapterLuid.HighPart)
        {
            continue;
        }

        result.push(DxgiAdapter {
            vendor,
            name,
            pci_vendor_id: desc.VendorId,
            pci_device_id: desc.DeviceId,
            dedicated_vram_mb,
            luid_low: desc.AdapterLuid.LowPart,
            luid_high: desc.AdapterLuid.HighPart,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_to_string_trims_nulls() {
        let buf = [b'R' as u16, b'T' as u16, b'X' as u16, 0, b'X' as u16];
        assert_eq!(wide_to_string(&buf), "RTX");
    }

    #[test]
    fn smoke_enumerate_dxgi_adapters() {
        let adapters = enumerate_dxgi_adapters().expect("DXGI enumeration should succeed");
        assert!(!adapters.is_empty(), "expected at least 1 hardware GPU");
        for a in &adapters {
            assert!(!a.name.is_empty());
        }
    }
}
