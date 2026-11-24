//! Converting encoded letters to characters of a code table.

use crate::{
    config::CodeTable,
    keys::{KEY_A, KEY_E, KEY_O, KEY_U, key_code_to_character},
    tables::{UNICODE_COMPOUND_MARK, code_table_row},
    word::{
        CAPS_MASK, CHAR_CODE_MASK, CHAR_MASK, MARK_MASK, MARK1_MASK, MARK2_MASK, MARK3_MASK,
        MARK4_MASK, MARK5_MASK, PURE_CHARACTER_MASK, TONE_MASK, TONEW_MASK,
    },
};

impl CodeTable {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Unicode => 0,
            Self::Tcvn3 => 1,
            Self::VniWindows => 2,
            Self::UnicodeCompound => 3,
            Self::Cp1258 => 4,
        }
    }
}

/// The table character for a letter with diacritics, tagged with
/// `CHAR_CODE_MASK`, or `data` unchanged when it is a plain key.
pub(crate) fn character_code(data: u32, table: CodeTable) -> u32 {
    let caps_elem = usize::from(data & CAPS_MASK == 0);
    let mut key = data & CHAR_MASK;
    let table = table.index();
    if data & MARK_MASK != 0 {
        let mark_elem = match data & MARK_MASK {
            MARK1_MASK => 0,
            MARK2_MASK => 2,
            MARK3_MASK => 4,
            MARK4_MASK => 6,
            MARK5_MASK => 8,
            _ => return data,
        };
        let mut mark_elem = mark_elem + caps_elem;
        if matches!(key, KEY_A | KEY_O | KEY_U | KEY_E)
            && data & TONE_MASK == 0
            && data & TONEW_MASK == 0
        {
            mark_elem += 4;
        }
        if data & TONE_MASK != 0 {
            key |= TONE_MASK;
        } else if data & TONEW_MASK != 0 {
            key |= TONEW_MASK;
        }
        return code_table_row(table, key)
            .and_then(|row| row.get(mark_elem))
            .map_or(data, |&code| u32::from(code) | CHAR_CODE_MASK);
    }
    let Some(row) = code_table_row(table, key) else {
        return data;
    };
    let index = if data & TONE_MASK != 0 {
        caps_elem
    } else if data & TONEW_MASK != 0 {
        caps_elem + 2
    } else {
        return data;
    };
    row.get(index)
        .map_or(data, |&code| u32::from(code) | CHAR_CODE_MASK)
}

/// Appends the UTF-16 units the host sends for `data` and returns how many
/// characters the target application sees for it (2 for a two-part letter
/// in a double-code table, otherwise 1).
pub(crate) fn encode(data: u32, table: CodeTable, out: &mut Vec<u16>) -> u8 {
    #[allow(clippy::cast_possible_truncation, reason = "keeps the low 16 bits")]
    let unit = data as u16;
    if data & PURE_CHARACTER_MASK != 0 {
        out.push(unit);
        return 1;
    }
    if data & CHAR_CODE_MASK == 0 {
        if let Some(value) = key_code_to_character(data) {
            let mut buffer = [0; 2];
            out.extend_from_slice(value.encode_utf16(&mut buffer));
        }
        return 1;
    }
    match table {
        CodeTable::Unicode => {
            out.push(unit);
            1
        }
        CodeTable::Tcvn3 | CodeTable::VniWindows | CodeTable::Cp1258 => {
            let [high, low] = unit.to_be_bytes();
            out.push(u16::from(low));
            if high > 32 {
                out.push(u16::from(high));
                if table == CodeTable::VniWindows {
                    return 2;
                }
            }
            1
        }
        CodeTable::UnicodeCompound => {
            let mark = unit >> 13;
            out.push(unit & 0x1FFF);
            if mark > 0 {
                out.push(UNICODE_COMPOUND_MARK[usize::from(mark - 1)]);
                return 2;
            }
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{KEY_D, KEY_I, KEY_Y};

    fn unicode(data: u32) -> char {
        let code = character_code(data, CodeTable::Unicode);
        assert_ne!(code & CHAR_CODE_MASK, 0, "{data:#x}");
        char::from_u32(code & CHAR_MASK).expect("valid char")
    }

    fn text(data: u32, table: CodeTable) -> (String, u8) {
        let mut units = Vec::new();
        let length = encode(character_code(data, table), table, &mut units);
        (String::from_utf16(&units).expect("utf16"), length)
    }

    #[test]
    fn maps_unicode_letters() {
        assert_eq!(unicode(KEY_A | TONE_MASK), 'â');
        assert_eq!(unicode(KEY_A | TONE_MASK | MARK1_MASK), 'ấ');
        assert_eq!(unicode(KEY_A | TONEW_MASK), 'ă');
        assert_eq!(unicode(KEY_A | TONEW_MASK | MARK5_MASK), 'ặ');
        assert_eq!(unicode(KEY_O | TONEW_MASK), 'ơ');
        assert_eq!(unicode(KEY_O | TONEW_MASK | MARK3_MASK), 'ở');
        assert_eq!(unicode(KEY_U | TONEW_MASK), 'ư');
        assert_eq!(unicode(KEY_U | TONEW_MASK | MARK5_MASK), 'ự');
        assert_eq!(unicode(KEY_D | TONE_MASK), 'đ');
        assert_eq!(unicode(KEY_D | TONE_MASK | CAPS_MASK), 'Đ');
        assert_eq!(unicode(KEY_E | TONE_MASK), 'ê');
        assert_eq!(unicode(KEY_E | TONE_MASK | MARK5_MASK), 'ệ');
        assert_eq!(unicode(KEY_I | MARK1_MASK), 'í');
        assert_eq!(unicode(KEY_Y | MARK2_MASK), 'ỳ');
        assert_eq!(unicode(KEY_A | MARK4_MASK | CAPS_MASK), 'Ã');
    }

    #[test]
    fn keeps_plain_keys() {
        assert_eq!(character_code(KEY_A, CodeTable::Unicode), KEY_A);
        assert_eq!(
            character_code(KEY_D | CAPS_MASK, CodeTable::Unicode),
            KEY_D | CAPS_MASK
        );
    }

    #[test]
    fn encodes_other_tables() {
        assert_eq!(
            text(KEY_A | TONE_MASK, CodeTable::Tcvn3),
            ("\u{A9}".into(), 1)
        );
        assert_eq!(
            text(KEY_A | TONE_MASK | MARK1_MASK, CodeTable::VniWindows),
            ("a\u{E1}".into(), 2)
        );
        assert_eq!(
            text(KEY_D | TONE_MASK, CodeTable::VniWindows),
            ("\u{F1}".into(), 1)
        );
        assert_eq!(
            text(KEY_A | TONE_MASK | MARK1_MASK, CodeTable::UnicodeCompound),
            ("â\u{301}".into(), 2)
        );
        assert_eq!(
            text(KEY_E | MARK2_MASK, CodeTable::Cp1258),
            ("e\u{CC}".into(), 1)
        );
        assert_eq!(text(KEY_A | CAPS_MASK, CodeTable::Unicode), ("A".into(), 1));
    }
}
