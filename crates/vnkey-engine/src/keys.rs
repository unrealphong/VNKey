//! Keys as the engine sees them.
//!
//! Internally every key is a macOS virtual key code, because the engine's
//! tables and rules are keyed by those codes.

pub(crate) const KEY_ESC: u16 = 53;
pub(crate) const KEY_DELETE: u16 = 51;
pub(crate) const KEY_TAB: u16 = 48;
pub(crate) const KEY_ENTER: u16 = 76;
pub(crate) const KEY_RETURN: u16 = 36;
pub(crate) const KEY_SPACE: u16 = 49;
pub(crate) const KEY_LEFT: u16 = 123;
pub(crate) const KEY_RIGHT: u16 = 124;
pub(crate) const KEY_DOWN: u16 = 125;
pub(crate) const KEY_UP: u16 = 126;

pub(crate) const KEY_A: u16 = 0;
pub(crate) const KEY_B: u16 = 11;
pub(crate) const KEY_C: u16 = 8;
pub(crate) const KEY_D: u16 = 2;
pub(crate) const KEY_E: u16 = 14;
pub(crate) const KEY_F: u16 = 3;
pub(crate) const KEY_G: u16 = 5;
pub(crate) const KEY_H: u16 = 4;
pub(crate) const KEY_I: u16 = 34;
pub(crate) const KEY_J: u16 = 38;
pub(crate) const KEY_K: u16 = 40;
pub(crate) const KEY_L: u16 = 37;
pub(crate) const KEY_M: u16 = 46;
pub(crate) const KEY_N: u16 = 45;
pub(crate) const KEY_O: u16 = 31;
pub(crate) const KEY_P: u16 = 35;
pub(crate) const KEY_Q: u16 = 12;
pub(crate) const KEY_R: u16 = 15;
pub(crate) const KEY_S: u16 = 1;
pub(crate) const KEY_T: u16 = 17;
pub(crate) const KEY_U: u16 = 32;
pub(crate) const KEY_V: u16 = 9;
pub(crate) const KEY_W: u16 = 13;
pub(crate) const KEY_X: u16 = 7;
pub(crate) const KEY_Y: u16 = 16;
pub(crate) const KEY_Z: u16 = 6;

pub(crate) const KEY_1: u16 = 18;
pub(crate) const KEY_2: u16 = 19;
pub(crate) const KEY_3: u16 = 20;
pub(crate) const KEY_4: u16 = 21;
pub(crate) const KEY_5: u16 = 23;
pub(crate) const KEY_6: u16 = 22;
pub(crate) const KEY_7: u16 = 26;
pub(crate) const KEY_8: u16 = 28;
pub(crate) const KEY_9: u16 = 25;
pub(crate) const KEY_0: u16 = 29;

pub(crate) const KEY_LEFT_BRACKET: u16 = 33;
pub(crate) const KEY_RIGHT_BRACKET: u16 = 30;
pub(crate) const KEY_DOT: u16 = 47;
pub(crate) const KEY_BACKQUOTE: u16 = 50;
pub(crate) const KEY_MINUS: u16 = 27;
pub(crate) const KEY_EQUALS: u16 = 24;
pub(crate) const KEY_BACK_SLASH: u16 = 42;
pub(crate) const KEY_SEMICOLON: u16 = 41;
pub(crate) const KEY_QUOTE: u16 = 39;
pub(crate) const KEY_COMMA: u16 = 43;
pub(crate) const KEY_SLASH: u16 = 44;

/// A code no table or rule matches, used for keys the engine does not know.
pub(crate) const KEY_OTHER: u16 = 0xFF;

const LETTERS: [(char, u16); 26] = [
    ('a', KEY_A),
    ('b', KEY_B),
    ('c', KEY_C),
    ('d', KEY_D),
    ('e', KEY_E),
    ('f', KEY_F),
    ('g', KEY_G),
    ('h', KEY_H),
    ('i', KEY_I),
    ('j', KEY_J),
    ('k', KEY_K),
    ('l', KEY_L),
    ('m', KEY_M),
    ('n', KEY_N),
    ('o', KEY_O),
    ('p', KEY_P),
    ('q', KEY_Q),
    ('r', KEY_R),
    ('s', KEY_S),
    ('t', KEY_T),
    ('u', KEY_U),
    ('v', KEY_V),
    ('w', KEY_W),
    ('x', KEY_X),
    ('y', KEY_Y),
    ('z', KEY_Z),
];

/// `(code, unshifted, shifted)` for the digit row and US punctuation.
const SYMBOLS: [(u16, char, char); 21] = [
    (KEY_1, '1', '!'),
    (KEY_2, '2', '@'),
    (KEY_3, '3', '#'),
    (KEY_4, '4', '$'),
    (KEY_5, '5', '%'),
    (KEY_6, '6', '^'),
    (KEY_7, '7', '&'),
    (KEY_8, '8', '*'),
    (KEY_9, '9', '('),
    (KEY_0, '0', ')'),
    (KEY_BACKQUOTE, '`', '~'),
    (KEY_MINUS, '-', '_'),
    (KEY_EQUALS, '=', '+'),
    (KEY_LEFT_BRACKET, '[', '{'),
    (KEY_RIGHT_BRACKET, ']', '}'),
    (KEY_BACK_SLASH, '\\', '|'),
    (KEY_SEMICOLON, ';', ':'),
    (KEY_QUOTE, '\'', '"'),
    (KEY_COMMA, ',', '<'),
    (KEY_DOT, '.', '>'),
    (KEY_SLASH, '/', '?'),
];

