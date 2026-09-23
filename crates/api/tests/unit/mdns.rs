#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

#[test]
fn an_instance_name_is_stable_per_shop() {
    assert_eq!(instance_name(1), "Dinar-1");
    assert_eq!(host_name(1), "dinar-1.local.");
    assert_eq!(service_type(), "_dzpos._tcp.local.");
}

#[test]
fn a_registration_can_be_created_and_unregistered() {
    // No network assertion: just that the daemon starts, registers, and
    // can be dropped without panic. Browsing would need a second daemon
    // and a sleep, flaky in CI, so keep this unit.
    let daemon = ServiceDaemon::new().expect("daemon");
    let info = ServiceInfo::new(
        SERVICE_TYPE,
        "Dinar-9-test",
        "dinar-9-test.local.",
        (),
        4317,
        HashMap::from([("shop".to_string(), "9".to_string())]),
    )
    .unwrap();
    daemon.register(info).unwrap();
    // Unregister by dropping daemon; explicit unregister is also fine.
    drop(daemon);
}
