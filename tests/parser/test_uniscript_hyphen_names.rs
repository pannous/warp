// card uniscript-literal-hyphen: `\:fracture-A` spells the index's `fracture A` as a whole, in texts and code alike;
// a hyphen after a name that makes no longer known name still ends it (`\:alpha-x` is α-x)
use crate::is;

#[test]
fn a_style_letter_is_one_entity_in_a_text() {
	is!("\"\\:fracture-A\"", "𝔄");
	is!("\"\\:bold-fracture-A \\:double-struck-R\"", "𝕬 ℝ");
}

#[test]
fn a_hyphen_after_a_whole_name_stays_a_hyphen() {
	is!("\"\\:alpha-x\"", "α-x");
	is!("\"\\:fracture-A-x\"", "𝔄-x");
}
