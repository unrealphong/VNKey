//! The typing state machine.
//!
//! Every key press updates the current word and fills [`Hook`] with how the
//! text before the cursor must change.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    reason = "word positions are small signed indices"
)]

use crate::{
    charset::character_code,
    config::{Config, InputType},
    keys::{
        KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_A,
        KEY_BACK_SLASH, KEY_BACKQUOTE, KEY_C, KEY_COMMA, KEY_D, KEY_DELETE, KEY_DOT, KEY_DOWN,
        KEY_E, KEY_ENTER, KEY_EQUALS, KEY_ESC, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J, KEY_K, KEY_LEFT,
        KEY_LEFT_BRACKET, KEY_M, KEY_MINUS, KEY_N, KEY_O, KEY_P, KEY_Q, KEY_QUOTE, KEY_R,
        KEY_RETURN, KEY_RIGHT, KEY_RIGHT_BRACKET, KEY_S, KEY_SEMICOLON, KEY_SLASH, KEY_SPACE,
        KEY_T, KEY_TAB, KEY_U, KEY_UP, KEY_W, KEY_X, KEY_Y, KEY_Z,
    },
    macros::MacroTable,
    tables::{
        CONSONANT_D, CONSONANT_TABLE, DOUBLE_W_ALLOWED, END_CONSONANT_TABLE, Rows,
        STANDALONE_W_BAD, VOWEL_FOR_MARK, quick_end_consonant, quick_start_consonant, quick_telex,
        vowel, vowel_combine,
    },
    word::{
        CAPS_MASK, CONSONANT_ALLOW_MASK, END_CONSONANT_MASK, IMPLIED_HORN_MASK, MARK_MASK,
        MARK1_MASK, MARK2_MASK, MARK3_MASK, MARK4_MASK, MARK5_MASK, MAX_BUFF, STANDALONE_MASK,
        TONE_MASK, TONEW_MASK, chr,
    },
};

pub(crate) const DO_NOTHING: u8 = 0;
pub(crate) const WILL_PROCESS: u8 = 1;
pub(crate) const RESTORE: u8 = 3;
pub(crate) const REPLACE_MACRO: u8 = 4;
pub(crate) const RESTORE_AND_START_NEW_SESSION: u8 = 5;

const MAX: i32 = MAX_BUFF as i32;

/// Keys that type a character but are not letters.
pub(crate) const CHAR_KEY_CODE: [u32; 21] = [
    KEY_BACKQUOTE,
    KEY_1,
    KEY_2,
    KEY_3,
    KEY_4,
    KEY_5,
    KEY_6,
    KEY_7,
    KEY_8,
    KEY_9,
    KEY_0,
    KEY_MINUS,
    KEY_EQUALS,
    KEY_LEFT_BRACKET,
    KEY_RIGHT_BRACKET,
    KEY_BACK_SLASH,
    KEY_SEMICOLON,
    KEY_QUOTE,
    KEY_COMMA,
    KEY_DOT,
    KEY_SLASH,
];

const BREAK_CODE: [u32; 17] = [
    KEY_ESC,
    KEY_TAB,
    KEY_ENTER,
    KEY_RETURN,
    KEY_LEFT,
    KEY_RIGHT,
    KEY_DOWN,
    KEY_UP,
    KEY_COMMA,
    KEY_DOT,
    KEY_SLASH,
    KEY_SEMICOLON,
    KEY_QUOTE,
    KEY_BACK_SLASH,
    KEY_MINUS,
    KEY_EQUALS,
    KEY_BACKQUOTE,
];

const MACRO_BREAK_CODE: [u32; 9] = [
    KEY_RETURN,
    KEY_COMMA,
    KEY_DOT,
    KEY_SLASH,
    KEY_SEMICOLON,
    KEY_QUOTE,
    KEY_BACK_SLASH,
    KEY_MINUS,
    KEY_EQUALS,
];

/// Tone, modifier, `đ` and remove-tone keys: s f r x j a o e w d z.
const TELEX_KEYS: [u32; 11] = [
    KEY_S, KEY_F, KEY_R, KEY_X, KEY_J, KEY_A, KEY_O, KEY_E, KEY_W, KEY_D, KEY_Z,
];
const VNI_KEYS: [u32; 11] = [
    KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0,
];

/// What the front-end must do after an event.
#[derive(Clone, Debug)]
pub(crate) struct Hook {
    pub code: u8,
    pub backspace_count: i32,
    pub new_char_count: i32,
    /// 1 word break, 2 delete, 3 normal key, 4 no empty character needed.
    pub ext_code: u8,
    /// New characters, last character first.
    pub char_data: [u32; MAX_BUFF],
    pub macro_key: Vec<u32>,
    pub macro_data: Vec<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct Core {
    pub config: Config,
    pub hook: Hook,
    pub macros: MacroTable,
    check_spelling: bool,
    use_spell_checking_before: bool,

    typing_word: [u32; MAX_BUFF],
    index: i32,
    long_word_helper: Vec<u32>,
    typing_states: Vec<Vec<u32>>,
    key_states: [u32; MAX_BUFF],
    state_index: i32,

    temp_disable_key: bool,
    is_correct: bool,
    is_changed: bool,
    vowel_count: i32,
    vowel_start: i32,
    vowel_end: i32,
    vowel_will_set_mark: i32,
    is_checked_grammar: bool,
    is_caps: bool,
    space_count: i32,
    has_handled_macro: bool,
    upper_case_status: u8,
    special_char: Vec<u32>,
    has_handle_quick_consonant: bool,
    will_temp_off_engine: bool,
}

fn is_consonant(code: u32) -> bool {
    !matches!(code, KEY_A | KEY_E | KEY_U | KEY_Y | KEY_I | KEY_O)
}

fn is_number_key(code: u32) -> bool {
    VNI_KEYS.contains(&code)
}

fn is_bracket_key(code: u32) -> bool {
    code == KEY_LEFT_BRACKET || code == KEY_RIGHT_BRACKET
}

fn caps(is_caps: bool) -> u32 {
    if is_caps { CAPS_MASK } else { 0 }
}

fn len(row: &[u32]) -> i32 {
    row.len() as i32
}

impl Core {
    pub(crate) fn new(config: Config) -> Self {
        let check_spelling = config.check_spelling;
        Self {
            config,
            hook: Hook {
                code: DO_NOTHING,
                backspace_count: 0,
                new_char_count: 0,
                ext_code: 0,
                char_data: [0; MAX_BUFF],
                macro_key: Vec::new(),
                macro_data: Vec::new(),
            },
            macros: MacroTable::new(),
            check_spelling,
            use_spell_checking_before: check_spelling,
            typing_word: [0; MAX_BUFF],
            index: 0,
            long_word_helper: Vec::new(),
            typing_states: Vec::new(),
            key_states: [0; MAX_BUFF],
            state_index: 0,
            temp_disable_key: false,
            is_correct: false,
            is_changed: false,
            vowel_count: 0,
            vowel_start: 0,
            vowel_end: 0,
            vowel_will_set_mark: 0,
            is_checked_grammar: false,
            is_caps: false,
            space_count: 0,
            has_handled_macro: false,
            upper_case_status: 0,
            special_char: Vec::new(),
            has_handle_quick_consonant: false,
            will_temp_off_engine: false,
        }
    }

