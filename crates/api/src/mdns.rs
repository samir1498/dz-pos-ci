//! mDNS for LAN mode (M6 T1). One desktop serves, the phone finds it on the
//! same Wi-Fi without typing an address. The service is `_dzpos._tcp.local.`,
//! instance `Dinar-<shop_id>`,TXT `shop=<id>` + `v=<semver>` so a phone that
//! paired to shop 2 does not talk to shop 1 next door.

use std::collections::HashMap;

use mdns_sd::{ServiceDaemon, ServiceInfo};

const SERVICE_TYPE: &str = "_dzpos._tcp.local.";
const DOMAIN: &str = "local.";

fn instance_name(shop_id: i32) -> String {
    format!("Dinar-{shop_id}")
}

fn host_name(shop_id: i32) -> String {
    format!("dinar-{shop_id}.{DOMAIN}")
}

/// Register the desktop on mDNS and return the daemon that keeps it alive.
/// The caller must hold the `ServiceDaemon`; dropping it unregisters.
pub fn register(shop_id: i32, port: u16) -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;
    let instance = instance_name(shop_id);
    let host = host_name(shop_id);
    let mut props = HashMap::new();
    props.insert("shop".to_string(), shop_id.to_string());
    props.insert("v".to_string(), env!("CARGO_PKG_VERSION").to_string());
    let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, (), port, props)?;
    daemon.register(info)?;
    Ok(daemon)
}

/// The service type the phone browses.
pub fn service_type() -> &'static str {
    SERVICE_TYPE
}

#[cfg(test)]
mod tests {
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
}
