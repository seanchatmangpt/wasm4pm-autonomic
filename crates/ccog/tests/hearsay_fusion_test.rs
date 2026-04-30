use ccog::breeds::hearsay::fuse_posture;
use ccog::FieldContext;
use ccog::PackPosture;

#[test]
fn hearsay_empty_outcomes_returns_calm() {
    let f = FieldContext::new("smoke");
    assert_eq!(fuse_posture(&[], &f).unwrap(), PackPosture::Calm);
}
