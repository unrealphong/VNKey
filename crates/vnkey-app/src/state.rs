//! Runtime state kept across launches in `state.json`.

use std::{collections::BTreeMap, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::settings::{read_json, write_json};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub vietnamese: bool,
    /// Vietnamese (`true`) or English per application bundle id.
    pub apps: BTreeMap<String, bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vietnamese: true,
            apps: BTreeMap::new(),
        }
    }
}

impl AppState {
    #[must_use]
    pub fn load(path: &Path) -> Self {
        read_json(path)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_json(path, self)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::settings::tests::temp_dir;

    #[test]
    fn defaults_to_vietnamese() {
        let state = AppState::load(&temp_dir().join("state.json"));
        assert!(state.vietnamese);
        assert!(state.apps.is_empty());
    }

    #[test]
    fn saves_and_loads() {
        let path = temp_dir().join("state.json");
        let mut state = AppState {
            vietnamese: false,
            ..AppState::default()
        };
        state.apps.insert("com.apple.TextEdit".into(), true);
        state.apps.insert("com.apple.Terminal".into(), false);
        state.save(&path).expect("save");
        assert_eq!(AppState::load(&path), state);
    }

    #[test]
    fn invalid_file_gives_default() {
        let path = temp_dir().join("state.json");
        fs::write(&path, "[]").expect("write");
        assert_eq!(AppState::load(&path), AppState::default());
    }
}
