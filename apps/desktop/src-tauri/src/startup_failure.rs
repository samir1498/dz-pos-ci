//! What a shopkeeper sees when Dinar cannot start.
//!
//! A release build on Windows carries no console
//! (`windows_subsystem = "windows"`, main.rs), so `eprintln!` in `main`
//! goes nowhere on that platform: the window never opens and nothing is
//! said. This is the native message box that says something instead.
//!
//! Language: this fires before anything on disk has been read, including
//! the shop's own language preference (`preferences` table), so nothing
//! here knows which of the three the reader has. `dzpos_core::print::strings`
//! holds the printed vocabulary per key in the three languages and needs no
//! database to read it, but its keys are ticket and facture words
//! (`Ticket`, `Tva`, `NetToPay`...); there is no key here to reuse without
//! growing that dictionary for a screen it was never meant to describe.
//! Rather than guess a language or add one, the lead sentence is said in
//! all three: a message in a language the reader does not have is no
//! better than no message. The path and the error chain beneath it are
//! left as they are, in whatever language the underlying error already
//! prints in (mostly SQLite's and diesel's own English), because a file
//! path and a driver's error text are not the kind of thing translation
//! changes the meaning of.

use std::error::Error;
use std::path::Path;

/// Builds the text the box shows. A plain function so it can be tested
/// without a `MessageBoxW` call: given the error `run()` returned and the
/// path it tried to open, it names the file, the folder it lives in, and
/// the whole chain of causes, not only the outermost `Display`.
///
/// `db_path` is `None` only when the failure happened before a path was
/// even chosen (`dirs::data_dir()` returning nothing, `db_path()` in
/// lib.rs); every other failure this app can have while starting knows the
/// path by the time it reaches here, because `run()` computes it first.
pub fn message(err: &dyn Error, db_path: Option<&Path>) -> String {
    let mut out = String::new();
    out.push_str("Dinar n'a pas pu démarrer.\n");
    out.push_str("Dinar could not start.\n");
    out.push_str("لم يتمكن دينار من بدء التشغيل.\n");

    if let Some(path) = db_path {
        let file = path.display();
        out.push('\n');
        out.push_str(&format!("Fichier / File / الملف: {file}\n"));
        let folder = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "?".to_owned());
        out.push_str(&format!("Dossier / Folder / المجلد: {folder}\n"));
    }

    out.push('\n');
    out.push_str(&error_chain(err));
    out
}

/// The error and every one of its sources, one per line. `CoreError::Io`'s
/// own `Display` is fixed on purpose ("the shop's files could not complete
/// the operation") because it can cross the wire to a caller who must
/// never learn a server path (`crates/core/src/error.rs`); the detail that
/// makes this box worth reading is the source chain underneath it, where
/// SQLite and diesel say what actually went wrong.
fn error_chain(err: &dyn Error) -> String {
    let mut lines = vec![err.to_string()];
    let mut source = err.source();
    while let Some(e) = source {
        lines.push(e.to_string());
        source = e.source();
    }
    lines.join("\n")
}

/// Shows the box and returns. Best effort: a call that fails (no display
/// attached, running as a service) is not a reason to hang the exit path
/// that is already unwinding, so the result is dropped rather than
/// propagated.
#[cfg(windows)]
pub fn show(err: &dyn Error, db_path: Option<&Path>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let text = to_wide(&message(err, db_path));
    let caption = to_wide("Dinar");
    // Safety: both buffers are null-terminated UTF-16 and live until the
    // call returns; a null HWND means the box has no owner window, which
    // is the only option here since the app's own window never opened.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
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
}
