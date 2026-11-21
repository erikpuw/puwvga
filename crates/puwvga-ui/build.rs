use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    if let Ok(path) = env::var("RC_EXE") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let kits_root = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if let Ok(entries) = std::fs::read_dir(kits_root) {
        let mut versions: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join("x64").join("rc.exe"))
            .filter(|p| p.exists())
            .collect();
        versions.sort();
        if let Some(latest) = versions.pop() {
            return Some(latest);
        }
    }
    for candidate in ["rc.exe", "llvm-rc.exe"] {
        if let Ok(status) = Command::new("where").arg(candidate).output() {
            if status.status.success() {
                if let Some(line) = String::from_utf8_lossy(&status.stdout).lines().next() {
                    let p = PathBuf::from(line.trim());
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-changed=puwvga.rc");
    println!("cargo:rerun-if-changed=../../assets/puwvga.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let res_path = out_dir.join("puwvga.res");
    let rc_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("puwvga.rc");

    let rc_exe = find_rc_exe().expect("Windows SDK rc.exe or llvm-rc.exe not found");
    let is_llvm = rc_exe
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_ascii_lowercase().contains("llvm-rc"))
        .unwrap_or(false);

    let mut cmd = Command::new(&rc_exe);
    if is_llvm {
        cmd.arg("/FO").arg(&res_path).arg(&rc_file);
    } else {
        cmd.arg("/nologo").arg("/fo").arg(&res_path).arg(&rc_file);
    }

    let status = cmd.status().expect("failed to execute resource compiler");
    assert!(status.success(), "resource compiler failed on puwvga.rc");
    println!("cargo:rustc-link-arg-bins={}", res_path.display());
}
