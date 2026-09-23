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
