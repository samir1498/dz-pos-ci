//! C2 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`:
//! the crate holds nothing clinic-specific yet, so the one test that belongs
//! here is that the crate itself is real and linked, not any behaviour.

#[test]
fn the_crate_name_matches_its_package() {
    assert_eq!(dzpos_clinic::CRATE_NAME, "dzpos-clinic");
}
