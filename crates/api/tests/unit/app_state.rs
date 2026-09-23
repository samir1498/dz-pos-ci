//! `lib.rs`'s own tests: the origin flag parser, the log path convention,
//! sidecar cleanup and the restore-rename recovery. Declared there with
//! `#[path]` rather than inline: `scripts/file-sizes.mjs` caps that file at
//! 600 lines, and each module below is still a child of it (`super::`
//! resolves its private items) so nothing about what it can reach changes.
//! None of the four names a retail type; the fixtures a restore or a backup
//! touches are kernel concepts regardless of the feature (S5 of
//! `a-kernel-crate-and-retail-as-the-first-module`).

mod origin_tests {
    use super::super::origin_from_flag;

    #[test]
    fn a_plain_origin_is_accepted() {
        for ok in [
            "http://100.111.55.62:5173",
            "http://localhost:5174",
            "https://till.example",
            "tauri://localhost",
        ] {
            assert!(origin_from_flag(ok).is_ok(), "{ok} was refused");
        }
    }

    #[test]
    fn a_wildcard_null_slash_path_or_space_is_refused_before_binding() {
        for bad in [
            "*",
            "null",
            "http://x:5173/",
            "http://a b",
            "http://x:5173/products",
            "ftp://x",
            "localhost:5173",
        ] {
            assert!(origin_from_flag(bad).is_err(), "{bad} was accepted");
        }
    }
}

mod log_path_tests {
    use super::super::default_log_path;
    use std::path::Path;

    #[test]
    fn the_log_sits_beside_the_shop_file() {
        assert_eq!(
            default_log_path(Path::new("/data/dzpos/shop.db")),
            Path::new("/data/dzpos/dzpos.log")
        );
    }

    #[test]
    fn a_shop_file_with_no_parent_keeps_the_log_in_the_working_directory() {
        assert_eq!(
            default_log_path(Path::new("shop.db")),
            Path::new("dzpos.log")
        );
    }
}

mod sidecar_tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::super::{clear_sidecars, sibling};

    #[test]
    fn the_sidecars_of_the_file_that_was_replaced_go_and_the_file_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        std::fs::write(sibling(&db, "-wal"), b"wal").unwrap();
        std::fs::write(sibling(&db, "-shm"), b"shm").unwrap();

        clear_sidecars(&db).unwrap();
        assert!(!sibling(&db, "-wal").exists());
        assert!(!sibling(&db, "-shm").exists());
        assert_eq!(std::fs::read(&db).unwrap(), b"the restored copy");
    }

    #[test]
    fn a_folder_with_no_sidecars_in_it_is_not_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        clear_sidecars(&db).unwrap();
    }

    /// The name comes back because the caller has to log it: it is the file
    /// someone will have to remove before the app will open the shop again.
    #[cfg(unix)]
    #[test]
    fn one_that_will_not_go_is_named_back_to_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        std::fs::write(&db, b"the restored copy").unwrap();
        let shm = sibling(&db, "-shm");
        std::fs::create_dir(&shm).unwrap();
        std::fs::write(shm.join("in the way"), b"x").unwrap();

        let (named, _) = clear_sidecars(&db).unwrap_err();
        assert_eq!(named, shm);
    }
}

mod reopen_original_tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::super::{reopen_original, ApiError};
    use dzpos_core::error::CoreError;

    fn rename_failed() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "rename refused")
    }

    /// The ordinary half: the shop file is still there and still opens, so
    /// the till goes on working and the only thing that went wrong is the
    /// one the caller asked about.
    #[test]
    fn a_shop_file_that_opens_again_leaves_the_till_working_and_names_the_rename() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("t.db");
        drop(dzpos_core::db::open(&db).unwrap());

        let mut slot = None;
        let err = reopen_original(&mut slot, &db, rename_failed());

        assert!(slot.is_some(), "the shop file was not put back in the slot");
        assert!(
            matches!(err, ApiError::Core(CoreError::Io(_))),
            "the rename is what failed and the answer should say so"
        );
    }

    /// Both halves gone: the rename did not happen and the file it would
    /// have replaced cannot be opened either. Answering with the rename
    /// alone would send a screen looking for a disk problem while every
    /// other route 500s on an empty slot, so the answer carries both facts
    /// and the one instruction that helps.
    #[test]
    fn a_shop_file_that_will_not_open_either_says_both_and_asks_for_a_relaunch() {
        let dir = tempfile::tempdir().unwrap();
        // A folder standing where the shop file was: SQLite will not open it
        // on any system, as any user.
        let db = dir.path().join("t.db");
        std::fs::create_dir(&db).unwrap();

        let mut slot = None;
        let err = reopen_original(&mut slot, &db, rename_failed());

        assert!(
            slot.is_none(),
            "a slot filled here would answer queries from a file nobody opened"
        );
        assert!(
            matches!(err, ApiError::NotRestoredRestartNeeded),
            "the caller was told about the rename only: {err}"
        );
    }
}
