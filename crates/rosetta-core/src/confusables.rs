//! Detection of words that mix Latin letters with look-alike scripts.
//!
//! The usual spoofing attack writes one word with letters from two scripts,
//! for example a Cyrillic "а" (U+0430) inside a Latin word. Such a word looks
//! like an ordinary Latin word but is a different string. This module finds
//! words that mix Latin with Greek, Cyrillic or Armenian letters.
//!
//! It does not detect words written entirely in one non-Latin script that look
//! like Latin words. Telling those apart needs the full UTS #39 confusables
//! table, which is not included. See `docs/architecture.md`.

/// Scripts whose letters are commonly used to imitate Latin letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Latin,
    Greek,
    Cyrillic,
    Armenian,
}

/// Returns the script of a letter, for the scripts this module tracks.
///
/// Other letters, digits and punctuation return `None` and do not affect the
/// result.
pub fn script_of(c: char) -> Option<Script> {
    if !c.is_alphabetic() {
        return None;
    }
    let code = c as u32;
    match code {
        0x41..=0x5A | 0x61..=0x7A => Some(Script::Latin),
        0xAA | 0xBA => Some(Script::Latin),
        0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x24F => Some(Script::Latin),
        0x1E00..=0x1EFF => Some(Script::Latin),
        0x370..=0x3FF | 0x1F00..=0x1FFF => Some(Script::Greek),
        0x400..=0x52F => Some(Script::Cyrillic),
        0x530..=0x58F => Some(Script::Armenian),
        _ => None,
    }
}

/// Returns `true` if `word` contains Latin letters and at least one letter from
/// Greek, Cyrillic or Armenian.
pub fn is_mixed_script_word(word: &str) -> bool {
    let mut latin = false;
    let mut other = false;
    for c in word.chars() {
        match script_of(c) {
            Some(Script::Latin) => latin = true,
            Some(_) => other = true,
            None => {}
        }
    }
    latin && other
}

/// Returns the words in `text` that mix Latin with another tracked script.
///
/// Words are runs of alphanumeric characters, so punctuation separates them.
pub fn mixed_script_words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| is_mixed_script_word(word))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cyrillic_letter_inside_latin_word() {
        // "pаypal" with a Cyrillic а (U+0430) in place of the Latin a.
        assert!(is_mixed_script_word("p\u{430}ypal"));
    }

    #[test]
    fn detects_greek_letter_inside_latin_word() {
        // "Αpple" with a Greek capital alpha (U+0391).
        assert!(is_mixed_script_word("\u{391}pple"));
    }

    #[test]
    fn plain_latin_and_plain_cyrillic_words_are_not_flagged() {
        assert!(!is_mixed_script_word("paypal"));
        assert!(!is_mixed_script_word("привет"));
    }

    #[test]
    fn accented_latin_is_not_mixed() {
        assert!(!is_mixed_script_word("café"));
    }

    #[test]
    fn finds_only_mixed_words_in_a_sentence() {
        let text = "Visit p\u{430}ypal.com and read the paypal docs.";
        assert_eq!(mixed_script_words(text), vec!["p\u{430}ypal"]);
    }

    #[test]
    fn digits_and_punctuation_do_not_count_as_a_script() {
        assert!(!is_mixed_script_word("v2.0-beta_1"));
    }
}
