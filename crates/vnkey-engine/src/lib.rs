//! Vietnamese input engine.
//!
//! Front-ends feed every key press and mouse click to [`Engine`] and apply
//! the returned [`Output`] to the focused application.

mod config;
mod keys;
pub mod legacy;

pub use config::{CodeTable, Config, InputType};
pub use keys::{Key, KeyInput};

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
    config: Config,
    vietnamese: bool,
}

impl Engine {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            vietnamese: true,
        }
    }

    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    #[must_use]
    pub fn is_vietnamese(&self) -> bool {
        self.vietnamese
    }

    pub fn set_vietnamese(&mut self, vietnamese: bool) {
        self.vietnamese = vietnamese;
    }

    pub fn handle_key(&mut self, input: KeyInput) -> Output {
        let _ = input;
        Output::Pass
    }

    pub fn handle_mouse_down(&mut self) {}
}
