//! User settings shared with the settings window through `settings.json`.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use vnkey_engine::Config;

use crate::hotkey::Hotkey;

/// Files in the data directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    pub settings: PathBuf,
    pub macros: PathBuf,
    pub state: PathBuf,
}

impl Paths {
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            settings: dir.join("settings.json"),
            macros: dir.join("macros.txt"),
            state: dir.join("state.json"),
        }
    }

    /// `~/Library/Application Support/VNKey` on macOS, the XDG config
    /// directory elsewhere.
    #[must_use]
    pub fn default_location() -> Option<Self> {
        let home = env::var_os("HOME").map(PathBuf::from);
        let dir = if cfg!(target_os = "macos") {
            home?.join("Library/Application Support/VNKey")
        } else {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .or_else(|| home.map(|home| home.join(".config")))?
                .join("vnkey")
        };
        Some(Self::in_dir(&dir))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(flatten)]
    pub config: Config,
    pub switch_hotkey: Hotkey,
    /// Remember Vietnamese or English per application.
    pub smart_switch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            config: Config::default(),
            switch_hotkey: Hotkey::default(),
            smart_switch: true,
        }
    }
}

impl Settings {
    /// Reads `path`. A missing or unreadable file gives the defaults.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        read_json(path)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_json(path, self)
    }
}

pub(crate) fn read_json<T: Default + for<'de> Deserialize<'de>>(path: &Path) -> T {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
            eprintln!("Ignoring {}: {error}", path.display());
            T::default()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => T::default(),
        Err(error) => {
            eprintln!("Cannot read {}: {error}", path.display());
            T::default()
        }
    }
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

/// Replaces `path` in one step, so readers never see a partial file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    fs::write(&temp, bytes)?;
    fs::rename(&temp, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp);
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use vnkey_engine::{CodeTable, InputType};

    use super::*;

    /// A fresh, empty directory for one test.
    pub(crate) fn temp_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = env::temp_dir().join(format!(
            "vnkey-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn parse(json: &str) -> Settings {
        let path = temp_dir().join("settings.json");
        fs::write(&path, json).expect("write settings");
        Settings::load(&path)
    }

    #[test]
    fn missing_fields_take_defaults() {
        assert_eq!(parse("{}"), Settings::default());
        assert!(Settings::default().smart_switch);
        assert!(Settings::default().config.check_spelling);
    }

    #[test]
    fn reads_engine_options_and_ignores_unknown_fields() {
        let settings = parse(
            r#"{"input_type":"simple_telex2","code_table":"vni_windows","quick_telex":true,"bogus":1}"#,
        );
        assert_eq!(settings.config.input_type, InputType::SimpleTelex2);
        assert_eq!(settings.config.code_table, CodeTable::VniWindows);
        assert!(settings.config.quick_telex);
        assert!(settings.config.modern_orthography);
    }

    #[test]
    fn invalid_or_missing_files_give_defaults() {
        assert_eq!(parse("{not json"), Settings::default());
        assert_eq!(
            Settings::load(&temp_dir().join("none.json")),
            Settings::default()
        );
    }

    #[test]
    fn saves_and_loads() {
        let path = temp_dir().join("nested/settings.json");
        let mut settings = Settings::default();
        settings.config.input_type = InputType::Vni;
        settings.config.code_table = CodeTable::Cp1258;
        settings.switch_hotkey = Hotkey {
            command: true,
            key: Some("space".into()),
            ..Hotkey::none()
        };
        settings.smart_switch = false;
        settings.save(&path).expect("save");
        assert_eq!(Settings::load(&path), settings);
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains(r#""input_type": "vni""#), "{text}");
        assert!(text.contains(r#""code_table": "cp1258""#), "{text}");
    }

    #[test]
    fn data_directory_files() {
        let paths = Paths::in_dir(Path::new("/data"));
        assert_eq!(paths.settings, Path::new("/data/settings.json"));
        assert_eq!(paths.macros, Path::new("/data/macros.txt"));
        assert_eq!(paths.state, Path::new("/data/state.json"));
    }
}
