use super::{admit, Refusal, Store};
use std::path::{Path, PathBuf};

/// The repository root, from this crate's own directory.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

#[test]
fn nothing_the_shop_installs_can_build_the_seeder() {
    // A `src/bin/` inside the API is a binary the API's own build
    // produces, which is how the seeder shipped for about an hour before
    // this test existed. It also breaks `cargo run -p dzpos-api`, which
    // is what `just api` and the playwright harness both run: cargo
    // cannot pick between two binaries.
    assert!(
        !root().join("crates/api/src/bin").exists(),
        "crates/api/src/bin exists: a second binary there ships with the API and breaks `cargo run -p dzpos-api`"
    );
    // And neither of the two packages a shop installs may name this one.
    // Not `dzpos-core`: the core depending on the seeder would be a
    // cycle, which cargo refuses on its own, and a check that restates
    // what the compiler already enforces is a check that teaches nothing.
    for manifest in ["crates/api/Cargo.toml", "apps/desktop/src-tauri/Cargo.toml"] {
        let text = std::fs::read_to_string(root().join(manifest)).unwrap();
        assert!(
            !text.contains("dzpos-seed"),
            "{manifest} depends on the seeder, which puts it in what the shop installs"
        );
    }
    // The bundle carries no sidecar binary at all: `externalBin` is the
    // Tauri key that copies one in beside the app.
    let tauri =
        std::fs::read_to_string(root().join("apps/desktop/src-tauri/tauri.conf.json")).unwrap();
    assert!(
        !tauri.contains("externalBin"),
        "the desktop bundle carries a sidecar binary; check it is not the seeder"
    );
}

#[test]
fn the_api_source_reaches_the_seeder_nowhere() {
    // Not a search for the word: the API has had identifiers with `seed`
    // in them that have nothing to do with this crate, and a check that
    // greps for a word is a check people learn to work around. What is
    // looked for is a way in: the crate by name, the service by path, and
    // a route or a flag spelled `seed`.
    for file in ["crates/api/src/main.rs", "crates/api/src/lib.rs"] {
        let text = std::fs::read_to_string(root().join(file)).unwrap();
        for forbidden in [
            "dzpos_seed",
            "dzpos-seed",
            "services::seed",
            "\"seed\"",
            "/seed",
        ] {
            assert!(
                !text.contains(forbidden),
                "{file} contains {forbidden}: the API has no seed flag, no seed route and no path to the seeder"
            );
        }
    }
}

fn dev_file() -> &'static Path {
    Path::new(".dev/dev.db")
}

#[test]
fn a_box_that_did_not_say_it_is_for_development_is_refused_whatever_the_path_says() {
    assert_eq!(
        admit(dev_file(), None, false, None),
        Err(Refusal::NotADevBox)
    );
    assert_eq!(
        admit(dev_file(), Some("0"), false, None),
        Err(Refusal::NotADevBox)
    );
    assert_eq!(
        admit(dev_file(), Some("yes"), false, None),
        Err(Refusal::NotADevBox)
    );
    // And --force does not buy it: a shop's PC is not a development box
    // because somebody passed a flag.
    assert_eq!(
        admit(dev_file(), None, true, None),
        Err(Refusal::NotADevBox)
    );
}

#[test]
fn a_file_outside_the_dev_directory_is_refused_and_force_does_not_lift_it() {
    for path in [
        "shop.db",
        "/home/samir/shop.db",
        "backups/.dev.db",
        ".dev/nested/dev.db",
        "/var/lib/dzpos/.development/dev.db",
    ] {
        assert_eq!(
            admit(Path::new(path), Some("1"), true, None),
            Err(Refusal::NotUnderDevDir),
            "{path}"
        );
    }
    // Relative and absolute alike, as long as the file sits directly in a
    // directory called `.dev`.
    assert!(admit(dev_file(), Some("1"), false, None).is_ok());
    assert!(admit(
        Path::new("/home/samir/dz-pos/.dev/other.db"),
        Some("1"),
        false,
        None
    )
    .is_ok());
}

#[test]
fn a_file_carrying_a_real_shops_identifiers_is_refused_until_somebody_says_otherwise() {
    let real = Some(Store {
        name: "Alimentation Générale Boukhalfa",
        has_identifiers: true,
    });
    assert_eq!(
        admit(dev_file(), Some("1"), false, real),
        Err(Refusal::LooksLikeARealShop)
    );
    // This is the one thing --force is for.
    assert!(admit(dev_file(), Some("1"), true, real).is_ok());
}

#[test]
fn the_seeders_own_file_and_a_fresh_one_are_both_let_through() {
    // What the seeder wrote last time: identifiers, but its own invented
    // shop, so re-seeding it needs no flag.
    assert!(admit(
        dev_file(),
        Some("1"),
        false,
        Some(Store {
            name: dzpos_core::services::seed::SHOP_NAME,
            has_identifiers: true,
        })
    )
    .is_ok());
    // And a file the first migration made and nobody filled in.
    assert!(admit(
        dev_file(),
        Some("1"),
        false,
        Some(Store {
            name: "Mon magasin",
            has_identifiers: false,
        })
    )
    .is_ok());
}
