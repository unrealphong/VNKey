//! The shortcut that switches between Vietnamese and English.

use inputs::KeyCode;
use serde::{Deserialize, Serialize};

/// Modifiers that must be held, plus an optional key. Without a key the
/// shortcut fires when the modifiers are released.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools, reason = "one flag per modifier key")]
pub struct Hotkey {
    pub control: bool,
    pub option: bool,
    pub command: bool,
    pub shift: bool,
    /// `a`–`z`, `0`–`9`, `space` or `backquote`.
    pub key: Option<String>,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            control: true,
            shift: true,
            ..Self::none()
        }
    }
}

impl Hotkey {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            control: false,
            option: false,
            command: false,
            shift: false,
            key: None,
        }
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers {
            control: self.control,
            option: self.option,
            command: self.command,
            shift: self.shift,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "one flag per modifier key")]
struct Modifiers {
    control: bool,
    option: bool,
    command: bool,
    shift: bool,
}

impl Modifiers {
    fn is_empty(self) -> bool {
        self == Self::default()
    }

    fn union(self, other: Self) -> Self {
        Self {
            control: self.control || other.control,
            option: self.option || other.option,
            command: self.command || other.command,
            shift: self.shift || other.shift,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Pass,
    Toggle,
    /// Toggle and keep the key press from reaching the application.
    ToggleAndSuppress,
}

/// Recognises the hotkey in the stream of key presses and releases.
#[derive(Debug)]
pub struct HotkeyDetector {
    wanted: Modifiers,
    key: Option<KeyCode>,
    held: Modifiers,
    peak: Modifiers,
    interrupted: bool,
}

impl HotkeyDetector {
    #[must_use]
    pub fn new(hotkey: &Hotkey) -> Self {
        let mut detector = Self {
            wanted: Modifiers::default(),
            key: None,
            held: Modifiers::default(),
            peak: Modifiers::default(),
            interrupted: false,
        };
        detector.set_hotkey(hotkey);
        detector
    }

    pub fn set_hotkey(&mut self, hotkey: &Hotkey) {
        self.wanted = hotkey.modifiers();
        self.key = hotkey.key.as_deref().and_then(key_code);
    }

    pub fn on_key(&mut self, code: KeyCode, pressed: bool) -> HotkeyAction {
        if let Some(modifier) = modifier(code) {
            if pressed {
                self.held = self.held.union(modifier);
                self.peak = self.peak.union(self.held);
                return HotkeyAction::Pass;
            }
            self.held = without(self.held, modifier);
            if !self.held.is_empty() {
                return HotkeyAction::Pass;
            }
            let fire = self.key.is_none()
                && !self.wanted.is_empty()
                && self.peak == self.wanted
                && !self.interrupted;
            self.peak = Modifiers::default();
            self.interrupted = false;
            return if fire {
                HotkeyAction::Toggle
            } else {
                HotkeyAction::Pass
            };
        }
        if !pressed {
            return HotkeyAction::Pass;
        }
        if self.key == Some(code) && !self.wanted.is_empty() && self.held == self.wanted {
            return HotkeyAction::ToggleAndSuppress;
        }
        if !self.held.is_empty() {
            self.interrupted = true;
        }
        HotkeyAction::Pass
    }
}

fn without(held: Modifiers, released: Modifiers) -> Modifiers {
    Modifiers {
        control: held.control && !released.control,
        option: held.option && !released.option,
        command: held.command && !released.command,
        shift: held.shift && !released.shift,
    }
}

fn modifier(code: KeyCode) -> Option<Modifiers> {
    let none = Modifiers::default();
    Some(match code {
        KeyCode::ControlLeft | KeyCode::ControlRight => Modifiers {
            control: true,
            ..none
        },
        KeyCode::AltLeft | KeyCode::AltRight => Modifiers {
            option: true,
            ..none
        },
        KeyCode::MetaLeft | KeyCode::MetaRight => Modifiers {
            command: true,
            ..none
        },
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Modifiers {
            shift: true,
            ..none
        },
        _ => return None,
    })
}

const NAMED_KEYS: [(&str, KeyCode); 38] = [
    ("a", KeyCode::KeyA),
    ("b", KeyCode::KeyB),
    ("c", KeyCode::KeyC),
    ("d", KeyCode::KeyD),
    ("e", KeyCode::KeyE),
    ("f", KeyCode::KeyF),
    ("g", KeyCode::KeyG),
    ("h", KeyCode::KeyH),
    ("i", KeyCode::KeyI),
    ("j", KeyCode::KeyJ),
    ("k", KeyCode::KeyK),
    ("l", KeyCode::KeyL),
    ("m", KeyCode::KeyM),
    ("n", KeyCode::KeyN),
    ("o", KeyCode::KeyO),
    ("p", KeyCode::KeyP),
    ("q", KeyCode::KeyQ),
    ("r", KeyCode::KeyR),
    ("s", KeyCode::KeyS),
    ("t", KeyCode::KeyT),
    ("u", KeyCode::KeyU),
    ("v", KeyCode::KeyV),
    ("w", KeyCode::KeyW),
    ("x", KeyCode::KeyX),
    ("y", KeyCode::KeyY),
    ("z", KeyCode::KeyZ),
    ("0", KeyCode::Digit0),
    ("1", KeyCode::Digit1),
    ("2", KeyCode::Digit2),
    ("3", KeyCode::Digit3),
    ("4", KeyCode::Digit4),
    ("5", KeyCode::Digit5),
    ("6", KeyCode::Digit6),
    ("7", KeyCode::Digit7),
    ("8", KeyCode::Digit8),
    ("9", KeyCode::Digit9),
    ("space", KeyCode::Space),
    ("backquote", KeyCode::Backquote),
];

fn key_code(name: &str) -> Option<KeyCode> {
    NAMED_KEYS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|&(_, code)| code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press_release(detector: &mut HotkeyDetector, keys: &[(KeyCode, bool)]) -> Vec<HotkeyAction> {
        keys.iter()
            .map(|&(code, pressed)| detector.on_key(code, pressed))
            .collect()
    }

    fn toggles(actions: &[HotkeyAction]) -> usize {
        actions
            .iter()
            .filter(|action| **action != HotkeyAction::Pass)
            .count()
    }

    #[test]
    fn control_shift_toggles_on_release() {
        let mut detector = HotkeyDetector::new(&Hotkey::default());
        let actions = press_release(
            &mut detector,
            &[
                (KeyCode::ControlLeft, true),
                (KeyCode::ShiftLeft, true),
                (KeyCode::ShiftLeft, false),
                (KeyCode::ControlLeft, false),
            ],
        );
        assert_eq!(actions.last(), Some(&HotkeyAction::Toggle));
        assert_eq!(toggles(&actions), 1);
        let actions = press_release(
            &mut detector,
            &[
                (KeyCode::ShiftRight, true),
                (KeyCode::ControlLeft, true),
                (KeyCode::ControlLeft, false),
                (KeyCode::ShiftRight, false),
            ],
        );
        assert_eq!(toggles(&actions), 1);
    }

    #[test]
    fn other_keys_or_modifiers_cancel() {
        let mut detector = HotkeyDetector::new(&Hotkey::default());
        let actions = press_release(
            &mut detector,
            &[
                (KeyCode::ControlLeft, true),
                (KeyCode::ShiftLeft, true),
                (KeyCode::KeyA, true),
                (KeyCode::KeyA, false),
                (KeyCode::ShiftLeft, false),
                (KeyCode::ControlLeft, false),
            ],
        );
        assert_eq!(toggles(&actions), 0);
        let actions = press_release(
            &mut detector,
            &[
                (KeyCode::ControlLeft, true),
                (KeyCode::AltLeft, true),
                (KeyCode::ShiftLeft, true),
                (KeyCode::ShiftLeft, false),
                (KeyCode::AltLeft, false),
                (KeyCode::ControlLeft, false),
            ],
        );
        assert_eq!(toggles(&actions), 0);
        let actions = press_release(
            &mut detector,
            &[(KeyCode::ShiftLeft, true), (KeyCode::ShiftLeft, false)],
        );
        assert_eq!(toggles(&actions), 0);
    }

    #[test]
    fn hotkey_with_key_suppresses_it() {
        let hotkey = Hotkey {
            command: true,
            key: Some("space".into()),
            ..Hotkey::none()
        };
        let mut detector = HotkeyDetector::new(&hotkey);
        assert_eq!(detector.on_key(KeyCode::Space, true), HotkeyAction::Pass);
        detector.on_key(KeyCode::MetaLeft, true);
        assert_eq!(
            detector.on_key(KeyCode::Space, true),
            HotkeyAction::ToggleAndSuppress
        );
        assert_eq!(
            detector.on_key(KeyCode::MetaLeft, false),
            HotkeyAction::Pass
        );
    }

    #[test]
    fn names_keys() {
        assert_eq!(key_code("z"), Some(KeyCode::KeyZ));
        assert_eq!(key_code("7"), Some(KeyCode::Digit7));
        assert_eq!(key_code("backquote"), Some(KeyCode::Backquote));
        assert_eq!(key_code("f1"), None);
    }
}