/// A key on a US QWERTY keyboard.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Key {
    /// `a`..=`z`, always lowercase; case comes from [`KeyInput`].
    Letter(char),
    /// The digit row, `0`..=`9`.
    Digit(u8),
    /// A punctuation key, given by its unshifted character:
    /// `` ` - = [ ] \ ; ' , . / ``.
    Symbol(char),
    Space,
    Backspace,
    Enter,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
    /// Any other key. It ends the current word.
    Other,
}

impl Key {
    /// The key and Shift state that type `value` on a US layout.
    #[must_use]
    pub fn from_char(value: char) -> Option<(Self, bool)> {
        if value.is_ascii_lowercase() {
            return Some((Self::Letter(value), false));
        }
        if value.is_ascii_uppercase() {
            return Some((Self::Letter(value.to_ascii_lowercase()), true));
        }
        if value == ' ' {
            return Some((Self::Space, false));
        }
        SYMBOLS.iter().find_map(|&(code, plain, shifted)| {
            let shift = if value == plain {
                false
            } else if value == shifted {
                true
            } else {
                return None;
            };
            Some((Self::from_code(code), shift))
        })
    }

    /// The character this key types, if it types one.
    #[must_use]
    pub fn to_char(self, shift: bool) -> Option<char> {
        match self {
            Self::Letter(letter) => Some(if shift {
                letter.to_ascii_uppercase()
            } else {
                letter
            }),
            Self::Space => Some(' '),
            Self::Digit(_) | Self::Symbol(_) => {
                let code = self.code();
                SYMBOLS
                    .iter()
                    .find(|entry| entry.0 == code)
                    .map(|&(_, plain, shifted)| if shift { shifted } else { plain })
            }
            _ => None,
        }
    }

    pub(crate) fn code(self) -> u16 {
        match self {
            Self::Letter(letter) => LETTERS
                .iter()
                .find(|entry| entry.0 == letter)
                .map_or(KEY_OTHER, |entry| entry.1),
            Self::Digit(digit) => {
                let value = char::from_digit(u32::from(digit), 10).unwrap_or('x');
                SYMBOLS
                    .iter()
                    .find(|entry| entry.1 == value)
                    .map_or(KEY_OTHER, |entry| entry.0)
            }
            Self::Symbol(symbol) => SYMBOLS
                .iter()
                .skip(10)
                .find(|entry| entry.1 == symbol)
                .map_or(KEY_OTHER, |entry| entry.0),
            Self::Space => KEY_SPACE,
            Self::Backspace => KEY_DELETE,
            Self::Enter => KEY_RETURN,
            Self::Tab => KEY_TAB,
            Self::Escape => KEY_ESC,
            Self::Left => KEY_LEFT,
            Self::Right => KEY_RIGHT,
            Self::Up => KEY_UP,
            Self::Down => KEY_DOWN,
            Self::Other => KEY_OTHER,
        }
    }

    pub(crate) fn from_code(code: u16) -> Self {
        if let Some(&(letter, _)) = LETTERS.iter().find(|entry| entry.1 == code) {
            return Self::Letter(letter);
        }
        if let Some(index) = SYMBOLS.iter().position(|entry| entry.0 == code) {
            return match u8::try_from(index) {
                Ok(digit) if index < 10 => Self::Digit((digit + 1) % 10),
                _ => Self::Symbol(SYMBOLS[index].1),
            };
        }
        match code {
            KEY_SPACE => Self::Space,
            KEY_DELETE => Self::Backspace,
            KEY_RETURN | KEY_ENTER => Self::Enter,
            KEY_TAB => Self::Tab,
            KEY_ESC => Self::Escape,
            KEY_LEFT => Self::Left,
            KEY_RIGHT => Self::Right,
            KEY_UP => Self::Up,
            KEY_DOWN => Self::Down,
            _ => Self::Other,
        }
    }
}

/// One key press as delivered to [`crate::Engine::handle_key`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyInput {
    pub key: Key,
    pub shift: bool,
    pub caps_lock: bool,
    /// Control, Option or Command is held.
    pub other_modifier: bool,
}

impl KeyInput {
    #[must_use]
    pub fn new(key: Key) -> Self {
        Self {
            key,
            shift: false,
            caps_lock: false,
            other_modifier: false,
        }
    }

    /// The input that types `value` on a US layout.
    #[must_use]
    pub fn from_char(value: char) -> Option<Self> {
        let (key, shift) = Key::from_char(value)?;
        Some(Self {
            shift,
            ..Self::new(key)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_printable_ascii_character() {
        for value in (' '..='~').filter(|c| !c.is_ascii_control()) {
            let (key, shift) = Key::from_char(value).expect("printable");
            assert_eq!(key.to_char(shift), Some(value), "{value:?}");
            assert_eq!(Key::from_code(key.code()), key, "{value:?}");
        }
    }

    #[test]
    fn digits_use_macos_codes() {
        assert_eq!(Key::Digit(1).code(), KEY_1);
        assert_eq!(Key::Digit(0).code(), KEY_0);
        assert_eq!(Key::from_code(KEY_0), Key::Digit(0));
        assert_eq!(Key::from_code(KEY_9), Key::Digit(9));
    }
}
