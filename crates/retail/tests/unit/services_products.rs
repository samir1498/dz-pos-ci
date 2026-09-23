use super::*;

#[test]
fn check_digit_matches_a_known_ean13() {
    // 978020137962-x, the ISBN example GS1 publishes; the digit is 4.
    assert_eq!(ean13_check_digit("978020137962").unwrap(), 4);
}

#[test]
fn a_twelve_digit_upc_is_filed_as_the_ean_it_is_short_for() {
    // The two numbers on one box. Filed apart, the UNIQUE index sees two
    // different strings and lets the same article in twice, and a scan
    // then resolves to whichever row sorts first by name.
    assert_eq!(canonical_barcode("613000900127"), "0613000900127");
    assert_eq!(
        canonical_barcode("613000900127"),
        canonical_barcode("0613000900127")
    );
}

#[test]
fn nothing_else_is_touched() {
    // An EAN-8 is eight digits and its own article, an in-store code is
    // already thirteen, and a twelve-character code with a letter in it
    // is not a UPC at all.
    assert_eq!(canonical_barcode("61300001"), "61300001");
    assert_eq!(canonical_barcode("2000010000073"), "2000010000073");
    assert_eq!(canonical_barcode("ABC123456789"), "ABC123456789");
}

#[test]
fn an_in_store_code_is_thirteen_digits_starting_at_two() {
    let code = in_store_barcode(1, 7).unwrap();
    assert_eq!(code.len(), 13);
    assert!(code.starts_with("200001000007"), "{code}");
}
