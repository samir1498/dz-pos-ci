use super::percent;
use dzpos_kernel::money::Bps;

#[test]
fn a_rate_is_a_percentage_with_the_decimals_it_needs() {
    let bps = |v: u32| Bps::new(v).unwrap_or(Bps::ZERO);
    assert_eq!(percent(bps(1900)), "19\u{202f}%");
    assert_eq!(percent(bps(900)), "9\u{202f}%");
    assert_eq!(percent(bps(0)), "0\u{202f}%");
    assert_eq!(percent(bps(950)), "9,5\u{202f}%");
    assert_eq!(percent(bps(1)), "0,01\u{202f}%");
    assert_eq!(percent(bps(10_000)), "100\u{202f}%");
}
