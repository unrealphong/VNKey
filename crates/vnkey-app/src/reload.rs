//! Applies edits made by the settings window while VNKey runs.

use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    session::Session,
    settings::{Paths, Settings},
};

const INTERVAL: Duration = Duration::from_millis(500);

type Stamp = Option<(SystemTime, u64)>;

/// Watches `settings.json` and `macros.txt` and reloads them into `session`
/// when they change. `on_change` runs after each reload.
pub fn spawn(session: Arc<Mutex<Session>>, paths: Paths, on_change: impl Fn() + Send + 'static) {
    thread::spawn(move || {
        let mut settings_stamp = stamp(&paths.settings);
        let mut macros_stamp = stamp(&paths.macros);
        loop {
            thread::sleep(INTERVAL);
            let mut changed = false;
            if update(&mut settings_stamp, &paths.settings) {
                let settings = Settings::load(&paths.settings);
                if let Ok(mut session) = session.lock() {
                    session.apply_settings(settings);
                }
                changed = true;
            }
            if update(&mut macros_stamp, &paths.macros) {
                if let Ok(mut session) = session.lock() {
                    let text = session.read_macros().unwrap_or_default();
                    session.load_macros(&text);
                }
                changed = true;
            }
            if changed {
                on_change();
            }
        }
    });
}

fn update(previous: &mut Stamp, path: &Path) -> bool {
    let current = stamp(path);
    let changed = current != *previous;
    *previous = current;
    changed
}

fn stamp(path: &Path) -> Stamp {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}
