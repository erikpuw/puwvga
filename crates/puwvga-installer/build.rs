use flate2::write::GzEncoder;
use flate2::Compression;
use std::env;
use std::fs::{self, File};
use std::io::Write;
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
    if let Ok(entries) = fs::read_dir(kits_root) {
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
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=puwvga-setup.rc");
    println!("cargo:rerun-if-changed=../../assets/puwvga.ico");
    println!("cargo:rerun-if-changed=../../target/release/puwvga.exe");

    // 1. Compress target/release/puwvga.exe into OUT_DIR/puwvga_payload.gz (or empty placeholder for debug/test builds)
    let release_exe = workspace_root.join("target").join("release").join("puwvga.exe");
    let payload_path = out_dir.join("puwvga_payload.gz");
    let mut encoder = GzEncoder::new(
        File::create(&payload_path).expect("failed to create payload file"),
        Compression::best(),
    );
    if release_exe.exists() {
        let bytes = fs::read(&release_exe).expect("failed to read release puwvga.exe");
        encoder.write_all(&bytes).expect("failed to compress payload");
    }
    encoder.finish().expect("failed to finalize gzip payload");

    // 2. Compile Windows icon + version info resource
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let res_path = out_dir.join("puwvga-setup.res");
        let rc_file = manifest_dir.join("puwvga-setup.rc");
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
        assert!(status.success(), "resource compiler failed on puwvga-setup.rc");
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
    }
}
