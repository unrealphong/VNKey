//! Stateful Vietnamese input engine.
//!
//! The engine is platform independent. Front-ends feed it printable keys and
//! apply the returned edit to the focused application.

use std::collections::HashMap;

/// Supported Vietnamese input methods.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputMethod {
    #[default]
    Telex,
    Vni,
}

/// An edit relative to text already sent to the focused application.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Edit {
    pub delete_chars: usize,
    pub insert: String,
}

impl Edit {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.delete_chars == 0 && self.insert.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    Plain,
    Breve,
    Circumflex,
    Horn,
    Stroke,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Tone {
    #[default]
    None,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Letter {
    base: char,
    shape: Shape,
    uppercase: bool,
}

impl Letter {
    fn new(value: char) -> Self {
        Self {
            base: value.to_ascii_lowercase(),
            shape: Shape::Plain,
            uppercase: value.is_ascii_uppercase(),
        }
    }

    fn is_vowel(self) -> bool {
        matches!(self.base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    }
}

#[derive(Clone, Debug, Default)]
struct WordState {
    letters: Vec<Letter>,
    tone: Tone,
}

impl WordState {
    fn render(&self) -> String {
        let tone_at = self.tone_index();
        self.letters
            .iter()
            .enumerate()
            .map(|(index, letter)| {
                render_letter(*letter, (tone_at == Some(index)).then_some(self.tone))
            })
            .collect()
    }

    fn tone_index(&self) -> Option<usize> {
        let mut vowels: Vec<usize> = self
            .letters
            .iter()
            .enumerate()
            .filter_map(|(i, letter)| letter.is_vowel().then_some(i))
            .collect();
        if vowels.len() > 1 && self.letters.first().is_some_and(|l| l.base == 'q') && vowels[0] == 1
        {
            vowels.remove(0);
        }
        if vowels.len() > 1
            && self.letters.first().is_some_and(|l| l.base == 'g')
            && vowels[0] == 1
            && self.letters[1].base == 'i'
        {
            vowels.remove(0);
        }
        if vowels.is_empty() {
            return None;
        }
        if let Some(index) = vowels.iter().copied().rev().find(|&i| {
            matches!(
                self.letters[i].shape,
                Shape::Breve | Shape::Circumflex | Shape::Horn
            )
        }) {
            return Some(index);
        }
        match vowels.as_slice() {
            [only] => Some(*only),
            [first, second] => {
                let pair = (self.letters[*first].base, self.letters[*second].base);
                let ends_in_vowel = *second + 1 == self.letters.len();
                if matches!(pair, ('o', 'a' | 'e') | ('u', 'y')) {
                    Some(*second)
                } else if ends_in_vowel {
                    Some(*first)
                } else {
                    Some(*second)
                }
            }
            _ => Some(vowels[vowels.len() - 2]),
        }
    }
}

/// Stateful input processor with undo-aware backspace handling and macros.
#[derive(Clone, Debug)]
pub struct Engine {
    method: InputMethod,
    enabled: bool,
    state: WordState,
    history: Vec<WordState>,
    rendered: String,
    macros: HashMap<String, String>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(InputMethod::Telex)
    }
}

impl Engine {
    #[must_use]
    pub fn new(method: InputMethod) -> Self {
        Self {
            method,
            enabled: true,
            state: WordState::default(),
            history: Vec::new(),
            rendered: String::new(),
            macros: HashMap::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.reset();
    }

    pub fn set_method(&mut self, method: InputMethod) {
        self.method = method;
        self.reset();
    }

    pub fn add_macro(&mut self, trigger: impl Into<String>, replacement: impl Into<String>) {
        self.macros.insert(trigger.into(), replacement.into());
    }

    /// Process one printable character.
    pub fn push(&mut self, input: char) -> Edit {
        if !self.enabled || !input.is_ascii_alphabetic() && !input.is_ascii_digit() {
            let macro_edit = self.finish_word();
            return Edit {
                delete_chars: macro_edit.delete_chars,
                insert: format!("{}{}", macro_edit.insert, input),
            };
        }

        self.history.push(self.state.clone());
        let old = self.rendered.clone();
        let handled = match self.method {
            InputMethod::Telex => self.apply_telex(input),
            InputMethod::Vni => self.apply_vni(input),
        };
        if !handled {
            self.state.letters.push(Letter::new(input));
        }
        self.rendered = self.state.render();
        diff(&old, &self.rendered)
    }

    /// Process a physical backspace. An engine-managed word is restored to its
    /// exact previous composition state.
    pub fn backspace(&mut self) -> Edit {
        let old = self.rendered.clone();
        if let Some(previous) = self.history.pop() {
            self.state = previous;
            self.rendered = self.state.render();
            diff(&old, &self.rendered)
        } else {
            Edit {
                delete_chars: 1,
                insert: String::new(),
            }
        }
    }

    /// End the current word and expand a matching macro, if any.
    pub fn finish_word(&mut self) -> Edit {
        let old = self.rendered.clone();
        let replacement = self.macros.get(&old).cloned();
        self.reset();
        replacement.map_or_else(Edit::default, |insert| Edit {
            delete_chars: old.chars().count(),
            insert,
        })
    }

    pub fn reset(&mut self) {
        self.state = WordState::default();
        self.history.clear();
        self.rendered.clear();
    }

    fn apply_telex(&mut self, input: char) -> bool {
        let key = input.to_ascii_lowercase();
        if let Some(tone) = match key {
            's' => Some(Tone::Acute),
            'f' => Some(Tone::Grave),
            'r' => Some(Tone::Hook),
            'x' => Some(Tone::Tilde),
            'j' => Some(Tone::Dot),
            _ => None,
        } {
            if self.state.letters.iter().any(|letter| letter.is_vowel()) {
                self.state.tone = if self.state.tone == tone {
                    Tone::None
                } else {
                    tone
                };
                return true;
            }
        }
        if key == 'z'
            && (self.state.tone != Tone::None
                || self.state.letters.iter().any(|l| l.shape != Shape::Plain))
        {
            self.state.tone = Tone::None;
            for letter in &mut self.state.letters {
                letter.shape = Shape::Plain;
            }
            return true;
        }
        if key == 'd'
            && self
                .state
                .letters
                .last()
                .is_some_and(|l| l.base == 'd' && l.shape == Shape::Plain)
        {
            self.state.letters.last_mut().expect("checked").shape = Shape::Stroke;
            return true;
        }
        if key == 'w' {
            let mut changed = false;
            for letter in self.state.letters.iter_mut().rev() {
                match (letter.base, letter.shape) {
                    ('a', Shape::Plain) => {
                        letter.shape = Shape::Breve;
                        changed = true;
                        break;
                    }
                    ('o' | 'u', Shape::Plain) => {
                        letter.shape = Shape::Horn;
                        changed = true;
                    }
                    _ if changed => break,
                    _ => {}
                }
            }
            if changed {
                return true;
            }
        }
        if matches!(key, 'a' | 'e' | 'o') {
            if let Some(letter) = self.state.letters.last_mut() {
                if letter.base == key && letter.shape == Shape::Plain {
                    letter.shape = Shape::Circumflex;
                    return true;
                }
            }
        }
        false
    }

    fn apply_vni(&mut self, input: char) -> bool {
        let key = input.to_ascii_lowercase();
        if let Some(tone) = match key {
            '1' => Some(Tone::Acute),
            '2' => Some(Tone::Grave),
            '3' => Some(Tone::Hook),
            '4' => Some(Tone::Tilde),
            '5' => Some(Tone::Dot),
            _ => None,
        } {
            if self.state.letters.iter().any(|letter| letter.is_vowel()) {
                self.state.tone = tone;
                return true;
            }
        }
        let shape = match key {
            '6' => Some(Shape::Circumflex),
            '7' => Some(Shape::Horn),
            '8' => Some(Shape::Breve),
            _ => None,
        };
        if let Some(shape) = shape {
            for letter in self.state.letters.iter_mut().rev() {
                let valid = match shape {
                    Shape::Circumflex => matches!(letter.base, 'a' | 'e' | 'o'),
                    Shape::Horn => matches!(letter.base, 'o' | 'u'),
                    Shape::Breve => letter.base == 'a',
                    _ => false,
                };
                if valid {
                    letter.shape = shape;
                    return true;
                }
            }
        }
        if key == '9' && self.state.letters.last().is_some_and(|l| l.base == 'd') {
            self.state.letters.last_mut().expect("checked").shape = Shape::Stroke;
            return true;
        }
        if key == '0' {
            self.state.tone = Tone::None;
            for letter in &mut self.state.letters {
                letter.shape = Shape::Plain;
            }
            return true;
        }
        false
    }
}

fn diff(old: &str, new: &str) -> Edit {
    let common = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .count();
    Edit {
        delete_chars: old.chars().count() - common,
        insert: new.chars().skip(common).collect(),
    }
}

fn render_letter(letter: Letter, tone: Option<Tone>) -> char {
    let tone = tone.unwrap_or(Tone::None);
    let row = match (letter.base, letter.shape) {
        ('a', Shape::Plain) => "aáàảãạ",
        ('a', Shape::Breve) => "ăắằẳẵặ",
        ('a', Shape::Circumflex) => "âấầẩẫậ",
        ('e', Shape::Plain) => "eéèẻẽẹ",
        ('e', Shape::Circumflex) => "êếềểễệ",
        ('i', _) => "iíìỉĩị",
        ('o', Shape::Plain) => "oóòỏõọ",
        ('o', Shape::Circumflex) => "ôốồổỗộ",
        ('o', Shape::Horn) => "ơớờởỡợ",
        ('u', Shape::Plain) => "uúùủũụ",
        ('u', Shape::Horn) => "ưứừửữự",
        ('y', _) => "yýỳỷỹỵ",
        ('d', Shape::Stroke) => "đđđđđđ",
        _ => {
            return if letter.uppercase {
                letter.base.to_ascii_uppercase()
            } else {
                letter.base
            };
        }
    };
    let index = match tone {
        Tone::None => 0,
        Tone::Acute => 1,
        Tone::Grave => 2,
        Tone::Hook => 3,
        Tone::Tilde => 4,
        Tone::Dot => 5,
    };
    let value = row
        .chars()
        .nth(index)
        .expect("Vietnamese table has six columns");
    if letter.uppercase {
        value.to_uppercase().next().unwrap_or(value)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(method: InputMethod, input: &str) -> String {
        let mut engine = Engine::new(method);
        let mut output = String::new();
        for value in input.chars() {
            let edit = engine.push(value);
            for _ in 0..edit.delete_chars {
                output.pop();
            }
            output.push_str(&edit.insert);
        }
        output
    }

    #[test]
    fn telex_words() {
        assert_eq!(type_text(InputMethod::Telex, "tieengs"), "tiếng");
        assert_eq!(type_text(InputMethod::Telex, "DDuowngf"), "Đường");
        assert_eq!(type_text(InputMethod::Telex, "Vieetj"), "Việt");
        assert_eq!(type_text(InputMethod::Telex, "hoaf"), "hoà");
    }

    #[test]
    fn vni_words() {
        assert_eq!(type_text(InputMethod::Vni, "tie6ng1"), "tiếng");
        assert_eq!(type_text(InputMethod::Vni, "d9u7o7ng2"), "đường");
    }

    #[test]
    fn backspace_restores_composition() {
        let mut engine = Engine::default();
        for value in "aas".chars() {
            engine.push(value);
        }
        assert_eq!(engine.rendered, "ấ");
        assert_eq!(
            engine.backspace(),
            Edit {
                delete_chars: 1,
                insert: "â".into()
            }
        );
        assert_eq!(
            engine.backspace(),
            Edit {
                delete_chars: 1,
                insert: "a".into()
            }
        );
    }

    #[test]
    fn macro_expansion() {
        let mut engine = Engine::default();
        engine.add_macro("btw", "by the way");
        for value in "btw".chars() {
            engine.push(value);
        }
        assert_eq!(
            engine.push(' '),
            Edit {
                delete_chars: 3,
                insert: "by the way ".into()
            }
        );
    }
}
