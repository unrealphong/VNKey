//! Vietnamese input engine.
//!
//! Front-ends feed every key press and mouse click to [`Engine`] and apply
//! the returned [`Output`] to the focused application.

mod charset;
mod config;
mod core;
mod keys;
pub mod legacy;
mod macros;
mod tables;
mod word;

pub use config::{CodeTable, Config, InputType};
pub use keys::{Key, KeyInput};
pub use macros::MacroTable;

use crate::{
    core::{Core, Event, REPLACE_MACRO, RESTORE, RESTORE_AND_START_NEW_SESSION, WILL_PROCESS},
    keys::key_code_to_character,
    word::{CAPS_MASK, MAX_BUFF},
};

/// What the front-end must do with the key press it just reported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Output {
    /// Let the key reach the application unchanged.
    Pass,
    /// Swallow the key, delete `backspaces` characters before the cursor,
    /// type `text`, then send `then` if present.
    Replace {
        backspaces: usize,
        text: String,
        then: Option<Key>,
    },
}

/// A stateful Vietnamese input session.
#[derive(Clone, Debug)]
pub struct Engine {
    core: Core,
    vietnamese: bool,
}

impl Engine {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let mut core = Core::new(Config::default());
        core.set_config(config);
        Self {
            core,
            vietnamese: true,
        }
    }

    #[must_use]
    pub fn config(&self) -> &Config {
        &self.core.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.core.set_config(config);
        self.core.start_new_session();
    }

    #[must_use]
    pub fn is_vietnamese(&self) -> bool {
        self.vietnamese
    }

    pub fn set_vietnamese(&mut self, vietnamese: bool) {
        self.vietnamese = vietnamese;
        self.core.start_new_session();
    }

    #[must_use]
    pub fn macros(&self) -> &MacroTable {
        &self.core.macros
    }

    pub fn macros_mut(&mut self) -> &mut MacroTable {
        &mut self.core.macros
    }

    /// Ends the current word, as when the user clicks elsewhere.
    pub fn handle_mouse_down(&mut self) {
        if self.vietnamese {
            self.core.handle_event(Event::Mouse, 0, 0, false);
        }
    }

    pub fn handle_key(&mut self, input: KeyInput) -> Output {
        let code = input.key.code();
        let other_control_key = input.other_modifier || input.key == Key::Other;

        if !self.vietnamese {
            let config = &self.core.config;
            if !(config.use_macro && config.use_macro_in_english_mode) {
                return Output::Pass;
            }
            self.core
                .english_mode(code, input.shift || input.caps_lock, other_control_key);
            if self.core.hook.code == REPLACE_MACRO {
                return self.macro_output(input);
            }
            return Output::Pass;
        }

        let caps_status = if input.shift {
            1
        } else if input.caps_lock {
            2
        } else {
            0
        };
        self.core
            .handle_event(Event::Keyboard, code, caps_status, other_control_key);

        match self.core.hook.code {
            WILL_PROCESS | RESTORE | RESTORE_AND_START_NEW_SESSION => self.process_output(input),
            REPLACE_MACRO => self.macro_output(input),
            _ => Output::Pass,
        }
    }

    fn process_output(&mut self, input: KeyInput) -> Output {
        let hook = &self.core.hook;
        let table = self.core.config.code_table;
        let backspaces = usize::try_from(hook.backspace_count)
            .ok()
            .filter(|&count| count > 0 && count < MAX_BUFF)
            .unwrap_or(0);
        let count = usize::try_from(hook.new_char_count)
            .unwrap_or(0)
            .min(MAX_BUFF);
        let mut units = Vec::new();
        for &data in hook.char_data[..count].iter().rev() {
            charset::encode(data, table, &mut units);
        }
        let mut text = String::from_utf16_lossy(&units);
        let mut then = None;
        let code = hook.code;
        if code == RESTORE || code == RESTORE_AND_START_NEW_SESSION {
            let key = input.key.code();
            if key_code_to_character(key).is_some() {
                let caps = if input.shift || input.caps_lock {
                    CAPS_MASK
                } else {
                    0
                };
                text.extend(key_code_to_character(key | caps));
            } else {
                then = Some(input.key);
            }
        }
        if code == RESTORE_AND_START_NEW_SESSION {
            self.core.start_new_session();
        }
        Output::Replace {
            backspaces,
            text,
            then,
        }
    }

    fn macro_output(&self, input: KeyInput) -> Output {
        let hook = &self.core.hook;
        let mut units = Vec::new();
        for &data in &hook.macro_data {
            charset::encode(data, self.core.config.code_table, &mut units);
        }
        let mut text = String::from_utf16_lossy(&units);
        let then = match input.key.to_char(input.shift) {
            Some(value) => {
                text.push(value);
                None
            }
            None => Some(input.key),
        };
        Output::Replace {
            backspaces: usize::try_from(hook.backspace_count).unwrap_or(0),
            text,
            then,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(engine: &mut Engine, keys: &str) -> String {
        let mut text = String::new();
        for value in keys.chars() {
            let input = KeyInput::from_char(value).expect("typeable");
            match engine.handle_key(input) {
                Output::Pass => text.push(value),
                Output::Replace {
                    backspaces,
                    text: insert,
                    then,
                } => {
                    for _ in 0..backspaces {
                        text.pop();
                    }
                    text.push_str(&insert);
                    assert!(then.is_none());
                }
            }
        }
        text
    }

    #[test]
    fn types_telex() {
        let mut engine = Engine::new(Config::default());
        assert_eq!(type_text(&mut engine, "tieng "), "tieng ");
        assert_eq!(type_text(&mut engine, "tienge"), "tiêng");
        assert_eq!(type_text(&mut engine, " Vieejt Nam"), " Việt Nam");
        assert_eq!(type_text(&mut engine, " dduwowngf"), " đường");
    }

    #[test]
    fn types_vni() {
        let mut engine = Engine::new(Config {
            input_type: InputType::Vni,
            ..Config::default()
        });
        assert_eq!(type_text(&mut engine, "tie6ng1 "), "tiếng ");
    }

    #[test]
    fn passes_everything_in_english_mode() {
        let mut engine = Engine::new(Config::default());
        engine.set_vietnamese(false);
        assert_eq!(type_text(&mut engine, "tieengs"), "tieengs");
    }
}
