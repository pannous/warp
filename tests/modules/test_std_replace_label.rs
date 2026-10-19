use crate::is;

#[test]
fn replace_accepts_with_as_its_replacement_label() {
	is!(r#""I like tea".replace "tea" with: "coffee""#, "I like coffee");
}
