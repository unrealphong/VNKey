//! Everything the listener, the menu bar and the settings reloader share.

use std::fs;

use inputs::KeyCode;
use vnkey_engine::{Engine, InputType, KeyInput, Output};

use crate::{
    hotkey::{HotkeyAction, HotkeyDetector},
    settings::{Paths, Settings},
    smart_switch::SmartSwitch,
    state::AppState,
};

#[derive(Debug, PartialEq, Eq)]
pub struct KeyOutcome {
    pub output: Output,
    /// Keep the key press from reaching the application.
    pub suppress: bool,
    /// Vietnamese/English changed, so the menu bar needs a refresh.
    pub mode_changed: bool,
}

pub struct Session {
    engine: Engine,
    settings: Settings,
    hotkey: HotkeyDetector,
    smart_switch: SmartSwitch,
    state: AppState,
    paths: Option<Paths>,
}

impl Session {
    /// Loads settings, macros and state from `paths`, or uses the defaults.
    #[must_use]
    pub fn new(paths: Option<Paths>) -> Self {
        let settings = paths
            .as_ref()
            .map_or_else(Settings::default, |paths| Settings::load(&paths.settings));
        let state = paths
            .as_ref()
            .map_or_else(AppState::default, |paths| AppState::load(&paths.state));
        let mut engine = Engine::new(settings.config.clone());
        engine.set_vietnamese(state.vietnamese);
        let mut session = Self {
            engine,
            hotkey: HotkeyDetector::new(&settings.switch_hotkey),
            settings,
            smart_switch: SmartSwitch::default(),
            state,
            paths,
        };
        if let Some(text) = session.read_macros() {
            session.load_macros(&text);
        }
        session
    }

