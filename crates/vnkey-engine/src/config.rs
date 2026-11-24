/// How Vietnamese letters and tones are typed.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum InputType {
    #[default]
    Telex,
    Vni,
    /// Telex without the standalone `w` → `ư` and `[`/`]` shortcuts.
    SimpleTelex1,
    /// Telex without the `[`/`]` shortcuts.
    SimpleTelex2,
}

/// The character encoding Vietnamese letters are sent in.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CodeTable {
    #[default]
    Unicode,
    /// TCVN3 (ABC), one byte per letter.
    Tcvn3,
    /// VNI Windows, up to two bytes per letter.
    VniWindows,
    /// Unicode with combining tone marks.
    UnicodeCompound,
    /// Vietnamese locale code page 1258.
    Cp1258,
}

/// Engine options.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent user-facing setting"
)]
pub struct Config {
    pub input_type: InputType,
    pub code_table: CodeTable,
    /// Place tones the modern way (`oà`, `uý`) instead of `òa`, `úy`.
    pub modern_orthography: bool,
    /// Do not move an already placed tone when the word changes.
    pub free_mark: bool,
    pub check_spelling: bool,
    /// Restore the typed keys when a word ends up misspelled.
    pub restore_if_wrong_spelling: bool,
    /// `cc` → `ch`, `gg` → `gi`, `kk` → `kh`, `nn` → `ng`, `qq` → `qu`,
    /// `pp` → `ph`, `tt` → `th`, `uu` → `ươ`.
    pub quick_telex: bool,
    /// `f` → `ph`, `j` → `gi`, `w` → `qu` at the start of a word.
    pub quick_start_consonant: bool,
    /// `g` → `ng`, `h` → `nh`, `k` → `ch` at the end of a word.
    pub quick_end_consonant: bool,
    /// Capitalise the first letter of a sentence.
    pub upper_case_first_char: bool,
    /// Accept words starting with `z`, `f`, `w` or `j`.
    pub allow_consonant_zfwj: bool,
    pub use_macro: bool,
    pub use_macro_in_english_mode: bool,
    /// `Btw` expands to `By the way`, `BTW` to `BY THE WAY`.
    pub auto_caps_macro: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_type: InputType::Telex,
            code_table: CodeTable::Unicode,
            modern_orthography: true,
            free_mark: false,
            check_spelling: true,
            restore_if_wrong_spelling: true,
            quick_telex: false,
            quick_start_consonant: false,
            quick_end_consonant: false,
            upper_case_first_char: false,
            allow_consonant_zfwj: false,
            use_macro: false,
            use_macro_in_english_mode: false,
            auto_caps_macro: false,
        }
    }
}
