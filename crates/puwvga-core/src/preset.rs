use crate::i18n::Language;
use crate::model::ColorSettings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PRESET_FILENAME: &str = "puwvga.presets.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PresetBundle {
    pub language: Language,
    pub theme: ThemePreference,
    pub display_colors: BTreeMap<String, ColorSettings>,
}

pub struct PresetStore;

impl PresetStore {
    /// Prefer storing next to `puwvga.exe` (portable mode).
    /// Fall back to `%APPDATA%\puwvga\puwvga.presets.json` when the exe directory is not writable.
    pub fn resolve_path(exe_dir: &Path, appdata_dir: &Path) -> PathBuf {
        if is_dir_writable(exe_dir) {
            exe_dir.join(PRESET_FILENAME)
        } else {
            appdata_dir.join("puwvga").join(PRESET_FILENAME)
        }
    }

    pub fn save(path: &Path, data: &PresetBundle) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    pub fn load(path: &Path) -> io::Result<PresetBundle> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

fn is_dir_writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(".puwvga_write_probe");
    match fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("puwvga_test_{tag}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolves_portable_when_exe_dir_writable_and_falls_back_when_missing() {
        let writable = unique_temp_dir("writable");
        let appdata = unique_temp_dir("appdata");

        let p1 = PresetStore::resolve_path(&writable, &appdata);
        assert_eq!(p1, writable.join(PRESET_FILENAME));

        let missing_dir = writable.join("nonexistent_readonly_dir");
        let p2 = PresetStore::resolve_path(&missing_dir, &appdata);
        assert_eq!(p2, appdata.join("puwvga").join(PRESET_FILENAME));

        let _ = fs::remove_dir_all(&writable);
        let _ = fs::remove_dir_all(&appdata);
    }

    #[test]
    fn preset_json_round_trip() {
        let dir = unique_temp_dir("roundtrip");
        let path = dir.join(PRESET_FILENAME);

        let mut bundle = PresetBundle {
            language: Language::En,
            theme: ThemePreference::Dark,
            display_colors: BTreeMap::new(),
        };
        bundle.display_colors.insert(
            "DISPLAY1".into(),
            ColorSettings {
                brightness: 62.0,
                vibrance_or_saturation: 70.0,
                ..ColorSettings::default()
            },
        );

        PresetStore::save(&path, &bundle).unwrap();
        let loaded = PresetStore::load(&path).unwrap();
        assert_eq!(loaded, bundle);

        let _ = fs::remove_dir_all(&dir);
    }
}
