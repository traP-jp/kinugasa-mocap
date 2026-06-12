use crate::rist::Profile;

#[test]
fn parses_rist_profiles() {
    assert!(matches!(Profile::parse("simple"), Some(Profile::Simple)));
    assert!(matches!(Profile::parse("main"), Some(Profile::Main)));
    assert!(matches!(
        Profile::parse("advanced"),
        Some(Profile::Advanced)
    ));
    assert!(Profile::parse("unknown").is_none());
}
