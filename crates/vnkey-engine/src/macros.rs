//! Text shortcuts ("macros") expanded when a word is finished.

use std::collections::BTreeMap;

use crate::{
    charset::character_code,
    config::CodeTable,
    keys::{character_to_key_code, key_code_to_character},
    tables::CODE_TABLES,
    word::{CAPS_MASK, CHAR_CODE_MASK, PURE_CHARACTER_MASK},
};

const FILE_HEADER: &str = ";Compatible VNKey Macro Data file for UniKey*** version=1 ***";

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    content: String,
    content_code: Vec<u32>,
}

/// Shortcut names and their expansions.
#[derive(Clone, Debug, Default)]
pub struct MacroTable {
    entries: BTreeMap<Vec<u32>, Entry>,
    code_table: CodeTable,
}

impl MacroTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a shortcut, or replaces the expansion of an existing one.
    pub fn add(&mut self, name: &str, content: &str) {
        let key = convert(name, self.code_table);
        let content_code = convert(content, self.code_table);
        self.entries
            .entry(key)
            .and_modify(|entry| {
                content.clone_into(&mut entry.content);
                entry.content_code.clone_from(&content_code);
            })
            .or_insert_with(|| Entry {
                name: name.to_owned(),
                content: content.to_owned(),
                content_code,
            });
    }

    /// Removes a shortcut. Returns whether it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.entries
            .remove(&convert(name, self.code_table))
            .is_some()
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(&convert(name, self.code_table))
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `(name, expansion)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .values()
            .map(|entry| (entry.name.as_str(), entry.content.as_str()))
    }

    /// Reads the UniKey-compatible text format: a header line, then one
    /// `name:expansion` per line. Existing names are kept.
    pub fn load_text(&mut self, text: &str, append: bool) {
        if !append {
            self.entries.clear();
        }
        for line in text.lines().skip(1) {
            let Some((name, content)) = line.split_once(':') else {
                continue;
            };
            let mut name = name.to_owned();
            let mut content = content.to_owned();
            while name.is_empty() && !content.is_empty() {
                let Some((head, rest)) = content.split_once(':') else {
                    break;
                };
                name = format!(":{head}");
                content = rest.to_owned();
            }
            if !name.is_empty() && !self.contains(&name) {
                self.add(&name, &content);
            }
        }
    }

    /// Writes the format read by [`MacroTable::load_text`].
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!("{FILE_HEADER}\n");
        for (name, content) in self.iter() {
            text.push_str(name);
            text.push(':');
            text.push_str(content);
            text.push('\n');
        }
        text
    }

    pub(crate) fn set_code_table(&mut self, code_table: CodeTable) {
        if self.code_table == code_table {
            return;
        }
        self.code_table = code_table;
        let entries = std::mem::take(&mut self.entries);
        for entry in entries.into_values() {
            self.add(&entry.name, &entry.content);
        }
    }

    /// Looks up the typed word `key`, converting it in place to character
    /// codes (and to lower case when `auto_caps` retries), and returns the
    /// expansion.
    pub(crate) fn find(&self, key: &mut [u32], auto_caps: bool) -> Option<Vec<u32>> {
        let table = self.code_table;
        for code in key.iter_mut() {
            *code = character_code(*code, table);
        }
        if let Some(entry) = self.entries.get(&*key) {
            return Some(entry.content_code.clone());
        }
        if !auto_caps {
            return None;
        }
        let mut all_caps = false;
        if key.len() > 1 && modify_case(&mut key[1], false, table) {
            all_caps = true;
            for code in &mut key[2..] {
                modify_case(code, false, table);
            }
        }
        if key.is_empty() || !modify_case(&mut key[0], false, table) {
            return None;
        }
        let entry = self.entries.get(&*key)?;
        let mut content = entry.content_code.clone();
        for (index, code) in content.iter_mut().enumerate() {
            if index != 0 && !all_caps {
                continue;
            }
            if let Some(value) = key_code_to_character(*code) {
                if let Some(upper) = character_to_key_code(value.to_ascii_uppercase()) {
                    *code = upper;
                }
                continue;
            }
            if *code & CHAR_CODE_MASK != 0 {
                modify_case(code, true, table);
            }
        }
        Some(content)
    }
}

/// Encodes `text` as typed keys, table characters or pure characters.
fn convert(text: &str, table: CodeTable) -> Vec<u32> {
    text.chars()
        .map(|value| {
            if let Some(code) = character_to_key_code(value) {
                return code;
            }
            let unit = u32::from(value) & 0xFFFF;
            for (index, (key, row)) in CODE_TABLES[0].iter().enumerate() {
                if let Some(position) = row.iter().position(|&code| u32::from(code) == unit) {
                    let target = CODE_TABLES[table.index()][index];
                    debug_assert_eq!(target.0, *key);
                    return u32::from(target.1[position]) | CHAR_CODE_MASK;
                }
            }
            u32::from(value) | PURE_CHARACTER_MASK
        })
        .collect()
}

fn modify_case(code: &mut u32, upper: bool, table: CodeTable) -> bool {
    let before = *code;
    if *code & CHAR_CODE_MASK == 0 {
        *code &= if upper { CAPS_MASK } else { !CAPS_MASK };
        return *code != before;
    }
    let unit = *code & 0xFFFF;
    for (_, row) in CODE_TABLES[table.index()] {
        if let Some(mut position) = row.iter().position(|&value| u32::from(value) == unit) {
            if position % 2 == 0 && !upper {
                position += 1;
            } else if position % 2 != 0 && upper {
                position -= 1;
            }
            *code = u32::from(row[position]) | CHAR_CODE_MASK;
            return *code != before;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::key_code_to_character;

    fn typed(text: &str) -> Vec<u32> {
        convert(text, CodeTable::Unicode)
    }

    fn render(codes: &[u32]) -> String {
        codes
            .iter()
            .map(|&code| {
                if code & PURE_CHARACTER_MASK != 0 || code & CHAR_CODE_MASK != 0 {
                    char::from_u32(code & 0xFFFF).expect("char")
                } else {
                    key_code_to_character(code).expect("key")
                }
            })
            .collect()
    }

    fn table() -> MacroTable {
        let mut table = MacroTable::new();
        table.add("btw", "by the way");
        table.add("vn", "Việt Nam");
        table
    }

    #[test]
    fn expands_exact_names() {
        let table = table();
        assert_eq!(
            render(&table.find(&mut typed("btw"), false).expect("btw")),
            "by the way"
        );
        assert_eq!(
            render(&table.find(&mut typed("vn"), false).expect("vn")),
            "Việt Nam"
        );
        assert!(table.find(&mut typed("Btw"), false).is_none());
    }

    #[test]
    fn auto_caps_follows_the_typed_case() {
        let table = table();
        assert_eq!(
            render(&table.find(&mut typed("Btw"), true).expect("Btw")),
            "By the way"
        );
        assert_eq!(
            render(&table.find(&mut typed("BTW"), true).expect("BTW")),
            "BY THE WAY"
        );
        assert_eq!(
            render(&table.find(&mut typed("VN"), true).expect("VN")),
            "VIỆT NAM"
        );
    }

    #[test]
    fn round_trips_the_text_format() {
        let mut loaded = MacroTable::new();
        loaded.load_text(&table().to_text(), false);
        let mut pairs: Vec<_> = loaded.iter().collect();
        pairs.sort_unstable();
        assert_eq!(pairs, vec![("btw", "by the way"), ("vn", "Việt Nam")]);
    }
}
