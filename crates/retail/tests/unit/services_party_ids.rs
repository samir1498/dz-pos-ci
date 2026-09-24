use super::*;
use crate::error::CoreError;

fn ready_seller() -> SellerBlock {
    SellerBlock {
        name: "Superette El Baraka".to_owned(),
        rc: Some("16/00-1234567 B 25".to_owned()),
        nif: Some("000216001234567".to_owned()),
        nis: Some("000116001234567".to_owned()),
        ai: Some("16012345678".to_owned()),
        address: Some("12 rue Didouche Mourad, Alger".to_owned()),
        phone: Some("0555 12 34 56".to_owned()),
    }
}

fn ready_company() -> PartyBlock {
    PartyBlock {
        name: "Entreprise Amrani".to_owned(),
        party_kind: PartyKind::Company,
        rc: Some("16/00-7654321 B 22".to_owned()),
        nif: None,
        nis: Some("098216007654321".to_owned()),
        ai: None,
        address: None,
    }
}

/// The behaviour this file exists for (T37): a seller short of both its
/// identifiers and a buyer short of one used to be two refusals, the
/// seller's on the first attempt and the buyer's only once the seller was
/// fixed. One refusal now names both.
#[test]
fn a_short_seller_and_a_short_buyer_are_named_together() {
    let mut seller = ready_seller();
    seller.rc = None;
    seller.nis = None;
    let mut buyer = ready_company();
    buyer.nis = None;

    let err = check(&seller, Some(&buyer)).unwrap_err();
    match err {
        RetailError::PartyIds {
            seller_missing,
            buyer_missing,
        } => {
            assert_eq!(seller_missing, vec!["rc", "nis"]);
            assert_eq!(buyer_missing, vec!["nis"]);
        }
        other => panic!("{other:?}"),
    }
}

/// A seller that is fine names an empty list rather than being left out of
/// the error, so the screen can tell "nothing wrong here" from "the server
/// did not check".
#[test]
fn a_ready_seller_reports_an_empty_list_beside_a_short_buyer() {
    let seller = ready_seller();
    let mut buyer = ready_company();
    buyer.rc = None;

    let err = check(&seller, Some(&buyer)).unwrap_err();
    match err {
        RetailError::PartyIds {
            seller_missing,
            buyer_missing,
        } => {
            assert!(seller_missing.is_empty());
            assert_eq!(buyer_missing, vec!["rc"]);
        }
        other => panic!("{other:?}"),
    }
}

/// The reverse: a short seller and a buyer with nothing missing.
#[test]
fn a_ready_buyer_reports_an_empty_list_beside_a_short_seller() {
    let mut seller = ready_seller();
    seller.rc = None;
    let buyer = ready_company();

    let err = check(&seller, Some(&buyer)).unwrap_err();
    match err {
        RetailError::PartyIds {
            seller_missing,
            buyer_missing,
        } => {
            assert_eq!(seller_missing, vec!["rc"]);
            assert!(buyer_missing.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

/// No customer at all is a different refusal (`customer_id`, not
/// `party_ids`) whatever state the seller is in, when the seller is ready:
/// there is no fiche to name identifiers on.
#[test]
fn no_customer_named_refuses_as_a_missing_customer_when_the_seller_is_ready() {
    let err = check(&ready_seller(), None).unwrap_err();
    match err {
        RetailError::Kernel(CoreError::Validation { field, .. }) => {
            assert_eq!(field, "customer_id");
        }
        other => panic!("{other:?}"),
    }
}

/// No customer named and a short seller: the seller's own list still
/// reaches the till, because that is the one thing a cashier could act on
/// before a customer is even picked.
#[test]
fn no_customer_named_and_a_short_seller_names_the_seller() {
    let mut seller = ready_seller();
    seller.nis = None;
    let err = check(&seller, None).unwrap_err();
    match err {
        RetailError::PartyIds {
            seller_missing,
            buyer_missing,
        } => {
            assert_eq!(seller_missing, vec!["nis"]);
            assert!(buyer_missing.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

/// Everything in place refuses nothing.
#[test]
fn two_ready_blocks_refuse_nothing() {
    check(&ready_seller(), Some(&ready_company())).unwrap();
}
