use super::*;

#[test]
fn a_shop_that_has_never_chosen_is_on_text() {
    assert_eq!(ThermalMode::default(), ThermalMode::Text);
}

#[test]
fn every_mode_survives_its_stored_spelling() {
    for mode in ThermalMode::ALL {
        assert_eq!(ThermalMode::parse(mode.as_str()), Some(mode));
    }
    assert_eq!(ThermalMode::parse("Raster"), None);
    assert_eq!(ThermalMode::parse("dots"), None);
}

/// The rule the whole plan turns on: Arabic is drawn whatever the shop
/// asked for, and the other two follow the shop.
#[test]
fn arabic_rasters_under_either_preference() {
    for stored in ThermalMode::ALL {
        assert_eq!(stored.for_lang(Lang::Ar), ThermalMode::Raster);
    }
}

#[test]
fn french_and_english_follow_the_shop() {
    for lang in [Lang::Fr, Lang::En] {
        assert_eq!(ThermalMode::Text.for_lang(lang), ThermalMode::Text);
        assert_eq!(ThermalMode::Raster.for_lang(lang), ThermalMode::Raster);
    }
}