    pub fn read_macros(&self) -> Option<String> {
        let path = &self.paths.as_ref()?.macros;
        fs::read_to_string(path)
            .inspect_err(|error| {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Cannot read {}: {error}", path.display());
                }
            })
            .ok()
    }

    /// Handles one key press or release. `code` is the physical key, `input`
    /// what the engine should see for a press, `app` the application in front.
    pub fn handle_key(
        &mut self,
        code: Option<KeyCode>,
        pressed: bool,
        input: Option<KeyInput>,
        app: Option<&str>,
    ) -> KeyOutcome {
        let mut mode_changed = false;
        if let Some(app) = app {
            mode_changed = self.app_in_front(app);
        }
        let action = code.map_or(HotkeyAction::Pass, |code| self.hotkey.on_key(code, pressed));
        if action != HotkeyAction::Pass {
            self.toggle_vietnamese();
            mode_changed = true;
        }
        let output = match input {
            Some(input) if pressed && action != HotkeyAction::ToggleAndSuppress => {
                self.engine.handle_key(input)
            }
            _ => Output::Pass,
        };
        KeyOutcome {
            output,
            suppress: action == HotkeyAction::ToggleAndSuppress,
            mode_changed,
        }
    }

    /// Records that `app` is in front and restores its remembered mode.
    /// Returns whether the mode changed.
    pub fn app_in_front(&mut self, app: &str) -> bool {
        if self.smart_switch.current() == Some(app) {
            return false;
        }
        let vietnamese = self.engine.is_vietnamese();
        let state = self.settings.smart_switch.then_some(&mut self.state);
        let remembered = self.smart_switch.app_changed(app, vietnamese, state);
        let changed = remembered.is_some_and(|mode| mode != vietnamese);
        if changed {
            self.engine.set_vietnamese(!vietnamese);
            self.state.vietnamese = !vietnamese;
        } else {
            self.engine.reset_composition();
        }
        if self.settings.smart_switch {
            self.save_state();
        }
        changed
    }

    pub fn handle_mouse_down(&mut self) {
        self.engine.handle_mouse_down();
    }

    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "used by the macOS menu bar")
    )]
    #[must_use]
    pub fn is_vietnamese(&self) -> bool {
        self.engine.is_vietnamese()
    }

    pub fn toggle_vietnamese(&mut self) {
        let vietnamese = !self.engine.is_vietnamese();
        self.engine.set_vietnamese(vietnamese);
        self.state.vietnamese = vietnamese;
        if self.settings.smart_switch {
            self.smart_switch.remember(vietnamese, &mut self.state);
        }
        self.save_state();
    }

    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "used by the macOS menu bar")
    )]
    #[must_use]
    pub fn input_type(&self) -> InputType {
        self.settings.config.input_type
    }

    /// Switches the input type for this run only.
    pub fn use_input_type(&mut self, input_type: InputType) {
        let mut settings = self.settings.clone();
        settings.config.input_type = input_type;
        self.apply_settings(settings);
    }

    #[cfg_attr(
        not(target_os = "macos"),
        allow(dead_code, reason = "used by the macOS menu bar")
    )]
    /// Switches the input type and saves it to `settings.json`.
    pub fn set_input_type(&mut self, input_type: InputType) {
        self.use_input_type(input_type);
        if let Some(paths) = &self.paths {
            if let Err(error) = self.settings.save(&paths.settings) {
                eprintln!("Cannot save {}: {error}", paths.settings.display());
            }
        }
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        if settings.config != self.settings.config {
            self.engine.set_config(settings.config.clone());
        }
        self.hotkey.set_hotkey(&settings.switch_hotkey);
        self.settings = settings;
    }

    pub fn load_macros(&mut self, text: &str) {
        self.engine.macros_mut().load_text(text, false);
    }

    fn save_state(&self) {
        if let Some(paths) = &self.paths {
            if let Err(error) = self.state.save(&paths.state) {
                eprintln!("Cannot save {}: {error}", paths.state.display());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vnkey_engine::Key;

    use super::*;
    use crate::settings::tests::temp_dir;

    fn session() -> (Session, Paths) {
        let paths = Paths::in_dir(&temp_dir());
        (Session::new(Some(paths.clone())), paths)
    }

    fn type_text(session: &mut Session, text: &str) -> String {
        let mut output = String::new();
        for value in text.chars() {
            let input = KeyInput::from_char(value).expect("ASCII");
            match session.handle_key(None, true, Some(input), None).output {
                Output::Pass => output.push(value),
                Output::Replace {
                    backspaces, text, ..
                } => {
                    for _ in 0..backspaces {
                        output.pop();
                    }
                    output.push_str(&text);
                }
            }
        }
        output
    }

    fn press_hotkey(session: &mut Session) -> bool {
        let keys = [
            (KeyCode::ControlLeft, true),
            (KeyCode::ShiftLeft, true),
            (KeyCode::ShiftLeft, false),
            (KeyCode::ControlLeft, false),
        ];
        keys.iter().fold(false, |changed, &(code, pressed)| {
            session
                .handle_key(Some(code), pressed, None, None)
                .mode_changed
                || changed
        })
    }

    fn letter(session: &mut Session, app: &str) -> KeyOutcome {
        session.handle_key(
            Some(KeyCode::KeyA),
            true,
            Some(KeyInput::new(Key::Letter('a'))),
            Some(app),
        )
    }

    #[test]
    fn hotkey_toggles_and_saves_the_mode() {
        let (mut session, paths) = session();
        assert!(session.is_vietnamese());
        assert!(press_hotkey(&mut session));
        assert!(!session.is_vietnamese());
        assert!(!AppState::load(&paths.state).vietnamese);
        assert!(!Session::new(Some(paths)).is_vietnamese());
    }

    #[test]
    fn hotkey_with_key_is_suppressed() {
        let (mut session, _) = session();
        session.apply_settings(Settings {
            switch_hotkey: crate::hotkey::Hotkey {
                command: true,
                key: Some("space".into()),
                ..crate::hotkey::Hotkey::none()
            },
            ..Settings::default()
        });
        session.handle_key(Some(KeyCode::MetaLeft), true, None, None);
        let outcome = session.handle_key(
            Some(KeyCode::Space),
            true,
            Some(KeyInput::new(Key::Space)),
            None,
        );
        assert!(outcome.suppress && outcome.mode_changed);
        assert_eq!(outcome.output, Output::Pass);
    }

    #[test]
    fn restores_the_mode_of_each_app() {
        let (mut session, paths) = session();
        assert!(!letter(&mut session, "editor").mode_changed);
        assert!(!letter(&mut session, "terminal").mode_changed);
        press_hotkey(&mut session);
        assert!(!session.is_vietnamese());
        assert!(letter(&mut session, "editor").mode_changed);
        assert!(session.is_vietnamese());
        assert!(letter(&mut session, "terminal").mode_changed);
        assert!(!session.is_vietnamese());
        assert_eq!(
            AppState::load(&paths.state).apps.get("terminal"),
            Some(&false)
        );
    }

    #[test]
    fn menu_toggle_is_remembered_for_the_app_in_front() {
        let (mut session, paths) = session();
        letter(&mut session, "editor");
        assert!(!session.app_in_front("terminal"));
        session.toggle_vietnamese();
        assert!(session.app_in_front("editor"));
        assert!(session.is_vietnamese());
        assert!(session.app_in_front("terminal"));
        assert!(!session.is_vietnamese());
        let apps = AppState::load(&paths.state).apps;
        assert_eq!(apps.get("editor"), Some(&true));
        assert_eq!(apps.get("terminal"), Some(&false));
    }

    #[test]
    fn smart_switch_can_be_turned_off() {
        let (mut session, _) = session();
        session.apply_settings(Settings {
            smart_switch: false,
            ..Settings::default()
        });
        letter(&mut session, "editor");
        press_hotkey(&mut session);
        assert!(!letter(&mut session, "terminal").mode_changed);
        assert!(!letter(&mut session, "editor").mode_changed);
        assert!(!session.is_vietnamese());
    }

    #[test]
    fn resets_the_word_on_app_change_even_when_the_mode_stays_the_same() {
        for smart_switch in [false, true] {
            let mut session = Session::new(None);
            session.apply_settings(Settings {
                smart_switch,
                ..Settings::default()
            });
            session.app_in_front("editor");
            assert_eq!(type_text(&mut session, "uo"), "uo");
            assert!(!session.app_in_front("terminal"));
            let outcome = session.handle_key(
                Some(KeyCode::KeyW),
                true,
                Some(KeyInput::new(Key::Letter('w'))),
                None,
            );
            assert_eq!(
                outcome.output,
                Output::Replace {
                    backspaces: 0,
                    text: "ư".into(),
                    then: None,
                }
            );
        }
    }

    #[test]
    fn settings_change_typing() {
        let (mut session, paths) = session();
        assert_eq!(type_text(&mut session, "aa "), "â ");
        session.set_input_type(InputType::Vni);
        assert_eq!(type_text(&mut session, "aa a6 "), "aa â ");
        assert_eq!(
            Settings::load(&paths.settings).config.input_type,
            InputType::Vni
        );
    }

    #[test]
    fn loads_macros() {
        let (mut session, paths) = session();
        let mut settings = Settings::default();
        settings.config.use_macro = true;
        session.apply_settings(settings.clone());
        session.load_macros(";header\nvn:Việt Nam\n");
        assert_eq!(type_text(&mut session, "vn "), "Việt Nam ");

        settings.save(&paths.settings).expect("save");
        std::fs::write(&paths.macros, ";header\nhn:Hà Nội\n").expect("write");
        let mut session = Session::new(Some(paths));
        assert_eq!(type_text(&mut session, "hn "), "Hà Nội ");
    }
}
