//! Vietnamese input engine.
//!
//! Front-ends feed every key press and mouse click to [`Engine`] and apply
//! the returned [`Output`] to the focused application.

mod charset;
mod config;
mod core;
mod keys;
mod macros;
mod tables;
mod word;

pub use config::{CodeTable, Config, InputType};
pub use keys::{Key, KeyInput};
pub use macros::MacroTable;

use crate::{
    core::{Core, REPLACE_MACRO, RESTORE, RESTORE_AND_START_NEW_SESSION, WILL_PROCESS},
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
    /// For double-code tables: how many characters each unit typed in the
    /// current line occupies in the application, oldest first.
    lengths: Vec<u8>,
}

impl Engine {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let mut core = Core::new(Config::default());
        core.set_config(config);
        Self {
            core,
            vietnamese: true,
            lengths: Vec::new(),
        }
    }

    #[must_use]
    pub fn config(&self) -> &Config {
        &self.core.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.core.set_config(config);
        self.reset_composition();
    }

    #[must_use]
    pub fn is_vietnamese(&self) -> bool {
        self.vietnamese
    }

    pub fn set_vietnamese(&mut self, vietnamese: bool) {
        self.vietnamese = vietnamese;
        self.reset_composition();
    }

    /// Discards the typing context when focus moves to another application.
    pub fn reset_composition(&mut self) {
        self.core.reset_composition();
        self.lengths.clear();
    }

    fn double_code(&self) -> bool {
        self.core.config.code_table.is_double_code()
    }

    fn record(&mut self, length: u8) {
        if self.double_code() {
            self.lengths.push(length);
        }
    }

    /// Characters to delete for `count` typed units.
    fn backspaces_for(&mut self, count: usize) -> usize {
        if !self.double_code() {
            return count;
        }
        (0..count)
            .map(|_| match self.lengths.pop() {
                Some(length) if length > 1 => 2,
                _ => 1,
            })
            .sum()
    }

    fn encode(&mut self, data: &[u32]) -> String {
        let table = self.core.config.code_table;
        let mut units = Vec::new();
        for &code in data {
            let length = charset::encode(code, table, &mut units);
            self.record(length);
        }
        String::from_utf16_lossy(&units)
    }

    /// Output for a key the engine leaves alone.
    fn pass(&mut self) -> Output {
        if !self.double_code() {
            return Output::Pass;
        }
        match self.core.hook.ext_code {
            1 => self.lengths.clear(),
            2 => {
                if self.lengths.pop().is_some_and(|length| length > 1) {
                    return Output::Replace {
                        backspaces: 2,
                        text: String::new(),
                        then: None,
                    };
                }
            }
            3 => self.lengths.push(1),
            _ => {}
        }
        Output::Pass
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
        self.reset_composition();
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
        self.core.handle_event(code, caps_status, other_control_key);

        match self.core.hook.code {
            WILL_PROCESS | RESTORE | RESTORE_AND_START_NEW_SESSION => self.process_output(input),
            REPLACE_MACRO => self.macro_output(input),
            _ => self.pass(),
        }
    }

    fn process_output(&mut self, input: KeyInput) -> Output {
        let hook = &self.core.hook;
        let code = hook.code;
        let units = usize::try_from(hook.backspace_count)
            .ok()
            .filter(|&count| count > 0 && count < MAX_BUFF)
            .unwrap_or(0);
        let count = usize::try_from(hook.new_char_count)
            .unwrap_or(0)
            .min(MAX_BUFF);
        let data: Vec<u32> = hook.char_data[..count].iter().rev().copied().collect();

        let backspaces = self.backspaces_for(units);
        let mut text = self.encode(&data);
        let mut then = None;
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
            self.record(1);
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

    fn macro_output(&mut self, input: KeyInput) -> Output {
        let units = usize::try_from(self.core.hook.backspace_count).unwrap_or(0);
        let data = self.core.hook.macro_data.clone();
        let backspaces = self.backspaces_for(units);
        let mut text = self.encode(&data);
        let then = match input.key.to_char(input.shift) {
            Some(value) => {
                text.push(value);
                None
            }
            None => Some(input.key),
        };
        self.record(1);
        Output::Replace {
            backspaces,
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
    fn completes_horns_in_telex() {
        for free_mark in [false, true] {
            for (keys, expected) in [
                ("w ww uw ow ", "ư w ư ơ "),
                ("uwo", "ươ"),
                ("uwo uwow uow ", "ươ ươ ươ "),
                ("uwoww ", "uow "),
                ("ruwouj rwouj ruoujw ", "rượu rượu rượu "),
                ("ngwoif dduwocj ", "người được "),
                ("thuowr THUOWR quow ", "thuở THUỞ quơ "),
            ] {
                let mut engine = Engine::new(Config {
                    free_mark,
                    ..Config::default()
                });
                assert_eq!(type_text(&mut engine, keys), expected, "{keys}");
            }
        }
    }

    #[test]
    fn completes_horns_in_simple_telex_and_vni() {
        for input_type in [InputType::SimpleTelex1, InputType::SimpleTelex2] {
            let mut engine = Engine::new(Config {
                input_type,
                ..Config::default()
            });
            assert_eq!(type_text(&mut engine, "uwo uwow "), "ươ ươ ");
        }
        let mut engine = Engine::new(Config {
            input_type: InputType::Vni,
            ..Config::default()
        });
        assert_eq!(type_text(&mut engine, "u7o u7o7 ruou57 "), "ươ ươ rượu ");
    }

    #[test]
    fn edits_inferred_horns_and_keeps_tones() {
        for (typed, expected) in [("uwo<ow", "ươ"), ("uwso", "ướ"), ("owsw", "ów")] {
            let mut engine = Engine::new(Config::default());
            assert_eq!(type_inputs(&mut engine, &keys(typed)), expected, "{typed}");
        }
    }

    #[test]
    fn mouse_click_discards_an_unfinished_word() {
        let mut engine = Engine::new(Config::default());
        assert_eq!(type_text(&mut engine, "wok"), "ươk");
        engine.handle_mouse_down();
        assert_eq!(type_text(&mut engine, "w"), "ư");
    }

    /// Applies `inputs` to a text field, as a front-end would.
    fn type_inputs(engine: &mut Engine, inputs: &[KeyInput]) -> String {
        let mut text = String::new();
        for &input in inputs {
            match engine.handle_key(input) {
                Output::Pass if input.key == Key::Backspace => {
                    text.pop();
                }
                Output::Pass => text.extend(input.key.to_char(input.shift)),
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

    fn keys(text: &str) -> Vec<KeyInput> {
        text.chars()
            .map(|value| {
                if value == '<' {
                    KeyInput::new(Key::Backspace)
                } else {
                    KeyInput::from_char(value).expect("typeable")
                }
            })
            .collect()
    }

    fn engine_with(code_table: CodeTable) -> Engine {
        Engine::new(Config {
            code_table,
            ..Config::default()
        })
    }

    #[test]
    fn counts_two_character_letters_in_vni_windows() {
        let mut engine = engine_with(CodeTable::VniWindows);
        assert_eq!(type_inputs(&mut engine, &keys("aas")), "a\u{E1}");
        assert_eq!(type_inputs(&mut engine, &keys("<")), "");

        for (typed, expected) in [
            ("vieejt", "vie\u{E4}t"),
            ("vieejt<<", "vi"),
            ("vieejt<<es", "v\u{ED}e"),
        ] {
            let mut engine = engine_with(CodeTable::VniWindows);
            assert_eq!(type_inputs(&mut engine, &keys(typed)), expected, "{typed}");
        }
    }

    #[test]
    fn counts_combining_marks_in_unicode_compound() {
        let mut engine = engine_with(CodeTable::UnicodeCompound);
        assert_eq!(type_inputs(&mut engine, &keys("aas")), "â\u{301}");
        assert_eq!(type_inputs(&mut engine, &keys("<")), "");
    }

    #[test]
    fn single_code_tables_delete_one_character() {
        let mut engine = engine_with(CodeTable::Tcvn3);
        assert_eq!(type_inputs(&mut engine, &keys("aas<")), "");
    }

    #[test]
    fn passes_everything_in_english_mode() {
        let mut engine = Engine::new(Config::default());
        engine.set_vietnamese(false);
        assert_eq!(type_text(&mut engine, "tieengs"), "tieengs");
    }
}
