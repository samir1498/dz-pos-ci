// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::message;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
struct Root;

impl fmt::Display for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the shop's files could not complete the operation")
    }
}
impl std::error::Error for Root {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&Cause)
    }
}

#[derive(Debug)]
struct Cause;

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "os error 28: no space left on device")
    }
}
impl std::error::Error for Cause {}

#[test]
fn all_three_languages_are_in_the_lead_sentence() {
    let text = message(&Root, None);
    assert!(text.contains("n'a pas pu démarrer"));
    assert!(text.contains("could not start"));
    assert!(text.contains("لم يتمكن"));
}

/// The folder is asserted on its own line and not as a substring of the
/// text: every folder here is a prefix of the file path above it, so a
/// bare `contains` would pass with the folder line deleted.
#[test]
fn the_path_and_its_folder_are_named_when_known() {
    let path = Path::new("/home/shop/.local/share/dzpos/dzpos.db");
    let text = message(&Root, Some(path));
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.contains(&"Fichier / File / الملف: /home/shop/.local/share/dzpos/dzpos.db"),
        "{text}"
    );
    assert!(
        lines.contains(&"Dossier / Folder / المجلد: /home/shop/.local/share/dzpos"),
        "{text}"
    );
}

#[test]
fn no_path_is_shown_when_none_was_chosen_yet() {
    let text = message(&Root, None);
    assert!(!text.contains("Fichier"));
}

#[test]
fn the_whole_chain_is_shown_not_only_the_outermost_display() {
    let text = message(&Root, None);
    assert!(text.contains("the shop's files could not complete the operation"));
    assert!(text.contains("os error 28: no space left on device"));
}
