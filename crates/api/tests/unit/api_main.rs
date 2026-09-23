#![allow(clippy::unwrap_used)]
use super::*;

// The walkthrough switch: loopback unless asked, so a plain `just api`
// never leaves the machine by accident.
#[test]
fn lan_mode_is_opt_in() {
    let plain = Args::try_parse_from(["dzpos-api", "--db", "x.db"]).unwrap();
    assert!(!plain.lan);
    let lan = Args::try_parse_from(["dzpos-api", "--db", "x.db", "--lan"]).unwrap();
    assert!(lan.lan);
}
