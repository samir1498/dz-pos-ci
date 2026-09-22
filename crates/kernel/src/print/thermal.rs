//! Which of the two ESC/POS paths a shop's thermal head is sent.
//!
//! There are two ways to put a ticket on a cheap 80 mm head and they are not
//! interchangeable. Text mode selects a single-byte table (`ESC t 19`) and
//! sends one byte per column, which is fast on any link and is what French
//! and English have always used. Raster mode draws the same lines into a
//! 1-bit bitmap and sends `GS v 0` bands, which needs no table at all
//! (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`).
//!
//! The shop chooses between them for the languages where there is a choice.
//! For Arabic there is none: no single-byte table a cheap head ships with
//! has Arabic in it, letters change shape with their neighbours, and a
//! text-mode head neither joins them nor runs right to left, so it prints a
//! box per byte. `for_lang` is where that is decided, once, so no handler
//! gets to have its own opinion about it.
//!
//! Not in `print::layout` beside `FactureLayout`: a layout is how a page is
//! drawn and this is which wire the bytes go down. They are stored the same
//! way, as one preference row each, which is the only thing they share.

use crate::lang::Lang;

/// The two ESC/POS paths. `Text` is the default, and it is the default for
/// a reason a shop feels: a 576-dot raster of a forty-line ticket is about
/// 86 KB, which is nothing over USB or a LAN and a few seconds over a slow
/// serial link, where text mode was a few hundred bytes. A shop that has
/// never opened the printing panel keeps the faster path for the two
/// languages that can use it.
///
/// A third mode is one arm here and one row in the panel. The preference
/// table carries no CHECK on the value, so adding one is not a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThermalMode {
    /// One byte per column down `ESC t 19`, the wire the nine `.txt`
    /// goldens pin.
    #[default]
    Text,
    /// The lines drawn into dots and sent as `GS v 0` bands, the wire the
    /// Arabic `.png` goldens pin.
    Raster,
}

impl ThermalMode {
    /// Both, in the order the panel lists them and the tests walk them.
    pub const ALL: [ThermalMode; 2] = [ThermalMode::Text, ThermalMode::Raster];

    /// The name the preference row stores, which is also the name the wire
    /// and the spool file carry. One spelling, so a value written by an
    /// older build reads back the same and a spool file names the path its
    /// bytes actually took.
    pub const fn as_str(self) -> &'static str {
        match self {
            ThermalMode::Text => "text",
            ThermalMode::Raster => "raster",
        }
    }

    /// The reverse. A stored name this build does not know reads as `None`
    /// and the caller falls back to the default, the way an unknown facture
    /// layout prints on the standard one: a row a newer build wrote should
    /// leave a shop printing, not refusing.
    pub fn parse(value: &str) -> Option<ThermalMode> {
        match value {
            "text" => Some(ThermalMode::Text),
            "raster" => Some(ThermalMode::Raster),
            _ => None,
        }
    }

    /// The mode a document in `lang` is actually sent in, which is the
    /// shop's choice everywhere except Arabic.
    ///
    /// Arabic always rasters, whatever the shop stored. This is not a
    /// preference being overridden for the sake of it: there is no
    /// single-byte table with Arabic in it, so the text path for an Arabic
    /// ticket is a row of boxes, and offering a shop the choice of boxes is
    /// offering it nothing. The panel says so in all three languages rather
    /// than leaving an owner to find it on paper.
    ///
    /// It lives here and not in the route because two routes, the spool
    /// writer and the TCP sender all have to agree: a spool file holding
    /// raster bytes while the wire got text is the bug this function exists
    /// to make impossible.
    pub const fn for_lang(self, lang: Lang) -> ThermalMode {
        match lang {
            Lang::Ar => ThermalMode::Raster,
            Lang::Fr | Lang::En => self,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
