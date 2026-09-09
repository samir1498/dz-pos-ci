// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The paper dictionary. It is not the desktop's `i18n/*.json`: the core
//! cannot read the desktop's files, and a printed document says things a
//! screen never does. The parity this file checks is the same one the
//! desktop's parity test checks, one key at a time in three languages.

use dzpos_core::lang::Lang;
use dzpos_core::print::strings::{text, Key};

#[test]
fn every_key_is_written_in_all_three_languages() {
    assert!(Key::ALL.len() >= 14, "the dictionary lost keys");
    for key in Key::ALL {
        for lang in Lang::ALL {
            let printed = text(key, lang);
            assert!(!printed.is_empty(), "{key:?} has no {lang:?} text");
            assert_eq!(
                printed.trim(),
                printed,
                "{key:?} in {lang:?} carries whitespace the template decides"
            );
        }
    }
}

/// A key that reads the same in French and in English is fine ("Total");
/// one that reads the same in Arabic as in French is a key nobody
/// translated, and on paper that is a document in the wrong language.
#[test]
fn the_arabic_text_is_arabic_and_not_a_copy_of_the_french() {
    for key in Key::ALL {
        let ar = text(key, Lang::Ar);
        assert_ne!(
            ar,
            text(key, Lang::Fr),
            "{key:?} was never translated into Arabic"
        );
        assert!(
            ar.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)),
            "{key:?} has no Arabic letters in its Arabic text: {ar}"
        );
    }
}
