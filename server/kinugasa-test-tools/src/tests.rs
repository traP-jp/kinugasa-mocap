use std::collections::BTreeMap;

use crate::e2e::{CompareMode, expand_template};

#[test]
fn expands_known_placeholders() {
    let mut placeholders = BTreeMap::new();
    placeholders.insert("host", "127.0.0.1".to_owned());
    placeholders.insert("port", "1234".to_owned());

    assert_eq!(
        expand_template("udp://{host}:{port}?pkt_size=1316", &placeholders),
        "udp://127.0.0.1:1234?pkt_size=1316"
    );
}

#[test]
fn parses_compare_modes() {
    assert_eq!(CompareMode::parse("both").unwrap(), CompareMode::Both);
    assert_eq!(CompareMode::parse("video").unwrap(), CompareMode::Video);
    assert_eq!(CompareMode::parse("audio").unwrap(), CompareMode::Audio);
    assert_eq!(CompareMode::parse("none").unwrap(), CompareMode::None);
    assert!(CompareMode::parse("metadata").is_err());
}
