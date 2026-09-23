// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The paper dictionary. It is not the desktop's `i18n/*.json`: the core
//! cannot read the desktop's files, and a printed document says things a
//! screen never does. The parity this file checks is the same one the
//! desktop's parity test checks, one key at a time in three languages.

use std::collections::HashSet;

use dzpos_core::lang::Lang;
use dzpos_core::print::strings::{shop_text, text, Key, ShopKey};

/// `Key::ALL` is written by hand beside the enum, so it is the one place a
/// key can be forgotten or listed twice. The exact count fails when a key
/// is added to the enum and not to the list, and the set fails when one is
/// pasted in twice: a duplicate would hide a missing key behind a length
/// that still adds up.
///
/// 53, not the 77 this file pinned before S4 of
/// `a-kernel-crate-and-retail-as-the-first-module`: S4 moved the twenty-four
/// shop-word variants out of `Key` into `ShopKey`, which
/// `the_shop_dictionary_lists_every_key_once` below pins the same way.
#[test]
fn the_dictionary_lists_every_key_once() {
    assert_eq!(
        Key::ALL.len(),
        53,
        "a key was added to the enum, not to ALL"
    );
    let listed: HashSet<Key> = Key::ALL.into_iter().collect();
    assert_eq!(listed.len(), Key::ALL.len(), "a key is listed twice in ALL");
}

/// `ShopKey::ALL`'s own count, the sibling check to the one above: S4 moved
/// twenty-four variants here and none were dropped or duplicated on the way.
#[test]
fn the_shop_dictionary_lists_every_key_once() {
    assert_eq!(
        ShopKey::ALL.len(),
        24,
        "a key was added to the enum, not to ALL"
    );
    let listed: HashSet<ShopKey> = ShopKey::ALL.into_iter().collect();
    assert_eq!(
        listed.len(),
        ShopKey::ALL.len(),
        "a key is listed twice in ALL"
    );
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

/// The same parity, walked over `ShopKey` rather than `Key`: a shop word
/// added without its Arabic is a failing test here too, not a French word
/// on an Arabic ticket.
#[test]
fn every_shop_key_is_written_in_all_three_languages() {
    for key in ShopKey::ALL {
        for lang in Lang::ALL {
            let printed = shop_text(key, lang);
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

/// The same check over `ShopKey`.
#[test]
fn the_shop_arabic_text_is_arabic_and_not_a_copy_of_the_french() {
    for key in ShopKey::ALL {
        let ar = shop_text(key, Lang::Ar);
        assert_ne!(
            ar,
            shop_text(key, Lang::Fr),
            "{key:?} was never translated into Arabic"
        );
        assert!(
            ar.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)),
            "{key:?} has no Arabic letters in its Arabic text: {ar}"
        );
    }
}