    pub(crate) fn set_config(&mut self, config: Config) {
        self.check_spelling = config.check_spelling;
        self.use_spell_checking_before = config.check_spelling;
        self.macros.set_code_table(config.code_table);
        self.config = config;
    }

    // ---- word access -------------------------------------------------

    fn w(&self, index: i32) -> u32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.typing_word.get(index))
            .copied()
            .unwrap_or(0)
    }

    fn c(&self, index: i32) -> u32 {
        chr(self.w(index))
    }

    fn set_w(&mut self, index: i32, value: u32) {
        if let Some(slot) = usize::try_from(index)
            .ok()
            .and_then(|index| self.typing_word.get_mut(index))
        {
            *slot = value;
        }
    }

    fn or_w(&mut self, index: i32, mask: u32) {
        self.set_w(index, self.w(index) | mask);
    }

    fn clear_w(&mut self, index: i32, mask: u32) {
        self.set_w(index, self.w(index) & !mask);
    }

    fn set_data(&mut self, index: i32, value: u32) {
        if let Some(slot) = usize::try_from(index)
            .ok()
            .and_then(|index| self.hook.char_data.get_mut(index))
        {
            *slot = value;
        }
    }

    fn code(&self, data: u32) -> u32 {
        character_code(data, self.config.code_table)
    }

    // ---- input type ----------------------------------------------------

    fn vni(&self) -> bool {
        self.config.input_type == InputType::Vni
    }

    fn processing(&self, index: usize) -> u32 {
        if self.vni() {
            VNI_KEYS[index]
        } else {
            TELEX_KEYS[index]
        }
    }

    fn is_key_z(&self, key: u32) -> bool {
        self.processing(10) == key
    }

    fn is_key_d(&self, key: u32) -> bool {
        self.processing(9) == key
    }

    fn is_key_w(&self, key: u32) -> bool {
        self.processing(8) == key || (self.vni() && self.processing(7) == key)
    }

    fn is_key_double(&self, key: u32) -> bool {
        if self.vni() {
            self.processing(6) == key
        } else {
            self.processing(5) == key || self.processing(6) == key || self.processing(7) == key
        }
    }

    fn mark_for_key(&self, key: u32) -> Option<u32> {
        [MARK1_MASK, MARK2_MASK, MARK3_MASK, MARK4_MASK, MARK5_MASK]
            .into_iter()
            .enumerate()
            .find(|&(index, _)| self.processing(index) == key)
            .map(|(_, mark)| mark)
    }

    fn is_mark_key(&self, key: u32) -> bool {
        if self.vni() {
            matches!(key, KEY_1 | KEY_2 | KEY_3 | KEY_4 | KEY_5)
        } else {
            matches!(key, KEY_S | KEY_F | KEY_R | KEY_J | KEY_X)
        }
    }

    fn is_special_key(&self, key: u32) -> bool {
        match self.config.input_type {
            InputType::Telex => matches!(
                key,
                KEY_W
                    | KEY_E
                    | KEY_R
                    | KEY_O
                    | KEY_LEFT_BRACKET
                    | KEY_RIGHT_BRACKET
                    | KEY_A
                    | KEY_S
                    | KEY_D
                    | KEY_F
                    | KEY_J
                    | KEY_Z
                    | KEY_X
            ),
            InputType::Vni => is_number_key(key),
            InputType::SimpleTelex1 | InputType::SimpleTelex2 => matches!(
                key,
                KEY_W
                    | KEY_E
                    | KEY_R
                    | KEY_O
                    | KEY_A
                    | KEY_S
                    | KEY_D
                    | KEY_F
                    | KEY_J
                    | KEY_Z
                    | KEY_X
            ),
        }
    }

    fn is_quick_telex_key(&self, key: u32) -> bool {
        self.index > 0
            && matches!(key, KEY_C | KEY_G | KEY_K | KEY_N | KEY_Q | KEY_P | KEY_T)
            && self.c(self.index - 1) == key
    }

    fn is_word_break(data: u32) -> bool {
        BREAK_CODE.contains(&data)
    }

    fn is_macro_break_code(data: u32) -> bool {
        MACRO_BREAK_CODE.contains(&data)
    }

    fn find_macro(&mut self) -> bool {
        match self
            .macros
            .find(&mut self.hook.macro_key, self.config.auto_caps_macro)
        {
            Some(data) => {
                self.hook.macro_data = data;
                true
            }
            None => false,
        }
    }

    // ---- word bookkeeping ----------------------------------------------

    fn set_key_data(&mut self, index: i32, key_code: u32, is_caps: bool) {
        if !(0..MAX).contains(&index) {
            return;
        }
        self.set_w(index, key_code | caps(is_caps));
    }

    fn insert_key(&mut self, key_code: u32, is_caps: bool, check_spelling: bool) {
        if self.index >= MAX {
            self.long_word_helper.push(self.typing_word[0]);
            self.typing_word.copy_within(1.., 0);
            self.set_key_data(self.index - 1, key_code, is_caps);
        } else {
            self.set_key_data(self.index, key_code, is_caps);
            self.index += 1;
        }
        if self.check_spelling && check_spelling {
            self.run_check_spelling(false);
        }
        if key_code == KEY_D && self.index - 2 >= 0 && is_consonant(self.c(self.index - 2)) {
            self.temp_disable_key = false;
        }
    }

    fn insert_state(&mut self, key_code: u32, is_caps: bool) {
        let value = key_code | caps(is_caps);
        if self.state_index >= MAX {
            self.key_states.copy_within(1.., 0);
            self.key_states[MAX_BUFF - 1] = value;
        } else {
            self.key_states[self.state_index as usize] = value;
            self.state_index += 1;
        }
    }

    fn push_chunks(&mut self, data: &[u32]) {
        let mut chunk = Vec::new();
        for (index, &value) in data.iter().enumerate() {
            if index != 0 && index % MAX_BUFF == 0 {
                self.typing_states.push(std::mem::take(&mut chunk));
            }
            chunk.push(value);
        }
        self.typing_states.push(chunk);
    }

    fn save_word(&mut self) {
        if self.hook.code == REPLACE_MACRO {
            let data = self.hook.macro_data.clone();
            self.push_chunks(&data);
            return;
        }
        if self.index > 0 {
            if !self.long_word_helper.is_empty() {
                let long = std::mem::take(&mut self.long_word_helper);
                self.push_chunks(&long);
            }
            let word = self.typing_word[..self.index as usize].to_vec();
            self.typing_states.push(word);
        }
    }

    fn save_repeated(&mut self, key_code: u32, count: i32) {
        self.typing_states
            .push(vec![key_code; usize::try_from(count).unwrap_or(0)]);
    }

    fn save_special_char(&mut self) {
        let special = std::mem::take(&mut self.special_char);
        self.typing_states.push(special);
    }

    fn restore_last_typing_state(&mut self) {
        let Some(data) = self.typing_states.pop() else {
            return;
        };
        let Some(&first) = data.first() else {
            return;
        };
        if first == KEY_SPACE {
            self.space_count = data.len() as i32;
            self.index = 0;
        } else if CHAR_KEY_CODE.contains(&chr(first)) {
            self.index = 0;
            self.special_char = data;
            self.run_check_spelling(false);
        } else {
            for (index, &value) in data.iter().enumerate().take(MAX_BUFF) {
                self.typing_word[index] = value;
            }
            self.index = i32::from(data.len() as u8);
        }
    }

    pub(crate) fn start_new_session(&mut self) {
        self.index = 0;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 0;
        self.temp_disable_key = false;
        self.state_index = 0;
        self.has_handled_macro = false;
        self.has_handle_quick_consonant = false;
        self.long_word_helper.clear();
    }

    pub(crate) fn reset_composition(&mut self) {
        self.start_new_session();
        self.typing_states.clear();
        self.special_char.clear();
        self.hook.macro_key.clear();
        self.space_count = 0;
        self.upper_case_status = 0;
        self.check_spelling = self.use_spell_checking_before;
        self.will_temp_off_engine = false;
    }

    // ---- spelling ------------------------------------------------------

    fn run_check_spelling(&mut self, force_check_vowel: bool) {
        let quick_start = if self.config.quick_start_consonant {
            END_CONSONANT_MASK
        } else {
            0
        };
        let allow_zfwj = if self.config.allow_consonant_zfwj {
            CONSONANT_ALLOW_MASK
        } else {
            0
        };
        let quick_end = if self.config.quick_end_consonant {
            END_CONSONANT_MASK
        } else {
            0
        };
        let mut spelling_ok = false;
        let mut vowel_ok = true;
        let mut end = self.index;
        if self.index > 0 && self.c(self.index - 1) == KEY_RIGHT_BRACKET {
            end = self.index - 1;
        }

        if end > 0 {
            let mut j: i32 = 0;
            if is_consonant(self.c(0)) {
                for row in CONSONANT_TABLE {
                    let mut flag = end < len(row);
                    j = 0;
                    while j < len(row) {
                        let entry = row[j as usize];
                        if end > j
                            && (entry & !quick_start) != self.c(j)
                            && (entry & !allow_zfwj) != self.c(j)
                        {
                            flag = true;
                            break;
                        }
                        j += 1;
                    }
                    if flag {
                        continue;
                    }
                    break;
                }
            }

            if j == end {
                spelling_ok = true;
            }

            let mut k = j;
            self.vowel_start = k;
            if self.c(self.vowel_start) == KEY_U
                && k > 0
                && k < end - 1
                && self.c(self.vowel_start - 1) == KEY_Q
            {
                k += 1;
                j = k;
                self.vowel_start = k;
            } else if self.index >= 2
                && self.c(0) == KEY_G
                && self.c(1) == KEY_I
                && is_consonant(self.c(2))
            {
                self.vowel_start = 1;
                k = 1;
                j = 1;
            }
            for _ in 0..3 {
                if k < end && !is_consonant(self.c(k)) {
                    k += 1;
                    self.vowel_end = k;
                }
            }

            if k > j {
                vowel_ok = false;
                if k - j > 1 && force_check_vowel {
                    for row in vowel_combine(self.c(j)) {
                        let mut flag = false;
                        let mut ii = 1;
                        while ii < len(row) {
                            let position = j + ii - 1;
                            let typed = self.c(position)
                                | (self.w(position) & TONEW_MASK)
                                | (self.w(position) & TONE_MASK);
                            if position < end && row[ii as usize] != typed {
                                flag = true;
                                break;
                            }
                            ii += 1;
                        }
                        if flag
                            || (k < end && row[0] == 0)
                            || (j + ii - 1 < end && !is_consonant(self.c(j + ii - 1)))
                        {
                            continue;
                        }
                        vowel_ok = true;
                        break;
                    }
                } else if !is_consonant(self.c(j)) {
                    vowel_ok = true;
                }

                for row in END_CONSONANT_TABLE {
                    let mut flag = false;
                    j = 0;
                    while j < len(row) {
                        if end > k + j && (row[j as usize] & !quick_end) != self.c(k + j) {
                            flag = true;
                            break;
                        }
                        j += 1;
                    }
                    if flag {
                        continue;
                    }
                    if k + j >= end {
                        spelling_ok = true;
                        break;
                    }
                }

                if spelling_ok {
                    let index = self.index;
                    let only_sac_or_nang = |data: u32| {
                        data & MARK1_MASK != 0 || data & MARK5_MASK != 0 || data & MARK_MASK == 0
                    };
                    let ends_with_ch =
                        index >= 3 && self.c(index - 1) == KEY_H && self.c(index - 2) == KEY_C;
                    if (ends_with_ch && !only_sac_or_nang(self.w(index - 3)))
                        || (!ends_with_ch
                            && index >= 2
                            && self.c(index - 1) == KEY_T
                            && !only_sac_or_nang(self.w(index - 2)))
                    {
                        spelling_ok = false;
                    }
                }
            }
        } else {
            spelling_ok = true;
        }
        self.temp_disable_key = !(spelling_ok && vowel_ok);
    }

    fn check_grammar(&mut self, delta_backspace: i32) {
        if self.index <= 1 || self.index >= MAX {
            return;
        }
        self.find_and_calculate_vowel(true);
        if self.vowel_count == 0 {
            return;
        }
        self.is_checked_grammar = false;
        let l = self.vowel_start;

        let last = self.index - 1;
        if delta_backspace == -1
            && self.c(last) == KEY_O
            && self.c(last - 1) == KEY_U
            && self.w(last - 1) & TONEW_MASK != 0
        {
            self.or_w(last, TONEW_MASK | IMPLIED_HORN_MASK);
            self.is_checked_grammar = true;
        }

        let adjust_grammar = !self.config.free_mark || delta_backspace == 1;
        if adjust_grammar && self.index >= 3 {
            for i in (0..self.index).rev() {
                if matches!(
                    self.c(i),
                    KEY_N | KEY_C | KEY_I | KEY_M | KEY_P | KEY_T | KEY_U
                ) && i - 2 >= l
                    && self.c(i - 1) == KEY_O
                    && self.c(i - 2) == KEY_U
                    && (self.w(i - 1) & TONEW_MASK) ^ (self.w(i - 2) & TONEW_MASK) != 0
                {
                    self.or_w(i - 2, TONEW_MASK);
                    self.or_w(i - 1, TONEW_MASK);
                    self.is_checked_grammar = true;
                    break;
                }
            }
        }

        if adjust_grammar {
            let mut i = l;
            while i <= self.vowel_end {
                if self.w(i) & MARK_MASK != 0 {
                    let mark = self.w(i) & MARK_MASK;
                    let pending = (
                        self.hook.backspace_count,
                        self.hook.new_char_count,
                        self.hook.char_data,
                    );
                    self.clear_w(i, MARK_MASK);
                    self.insert_mark(mark, false);
                    if i == self.vowel_will_set_mark {
                        (
                            self.hook.backspace_count,
                            self.hook.new_char_count,
                            self.hook.char_data,
                        ) = pending;
                    } else {
                        self.is_checked_grammar = true;
                    }
                    break;
                }
                i += 1;
            }
        }

        if self.is_checked_grammar {
            if self.hook.code == DO_NOTHING {
                self.hook.code = WILL_PROCESS;
            }
            self.hook.backspace_count = 0;
            for i in (l..self.index).rev() {
                self.hook.backspace_count += 1;
                let value = self.code(self.w(i));
                self.set_data(self.index - 1 - i, value);
            }
            self.hook.new_char_count = self.hook.backspace_count;
            self.hook.backspace_count += delta_backspace;
            self.hook.ext_code = 4;
        }
    }

    fn check_correct_vowel(&mut self, charset: Rows, row: usize, mark_key: u32) {
        if self.index >= 2 && self.c(self.index - 1) == KEY_U && self.c(self.index - 2) == KEY_Q {
            self.is_correct = false;
            return;
        }
        let quick_end = if self.config.quick_end_consonant {
            END_CONSONANT_MASK
        } else {
            0
        };
        let entries = charset[row];
        let mut k = self.index - 1;
        let mut j = len(entries) - 1;
        while j >= 0 {
            if (entries[j as usize] & !quick_end) != self.c(k) {
                self.is_correct = false;
                return;
            }
            k -= 1;
            if k < 0 {
                break;
            }
            j -= 1;
        }

        if self.is_correct
            && entries.len() > 1
            && (self.processing(1) == mark_key
                || self.processing(3) == mark_key
                || self.processing(2) == mark_key)
            && (entries[1] == KEY_C
                || entries[1] == KEY_T
                || (entries.len() > 2 && entries[2] == KEY_T))
        {
            self.is_correct = false;
        }

        if self.is_correct && k >= 0 && self.c(k) == self.c(k + 1) {
            self.is_correct = false;
        }
    }

    fn find_and_calculate_vowel(&mut self, for_grammar: bool) {
        self.vowel_count = 0;
        self.vowel_start = 0;
        self.vowel_end = 0;
        let mut iii = self.index - 1;
        while iii >= 0 {
            if is_consonant(self.c(iii)) {
                if self.vowel_count > 0 {
                    break;
                }
            } else {
                if self.vowel_count == 0 {
                    self.vowel_end = iii;
                }
                if !for_grammar
                    && iii > 0
                    && ((self.c(iii) == KEY_I && self.c(iii - 1) == KEY_G)
                        || (self.c(iii) == KEY_U && self.c(iii - 1) == KEY_Q))
                {
                    break;
                }
                self.vowel_start = iii;
                self.vowel_count += 1;
            }
            iii -= 1;
        }
        if self.vowel_start > 0
            && self.c(self.vowel_start) == KEY_U
            && self.c(self.vowel_start - 1) == KEY_Q
        {
            self.vowel_start += 1;
            self.vowel_count -= 1;
        }
    }

    // ---- tone marks ----------------------------------------------------

    fn remove_mark(&mut self) {
        self.find_and_calculate_vowel(true);
        self.is_changed = false;
        if self.index > 0 {
            for i in self.vowel_start..=self.vowel_end {
                if self.w(i) & MARK_MASK != 0 {
                    self.clear_w(i, MARK_MASK);
                    self.is_changed = true;
                }
            }
        }
        if self.is_changed {
            self.hook.code = WILL_PROCESS;
            self.hook.backspace_count = 0;
            for i in (self.vowel_start..self.index).rev() {
                self.hook.backspace_count += 1;
                let value = self.code(self.w(i));
                self.set_data(self.index - 1 - i, value);
            }
            self.hook.new_char_count = self.hook.backspace_count;
        } else {
            self.hook.code = DO_NOTHING;
        }
    }

    fn can_has_end_consonant(&self) -> bool {
        for row in vowel_combine(self.c(self.vowel_start)) {
            let mut kk = self.vowel_start;
            let mut iii = 1;
            while iii < len(row) {
                let typed = self.c(kk) | (self.w(kk) & TONE_MASK) | (self.w(kk) & TONEW_MASK);
                if kk > self.vowel_end || typed != row[iii as usize] {
                    break;
                }
                kk += 1;
                iii += 1;
            }
            if iii >= len(row) {
                return row[0] == 1;
            }
        }
        false
    }

    fn set_mark_position(&mut self, position: i32) {
        self.vowel_will_set_mark = position;
        self.hook.backspace_count = self.index - position;
    }

    /// Where the tone goes with modern orthography (`oà`, `uý`).
    fn modern_mark_position(&self) -> i32 {
        let vsi = self.vowel_start;
        let vei = self.vowel_end;
        let c = |index: i32| self.c(index);
        let w = |index: i32| self.w(index);
        let mut position = vei;

        if self.vowel_count == 3
            && ((c(vsi) == KEY_O && c(vsi + 1) == KEY_A && c(vsi + 2) == KEY_I)
                || (c(vsi) == KEY_U && c(vsi + 1) == KEY_Y && c(vsi + 2) == KEY_U)
                || (c(vsi) == KEY_O && c(vsi + 1) == KEY_E && c(vsi + 2) == KEY_O)
                || (c(vsi) == KEY_U && c(vsi + 1) == KEY_Y && c(vsi + 2) == KEY_A))
        {
            position = vsi + 1;
        } else if (c(vsi) == KEY_O && c(vsi + 1) == KEY_I)
            || (c(vsi) == KEY_A && c(vsi + 1) == KEY_I)
            || (c(vsi) == KEY_U && c(vsi + 1) == KEY_I)
        {
            position = vsi;
        } else if c(vei - 1) == KEY_A && c(vei) == KEY_Y {
            position = vei - 1;
        } else if c(vsi) == KEY_U && c(vsi + 1) == KEY_O {
            position = vsi + 1;
        } else if c(vsi + 1) == KEY_O || c(vsi + 1) == KEY_U {
            position = vei - 1;
        } else if c(vsi) == KEY_O || c(vsi) == KEY_U {
            position = vei;
        }

        if (c(vsi) == KEY_I && w(vsi + 1) & (KEY_E | TONE_MASK) != 0)
            || (c(vsi) == KEY_Y && w(vsi + 1) & (KEY_E | TONE_MASK) != 0)
            || (c(vsi) == KEY_U && w(vsi + 1) == (KEY_O | TONE_MASK))
            || (w(vsi) == (KEY_U | TONEW_MASK) && w(vsi + 1) == (KEY_O | TONEW_MASK))
        {
            position = if vsi + 2 < self.index
                && matches!(
                    c(vsi + 2),
                    KEY_P | KEY_T | KEY_M | KEY_N | KEY_O | KEY_U | KEY_I | KEY_C
                ) {
                vsi + 1
            } else {
                vsi
            };
        } else if c(vsi) == KEY_U && w(vsi + 1) == (KEY_U | TONEW_MASK) {
            position = vsi;
        }

        if self.vowel_count == 2 {
            if c(vsi) == KEY_I && matches!(c(vsi + 1), KEY_A | KEY_U | KEY_O) {
                position = if vsi == 0 || c(vsi - 1) != KEY_G {
                    vsi
                } else {
                    vsi + 1
                };
            } else if c(vsi) == KEY_U && c(vsi + 1) == KEY_A {
                if vsi != 0 && c(vsi - 1) == KEY_Q {
                    position = vsi + 1;
                } else if vei + 1 >= self.index || !self.can_has_end_consonant() {
                    position = vsi;
                }
            } else if c(vsi) == KEY_O && c(vsi + 1) == KEY_O {
                position = vei;
            }
        }
        position
    }

    /// Where the tone goes with the old orthography (`òa`, `úy`).
    fn old_mark_position(&self) -> i32 {
        let vsi = self.vowel_start;
        let vei = self.vowel_end;
        let mut position = if self.vowel_count == 0 && self.c(vei) == KEY_I {
            vei
        } else {
            vsi
        };
        if self.vowel_count == 3
            || (vei + 1 < self.index
                && is_consonant(self.c(vei + 1))
                && self.can_has_end_consonant())
        {
            position = vsi + 1;
        }
        for ii in vsi..=vei {
            if (self.c(ii) == KEY_E && self.w(ii) & TONE_MASK != 0)
                || (self.c(ii) == KEY_O && self.w(ii) & TONEW_MASK != 0)
            {
                position = ii;
                break;
            }
        }
        position
    }

    fn insert_mark(&mut self, mark_mask: u32, can_modify_flag: bool) {
        self.vowel_count = 0;
        if can_modify_flag {
            self.hook.code = WILL_PROCESS;
        }
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 0;

        self.find_and_calculate_vowel(false);
        self.vowel_will_set_mark = 0;

        if self.vowel_count == 1 {
            self.vowel_will_set_mark = self.vowel_end;
            self.hook.backspace_count = self.index - self.vowel_end;
        } else {
            if self.config.modern_orthography {
                let position = self.modern_mark_position();
                self.set_mark_position(position);
            } else {
                let position = self.old_mark_position();
                self.set_mark_position(position);
                self.hook.new_char_count = self.hook.backspace_count;
            }
            if self.w(self.vowel_end) & TONE_MASK != 0 || self.w(self.vowel_end) & TONEW_MASK != 0 {
                self.vowel_will_set_mark = self.vowel_end;
            }
        }

        let mark_at = self.vowel_will_set_mark;
        let mut kk = self.index - 1 - self.vowel_start;
        if self.w(mark_at) & mark_mask != 0 {
            self.clear_w(mark_at, MARK_MASK);
            if can_modify_flag {
                self.hook.code = RESTORE;
            }
            for ii in self.vowel_start..self.index {
                self.clear_w(ii, MARK_MASK);
                let value = self.code(self.w(ii));
                self.set_data(kk, value);
                kk -= 1;
            }
            self.temp_disable_key = true;
        } else {
            self.clear_w(mark_at, MARK_MASK);
            self.or_w(mark_at, mark_mask);
            for ii in self.vowel_start..self.index {
                if ii != mark_at {
                    self.clear_w(ii, MARK_MASK);
                }
                let value = self.code(self.w(ii));
                self.set_data(kk, value);
                kk -= 1;
            }
            self.hook.backspace_count = self.index - self.vowel_start;
        }
        self.hook.new_char_count = self.hook.backspace_count;
    }

    // ---- modifiers -----------------------------------------------------

    fn insert_d(&mut self) {
        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 0;
        for ii in (0..self.index).rev() {
            self.hook.backspace_count += 1;
            let slot = self.index - 1 - ii;
            if self.c(ii) == KEY_D {
                if self.w(ii) & TONE_MASK != 0 {
                    self.hook.code = RESTORE;
                    self.clear_w(ii, TONE_MASK);
                    self.set_data(slot, self.w(ii));
                    self.temp_disable_key = true;
                } else {
                    self.or_w(ii, TONE_MASK);
                    let value = self.code(self.w(ii));
                    self.set_data(slot, value);
                }
                break;
            }
            let value = self.code(self.w(ii));
            self.set_data(slot, value);
        }
        self.hook.new_char_count = self.hook.backspace_count;
    }

    fn insert_aoe(&mut self, data: u32) {
        self.find_and_calculate_vowel(false);
        for ii in self.vowel_start..=self.vowel_end {
            self.clear_w(ii, TONEW_MASK | IMPLIED_HORN_MASK);
        }
        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 0;
        for ii in (0..self.index).rev() {
            self.hook.backspace_count += 1;
            let slot = self.index - 1 - ii;
            if self.c(ii) == data {
                if self.w(ii) & TONE_MASK != 0 {
                    self.hook.code = RESTORE;
                    self.clear_w(ii, TONE_MASK);
                    self.set_data(slot, self.w(ii));
                    if data != KEY_O {
                        self.temp_disable_key = true;
                    }
                } else {
                    self.or_w(ii, TONE_MASK);
                    if !self.is_key_d(data) {
                        self.clear_w(ii, TONEW_MASK);
                    }
                    let value = self.code(self.w(ii));
                    self.set_data(slot, value);
                }
                break;
            }
            let value = self.code(self.w(ii));
            self.set_data(slot, value);
        }
        self.hook.new_char_count = self.hook.backspace_count;
    }

    fn insert_w(&mut self) {
        self.find_and_calculate_vowel(false);
        let vsi = self.vowel_start;
        if self.vowel_count > 1 && self.w(vsi + 1) & IMPLIED_HORN_MASK != 0 {
            // Accept an explicit horn in sequences such as `uwow` and `u7o7`.
            self.clear_w(vsi + 1, IMPLIED_HORN_MASK);
            self.hook.code = WILL_PROCESS;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 0;
            return;
        }
        for ii in vsi..=self.vowel_end {
            self.clear_w(ii, TONE_MASK);
        }

        if self.vowel_count > 1 {
            self.hook.backspace_count = self.index - vsi;
            self.hook.new_char_count = self.hook.backspace_count;
            if self.w(vsi) & TONEW_MASK != 0
                && (self.w(vsi + 1) & TONEW_MASK != 0 || matches!(self.c(vsi + 1), KEY_I | KEY_A))
            {
                self.hook.code = RESTORE;
                for ii in vsi..self.index {
                    self.clear_w(ii, TONEW_MASK);
                    let value = self.code(self.w(ii)) & !STANDALONE_MASK;
                    self.set_data(self.index - 1 - ii, value);
                }
                self.temp_disable_key = true;
            } else {
                self.hook.code = WILL_PROCESS;
                let (first, second) = (self.c(vsi), self.c(vsi + 1));
                if first == KEY_U && second == KEY_O {
                    if self.index == vsi + 2
                        && vsi >= 2
                        && self.c(vsi - 2) == KEY_T
                        && self.c(vsi - 1) == KEY_H
                    {
                        self.or_w(vsi + 1, TONEW_MASK);
                    } else {
                        self.or_w(vsi, TONEW_MASK);
                        self.or_w(vsi + 1, TONEW_MASK);
                    }
                } else if (first == KEY_U && matches!(second, KEY_A | KEY_I | KEY_U))
                    || (first == KEY_O && second == KEY_I)
                {
                    self.or_w(vsi, TONEW_MASK);
                } else if (first == KEY_I && second == KEY_O) || (first == KEY_O && second == KEY_A)
                {
                    self.or_w(vsi + 1, TONEW_MASK);
                } else {
                    self.temp_disable_key = true;
                    self.is_changed = false;
                    self.hook.code = DO_NOTHING;
                }
                for ii in vsi..self.index {
                    let value = self.code(self.w(ii));
                    self.set_data(self.index - 1 - ii, value);
                }
            }
            return;
        }

        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 0;
        for ii in (0..self.index).rev() {
            if ii < vsi {
                break;
            }
            self.hook.backspace_count += 1;
            let slot = self.index - 1 - ii;
            if matches!(self.c(ii), KEY_A | KEY_U | KEY_O) {
                if self.w(ii) & TONEW_MASK != 0 {
                    let word_caps = self.w(ii) & CAPS_MASK;
                    if self.w(ii) & STANDALONE_MASK != 0 {
                        self.hook.code = WILL_PROCESS;
                        if self.c(ii) == KEY_U {
                            self.set_w(ii, KEY_W | word_caps);
                        } else if self.c(ii) == KEY_O {
                            self.hook.code = RESTORE;
                            self.set_w(ii, KEY_O | word_caps);
                        }
                        self.set_data(slot, self.w(ii));
                    } else {
                        self.hook.code = RESTORE;
                        self.clear_w(ii, TONEW_MASK);
                        let value = self.code(self.w(ii));
                        self.set_data(slot, value);
                    }
                    self.temp_disable_key = true;
                } else {
                    self.or_w(ii, TONEW_MASK);
                    self.clear_w(ii, TONE_MASK);
                    let value = self.code(self.w(ii));
                    self.set_data(slot, value);
                }
            } else {
                let value = self.code(self.w(ii));
                self.set_data(slot, value);
            }
        }
        self.hook.new_char_count = self.hook.backspace_count;
    }

    fn reverse_last_standalone_char(&mut self, key_code: u32, is_caps: bool) {
        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 1;
        self.hook.ext_code = 4;
        let last = self.index - 1;
        self.set_w(
            last,
            key_code | TONEW_MASK | STANDALONE_MASK | caps(is_caps),
        );
        let value = self.code(self.w(last));
        self.set_data(0, value);
    }

    fn check_for_standalone_char(&mut self, data: u32, is_caps: bool, key_will_reverse: u32) {
        let last = self.index - 1;
        if self.c(last) == key_will_reverse && self.w(last) & TONEW_MASK != 0 {
            self.hook.code = WILL_PROCESS;
            self.hook.backspace_count = 1;
            self.hook.new_char_count = 1;
            self.set_w(last, data | caps(is_caps));
            let value = self.code(self.w(last));
            self.set_data(0, value);
            return;
        }

        if self.index > 0 && self.c(last) == KEY_U && key_will_reverse == KEY_O {
            self.insert_key(key_will_reverse, is_caps, true);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
            return;
        }

        match self.index {
            0 => {
                self.insert_key(data, is_caps, false);
                self.reverse_last_standalone_char(key_will_reverse, is_caps);
            }
            1 => {
                if STANDALONE_W_BAD.contains(&self.c(0)) {
                    self.insert_key(data, is_caps, true);
                    return;
                }
                self.insert_key(data, is_caps, false);
                self.reverse_last_standalone_char(key_will_reverse, is_caps);
            }
            2 => {
                let start = [self.c(0), self.c(1)];
                if DOUBLE_W_ALLOWED.contains(&start) {
                    self.insert_key(data, is_caps, false);
                    self.reverse_last_standalone_char(key_will_reverse, is_caps);
                    return;
                }
                self.insert_key(data, is_caps, true);
            }
            _ => self.insert_key(data, is_caps, true),
        }
    }

    fn upper_case_first_character(&mut self) {
        if self.w(0) & CAPS_MASK != 0 {
            return;
        }
        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 0;
        self.hook.new_char_count = 1;
        self.or_w(0, CAPS_MASK);
        let value = self.code(self.w(0));
        self.set_data(0, value);
        self.upper_case_status = 0;
        if self.config.use_macro {
            if let Some(first) = self.hook.macro_key.first_mut() {
                *first |= CAPS_MASK;
            }
        }
    }

    fn handle_main_key(&mut self, data: u32, is_caps: bool) {
        if self.is_key_z(data) {
            self.remove_mark();
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }
        if data == KEY_LEFT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_O);
            return;
        }
        if data == KEY_RIGHT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_U);
            return;
        }

        if self.is_key_d(data) {
            self.is_correct = false;
            self.is_changed = false;
            for (row, entries) in CONSONANT_D.iter().enumerate() {
                if self.index < len(entries) {
                    continue;
                }
                self.is_correct = true;
                self.check_correct_vowel(CONSONANT_D, row, data);
                if !self.is_correct
                    && self.index - 2 >= 0
                    && self.c(self.index - 1) == KEY_D
                    && is_consonant(self.c(self.index - 2))
                {
                    self.is_correct = true;
                }
                if self.is_correct {
                    self.is_changed = true;
                    self.insert_d();
                    break;
                }
            }
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        if self.is_mark_key(data) {
            for charset in VOWEL_FOR_MARK {
                self.is_correct = false;
                self.is_changed = false;
                for (row, entries) in charset.iter().enumerate() {
                    if self.index < len(entries) {
                        continue;
                    }
                    self.is_correct = true;
                    self.check_correct_vowel(charset, row, data);
                    if self.is_correct {
                        self.is_changed = true;
                        if let Some(mark) = self.mark_for_key(data) {
                            self.insert_mark(mark, true);
                        }
                        break;
                    }
                }
                if self.is_correct {
                    break;
                }
            }
            if !self.is_changed {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        if self.vni() {
            for i in (0..self.index).rev() {
                if matches!(self.c(i), KEY_O | KEY_A | KEY_E) {
                    self.vowel_end = i;
                    break;
                }
            }
        }

        let key_for_aeo = if !self.vni() {
            data
        } else if data == KEY_7 || data == KEY_8 {
            KEY_W
        } else if data == KEY_6 {
            self.c(self.vowel_end)
        } else {
            data
        };
        let charset = vowel(key_for_aeo);
        self.is_correct = false;
        self.is_changed = false;
        for (row, entries) in charset.iter().enumerate() {
            if self.index < len(entries) {
                continue;
            }
            self.is_correct = true;
            self.check_correct_vowel(charset, row, data);
            if self.is_correct {
                self.is_changed = true;
                if self.is_key_double(data) {
                    self.insert_aoe(key_for_aeo);
                } else if self.is_key_w(data) {
                    if self.vni() {
                        for j in (0..self.index).rev() {
                            if matches!(self.c(j), KEY_O | KEY_U | KEY_A | KEY_E) {
                                self.vowel_end = j;
                                break;
                            }
                        }
                        let vei = self.vowel_end;
                        if (data == KEY_7
                            && self.c(vei) == KEY_A
                            && (vei - 1 < 0 || self.c(vei - 1) != KEY_U))
                            || (data == KEY_8 && matches!(self.c(vei), KEY_O | KEY_U))
                        {
                            break;
                        }
                    }
                    self.insert_w();
                }
                break;
            }
        }

        if !self.is_changed {
            if data == KEY_W && self.config.input_type != InputType::SimpleTelex1 {
                self.check_for_standalone_char(data, is_caps, KEY_U);
            } else {
                self.insert_key(data, is_caps, true);
            }
        }
    }

    fn handle_quick_telex(&mut self, data: u32, is_caps: bool) {
        let Some([first, second]) = quick_telex(data) else {
            return;
        };
        self.hook.code = WILL_PROCESS;
        self.hook.backspace_count = 1;
        self.hook.new_char_count = 2;
        self.hook.char_data[1] = first | caps(is_caps);
        self.hook.char_data[0] = second | caps(is_caps);
        self.insert_key(second, is_caps, false);
    }

    fn check_restore_if_wrong_spelling(&mut self, handle_code: u8) -> bool {
        for ii in 0..self.index {
            if !is_consonant(self.c(ii)) && self.w(ii) & (MARK_MASK | TONE_MASK | TONEW_MASK) != 0 {
                self.hook.code = handle_code;
                self.hook.backspace_count = self.index;
                self.hook.new_char_count = self.state_index;
                for i in 0..self.state_index {
                    self.set_w(i, self.key_states[i as usize]);
                    self.set_data(self.state_index - 1 - i, self.w(i));
                }
                self.index = self.state_index;
                return true;
            }
        }
        false
    }

    fn check_quick_consonant(&mut self) -> bool {
        if self.index <= 1 {
            return false;
        }
        let mut handled = false;
        if self.config.quick_start_consonant {
            if let Some([first, second]) = quick_start_consonant(self.c(0)) {
                self.hook.code = RESTORE;
                self.hook.backspace_count = self.index;
                self.hook.new_char_count = self.index + 1;
                if self.index < MAX - 1 {
                    self.index += 1;
                }
                for i in (2..self.index).rev() {
                    self.set_w(i, self.w(i - 1));
                }
                let both_caps = self.w(0) & CAPS_MASK != 0 && self.w(2) & CAPS_MASK != 0;
                self.set_w(1, second | caps(both_caps));
                self.set_w(0, first | (self.w(0) & CAPS_MASK));
                handled = true;
            }
        }
        if self.config.quick_end_consonant
            && self.index - 2 >= 0
            && !is_consonant(self.c(self.index - 2))
        {
            if let Some([first, second]) = quick_end_consonant(self.c(self.index - 1)) {
                self.hook.code = RESTORE;
                if handled {
                    self.hook.new_char_count += 1;
                } else {
                    self.hook.backspace_count = 1;
                    self.hook.new_char_count = 2;
                }
                if self.index < MAX - 1 {
                    self.index += 1;
                }
                let last_caps = self.w(self.index - 2) & CAPS_MASK;
                self.set_w(self.index - 1, second | last_caps);
                self.set_w(self.index - 2, first | last_caps);
                handled = true;
            }
        }
        if handled {
            self.has_handle_quick_consonant = true;
            for i in (0..self.index).rev() {
                let value = self.code(self.w(i));
                self.set_data(self.index - 1 - i, value);
            }
        }
        handled
    }

    // ---- entry points --------------------------------------------------

    /// Tracks the word typed while Vietnamese is off, for macros.
    pub(crate) fn english_mode(&mut self, data: u32, is_caps: bool, other_control_key: bool) {
        self.hook.code = DO_NOTHING;
        if other_control_key && !is_caps {
            self.hook.macro_key.clear();
            self.will_temp_off_engine = false;
        } else if data == KEY_SPACE {
            if !self.has_handled_macro && self.find_macro() {
                self.hook.code = REPLACE_MACRO;
                self.hook.backspace_count = self.hook.macro_key.len() as i32;
            }
            self.hook.macro_key.clear();
            self.will_temp_off_engine = false;
        } else if data == KEY_DELETE {
            if self.hook.macro_key.pop().is_none() {
                self.will_temp_off_engine = false;
            }
        } else if Self::is_word_break(data) && !CHAR_KEY_CODE.contains(&data) {
            self.hook.macro_key.clear();
            self.will_temp_off_engine = false;
        } else if !self.will_temp_off_engine {
            self.hook.macro_key.push(data | caps(is_caps));
        }
    }

    /// Handles a key press.
    ///
    /// `caps_status` is 1 for Shift, 2 for Caps Lock, 0 otherwise.
    pub(crate) fn handle_event(&mut self, data: u32, caps_status: u8, other_control_key: bool) {
        self.is_caps = caps_status == 1 || caps_status == 2;
        let is_caps = self.is_caps;

        if (is_number_key(data) && caps_status == 1)
            || other_control_key
            || Self::is_word_break(data)
            || (self.index == 0 && is_number_key(data))
        {
            self.hook.code = DO_NOTHING;
            self.hook.backspace_count = 0;
            self.hook.new_char_count = 0;
            self.hook.ext_code = 1;

            if self.config.use_macro
                && Self::is_macro_break_code(data)
                && !self.has_handled_macro
                && self.find_macro()
            {
                self.hook.code = REPLACE_MACRO;
                self.hook.backspace_count = self.hook.macro_key.len() as i32;
                self.has_handled_macro = true;
            } else if (self.config.quick_start_consonant || self.config.quick_end_consonant)
                && !self.temp_disable_key
                && Self::is_macro_break_code(data)
            {
                self.check_quick_consonant();
            } else if self.config.restore_if_wrong_spelling && Self::is_word_break(data) {
                if !self.temp_disable_key && self.check_spelling {
                    self.run_check_spelling(true);
                }
                if self.temp_disable_key
                    && !self.check_restore_if_wrong_spelling(RESTORE_AND_START_NEW_SESSION)
                {
                    self.hook.code = DO_NOTHING;
                }
            }

            let is_char_key_code = CHAR_KEY_CODE.contains(&data);
            if is_char_key_code {
                if self.space_count > 0 {
                    self.save_repeated(KEY_SPACE, self.space_count);
                    self.space_count = 0;
                } else {
                    self.save_word();
                }
                self.special_char.push(data | caps(is_caps));
                self.hook.ext_code = 3;
            } else {
                self.special_char.clear();
                self.typing_states.clear();
            }

            if self.hook.code == DO_NOTHING {
                self.start_new_session();
                self.check_spelling = self.use_spell_checking_before;
                self.will_temp_off_engine = false;
            } else if self.hook.code == REPLACE_MACRO || self.has_handle_quick_consonant {
                self.index = 0;
            }

            if self.config.use_macro {
                if is_char_key_code {
                    self.hook.macro_key.push(data | caps(is_caps));
                } else {
                    self.hook.macro_key.clear();
                }
            }

            if self.config.upper_case_first_char {
                self.upper_case_status = if data == KEY_DOT {
                    1
                } else if data == KEY_ENTER || data == KEY_RETURN {
                    2
                } else {
                    0
                };
            }
        } else if data == KEY_SPACE {
            if !self.temp_disable_key && self.check_spelling {
                self.run_check_spelling(true);
            }
            if self.config.use_macro && !self.has_handled_macro && self.find_macro() {
                self.hook.code = REPLACE_MACRO;
                self.hook.backspace_count = self.hook.macro_key.len() as i32;
                self.space_count += 1;
                self.has_handled_macro = true;
            } else if (self.config.quick_start_consonant || self.config.quick_end_consonant)
                && !self.temp_disable_key
                && self.check_quick_consonant()
            {
                self.space_count += 1;
            } else if self.config.restore_if_wrong_spelling
                && self.temp_disable_key
                && !self.has_handled_macro
            {
                if !self.check_restore_if_wrong_spelling(RESTORE) {
                    self.hook.code = DO_NOTHING;
                }
                self.space_count += 1;
            } else {
                self.hook.code = DO_NOTHING;
                self.space_count += 1;
            }
            if self.config.use_macro {
                self.hook.macro_key.clear();
            }
            if self.config.upper_case_first_char && self.upper_case_status == 1 {
                self.upper_case_status = 2;
            }
            if self.space_count == 1 {
                if self.special_char.is_empty() {
                    self.save_word();
                } else {
                    self.save_special_char();
                }
            }
            self.check_spelling = self.use_spell_checking_before;
            self.will_temp_off_engine = false;
        } else if data == KEY_DELETE {
            self.hook.code = DO_NOTHING;
            self.hook.ext_code = 2;
            if !self.special_char.is_empty() {
                self.special_char.pop();
                if self.special_char.is_empty() {
                    self.restore_last_typing_state();
                }
            } else if self.space_count > 0 {
                self.space_count -= 1;
                if self.space_count == 0 {
                    self.restore_last_typing_state();
                }
            } else {
                if self.state_index > 0 {
                    self.state_index -= 1;
                }
                if self.index > 0 {
                    self.index -= 1;
                    if let Some(first) = self.long_word_helper.pop() {
                        self.typing_word.copy_within(..MAX_BUFF - 1, 1);
                        self.typing_word[0] = first;
                        self.index += 1;
                    }
                    if self.check_spelling {
                        self.run_check_spelling(false);
                    }
                }
                if self.config.use_macro {
                    self.hook.macro_key.pop();
                }
                self.hook.backspace_count = 0;
                self.hook.new_char_count = 0;
                self.hook.ext_code = 2;
                if self.index == 0 {
                    self.start_new_session();
                    self.special_char.clear();
                    self.restore_last_typing_state();
                } else {
                    self.check_grammar(1);
                }
            }
        } else {
            if self.will_temp_off_engine {
                self.hook.code = DO_NOTHING;
                self.hook.ext_code = 3;
                return;
            }
            if self.space_count > 0 {
                self.hook.backspace_count = 0;
                self.hook.new_char_count = 0;
                self.hook.ext_code = 0;
                self.start_new_session();
                self.save_repeated(KEY_SPACE, self.space_count);
                self.space_count = 0;
            } else if !self.special_char.is_empty() {
                self.save_special_char();
            }

            self.insert_state(data, is_caps);

            if !self.is_special_key(data) || self.temp_disable_key {
                if self.config.quick_telex && self.is_quick_telex_key(data) {
                    self.handle_quick_telex(data, is_caps);
                    return;
                }
                self.hook.code = DO_NOTHING;
                self.hook.backspace_count = 0;
                self.hook.new_char_count = 0;
                self.hook.ext_code = 3;
                self.insert_key(data, is_caps, true);
            } else {
                self.hook.code = DO_NOTHING;
                self.hook.ext_code = 3;
                self.handle_main_key(data, is_caps);
            }

            if !self.is_key_d(data) {
                if self.hook.code == DO_NOTHING {
                    self.check_grammar(-1);
                } else {
                    self.check_grammar(0);
                }
            }

            if self.hook.code == RESTORE {
                self.insert_key(data, is_caps, true);
                self.state_index -= 1;
            }

            if self.config.use_macro {
                if self.hook.code == DO_NOTHING {
                    self.hook.macro_key.push(data | caps(is_caps));
                } else if self.hook.code == WILL_PROCESS || self.hook.code == RESTORE {
                    for _ in 0..self.hook.backspace_count {
                        self.hook.macro_key.pop();
                    }
                    let start = if self.hook.code == WILL_PROCESS {
                        self.index - self.hook.new_char_count
                    } else {
                        self.index - self.hook.backspace_count
                    };
                    for i in start..start + self.hook.new_char_count {
                        self.hook.macro_key.push(self.w(i));
                    }
                }
            }

            if self.config.upper_case_first_char {
                if self.index == 1 && self.upper_case_status == 2 {
                    self.upper_case_first_character();
                }
                self.upper_case_status = 0;
            }

            if is_bracket_key(data)
                && (is_bracket_key(chr(self.hook.char_data[0]))
                    || matches!(
                        self.config.input_type,
                        InputType::SimpleTelex1 | InputType::SimpleTelex2
                    ))
            {
                let pending = if self.hook.code == WILL_PROCESS {
                    self.hook.backspace_count
                } else {
                    0
                };
                if self.index - pending > 0 {
                    self.index -= 1;
                    self.save_word();
                }
                self.index = 0;
                self.temp_disable_key = false;
                self.state_index = 0;
                self.hook.ext_code = 3;
                self.special_char.push(data | caps(is_caps));
            }
        }
    }
}
