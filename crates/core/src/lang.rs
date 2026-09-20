//! The language a document is printed in.
//!
//! It started in `money::words`, where the amount in letters needed it. A
//! printed template needs it too, and the print module is not part of the
//! money module, so it lives here and `money::words` re-exports it: one
//! enum, no conversion between two spellings of the same three languages.

use serde::{Deserialize, Serialize};

/// The print language of a document. The same three the screen has
/// (`apps/desktop/src/i18n`), because the ruling in `docs/features.md` §4
/// is that a document prints in the language the till is being used in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Fr,
    En,
    Ar,
}

impl Lang {
    /// The three, in the order the goldens and the dictionary test walk
    /// them. A new language added here fails every "all three" test until
    /// it has its strings, which is the point.
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Ar];

    /// The BCP 47 tag for the `lang` attribute of a printed page.
    pub const fn tag(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
            Lang::Ar => "ar",
        }
    }

    /// Arabic is written right to left; the other two are not. The value
    /// goes straight into the `dir` attribute of the printed page.
    pub const fn dir(self) -> &'static str {
        match self {
            Lang::Ar => "rtl",
            Lang::Fr | Lang::En => "ltr",
        }
    }

    /// The reverse of `tag`. A spelling this build does not recognise reads
    /// as `None` rather than as an error, the way `services::preferences`'s
    /// own `Theme::parse` reads an unknown theme: a stored or requested
    /// language this build has never heard of should fall back to the
    /// caller's own language, not refuse to print.
    pub fn parse(value: &str) -> Option<Lang> {
        match value {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            "ar" => Some(Lang::Ar),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Lang;

    #[test]
    fn only_arabic_is_right_to_left() {
        assert_eq!(Lang::Ar.dir(), "rtl");
        assert_eq!(Lang::Fr.dir(), "ltr");
        assert_eq!(Lang::En.dir(), "ltr");
        assert_eq!(Lang::ALL.len(), 3);
    }

    #[test]
    fn a_tag_is_the_lowercase_code_the_wire_uses() {
        for lang in Lang::ALL {
            let json = serde_json::to_string(&lang).unwrap_or_default();
            assert_eq!(json, format!("\"{}\"", lang.tag()));
        }
    }

    #[test]
    fn parse_is_the_reverse_of_tag_for_every_language() {
        for lang in Lang::ALL {
            assert_eq!(Lang::parse(lang.tag()), Some(lang));
        }
    }

    #[test]
    fn a_spelling_this_build_does_not_know_reads_as_no_language() {
        assert_eq!(Lang::parse("de"), None);
        assert_eq!(Lang::parse("FR"), None);
        assert_eq!(Lang::parse(""), None);
    }
}
