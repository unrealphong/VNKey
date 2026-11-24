//! The engine's internal letter encoding.
//!
//! Each typed letter is a `u32`: the low 16 bits hold a key code (or, once
//! converted, a character code) and the high bits hold case, diacritics and
//! tone flags.

/// Capital letter.
pub(crate) const CAPS_MASK: u32 = 0x1_0000;
/// Circumflex (`â ê ô`), or the stroke of `đ`.
pub(crate) const TONE_MASK: u32 = 0x2_0000;
/// Breve or horn (`ă ơ ư`).
pub(crate) const TONEW_MASK: u32 = 0x4_0000;

/// Sắc.
pub(crate) const MARK1_MASK: u32 = 0x8_0000;
/// Huyền.
pub(crate) const MARK2_MASK: u32 = 0x10_0000;
/// Hỏi.
pub(crate) const MARK3_MASK: u32 = 0x20_0000;
/// Ngã.
pub(crate) const MARK4_MASK: u32 = 0x40_0000;
/// Nặng.
pub(crate) const MARK5_MASK: u32 = 0x80_0000;
pub(crate) const MARK_MASK: u32 = 0xF8_0000;

pub(crate) const CHAR_MASK: u32 = 0xFFFF;
/// Produced by a standalone key (`w`, `[`, `]`).
pub(crate) const STANDALONE_MASK: u32 = 0x100_0000;
/// The low bits are a character code, not a key code.
pub(crate) const CHAR_CODE_MASK: u32 = 0x200_0000;
/// A literal character that must be sent as is.
pub(crate) const PURE_CHARACTER_MASK: u32 = 0x8000_0000;

/// In tables: a shortened end consonant (`g` for `ng`), only valid with quick
/// consonants enabled.
pub(crate) const END_CONSONANT_MASK: u32 = 0x4000;
/// In tables: an initial consonant only valid when `z f w j` are allowed.
pub(crate) const CONSONANT_ALLOW_MASK: u32 = 0x8000;

pub(crate) const MAX_BUFF: usize = 32;

/// The key code part of an encoded letter.
pub(crate) fn chr(data: u32) -> u32 {
    data & CHAR_MASK
}
