// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The paper dictionary. It is not the desktop's `i18n/*.json`: the core
//! cannot read the desktop's files, and a printed document says things a
//! screen never does. The parity this file checks is the same one the
//! desktop's parity test checks, one key at a time in three languages.

use std::collections::HashSet;

use dzpos_core::lang::Lang;
use dzpos_core::print::strings::{text, Key};

/// `Key::ALL` is written by hand beside the enum, so it is the one place a
/// key can be forgotten or listed twice. The exact count fails when a key
/// is added to the enum and not to the list, and the set fails when one is
/// pasted in twice: a duplicate would hide a missing key behind a length
/// that still adds up.
#[test]
fn the_dictionary_lists_every_key_once() {
    assert_eq!(
        Key::ALL.len(),
        77,
        "a key was added to the enum, not to ALL"
    );
    let listed: HashSet<Key> = Key::ALL.into_iter().collect();
    assert_eq!(listed.len(), Key::ALL.len(), "a key is listed twice in ALL");
}

#[test]
fn every_key_is_written_in_all_three_languages() {
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
