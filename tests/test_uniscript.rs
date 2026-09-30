//! Uniscript (wiki/uniscript.md) in wasp: lib/uniscript.wasp over the index of the uniscript package (packages.wasp)

use std::process::Command;
use warp::diagnostic::take_runtime_warnings;
use warp::{error, is};

fn converts(uniscript: &str, unicode: &str) {
	is!(&format!("use uniscript; uniscript(\"{uniscript}\")"), unicode);
}

fn spells(unicode: &str, uniscript: &str) {
	is!(&format!("use uniscript; unicode_to_uniscript(\"{unicode}\")"), uniscript);
}

#[test]
fn entities_become_characters() {
	converts("<:alpha>", "α");
	converts("\\\\:infinity", "∞");
	converts("<:greek small letter alpha>", "α");
	converts("<:dopf>", "𝕕"); // HTML name, backwards compatible
	converts("<:alpha> > <:beta>", "α > β");
	converts("<:forall> x <:in> <:double R>", "∀ x ∈ ℝ");
}

#[test]
fn block_types_style_their_operands() {
	converts("<:fracture A>", "𝔄");
	converts("<:fracture A b c >", "𝔄𝔟𝔠");
	converts("<:fracture> A b c <:>", " 𝔄 𝔟 𝔠 ");
	converts("<:greek> a b g d <:/greek>", " α β γ δ ");
	converts("<:double d>", "𝕕");
	converts("<:double-d>", "𝕕");
	converts("x<:upper a>", "xᵃ");
	converts("<:ligature ae>", "æ");
	converts("<:reverseInPlace e>", "ɘ");
	converts("<:iconic ⚠>", "⚠\u{FE0F}");
}

#[test]
fn greek_is_transliterated_phonetically() {
	converts("<:greek> athos <:/greek>", " αθοσ "); // th is one letter
	converts("<:greek th ch ps>", "θχψ");
	converts("<:greek eta Omega lambda>", "ηΩλ");
}

/// Full block tags keep their text as written, spaces and line breaks included; inline tags drop the spaces between operands
#[test]
fn full_blocks_keep_their_spaces() {
	converts("<:greek> filosofia kosmos<:/greek>", " φιλοσοφια κοσμοσ");
	converts("<:greek a kosmos>", "ακοσμοσ");
	converts("<:greek phi chi>", "φχ");
	converts("<:greek>\\nkosmos\\n<:/greek>", "\nκοσμοσ\n");
}

/// A character or combination without a Unicode counterpart stays plain, with a warning naming it and its position
fn warns(uniscript: &str, unicode: &str, warning: &str) {
	take_runtime_warnings();
	converts(uniscript, unicode);
	assert_eq!(take_runtime_warnings(), vec![warning.to_string()], "{uniscript}");
}

#[test]
fn unsupported_characters_and_combinations_warn() {
	warns("<:greek c>", "c", "uniscript: no greek form of c at byte 0");
	warns("x <:fracture 7>", "x 7", "uniscript: no fracture form of 7 at byte 2");
	warns("<:red 𓀀>", "𓀀", "uniscript: red does not apply to 𓀀 at byte 0");
	warns("<:mirror red 狗>", "狗\u{E004D}", "uniscript: red does not apply to 狗 at byte 0");
	warns("<:beside a b>", "ab", "uniscript: no beside group of a at byte 0");
}

#[test]
fn use_strict_makes_uniscript_warnings_errors() {
	is!("use strict; use uniscript; uniscript(\"<:greek c>\")", error("uniscript: no greek form of c at byte 0"));
	is!("use strict; use uniscript; uniscript(\"<:greek a>\")", "α");
}

#[test]
fn colors_and_geometry_are_suffix_controls() {
	converts("<:red circle>", "🔴");
	converts("<:brown heart>", "🤎");
	converts("<:red A>", "A\u{E0072}");
	converts("<:mirror e>", "e\u{E004D}");
	converts("<:mirror 𓀀>", "𓀀\u{13440}");
}

#[test]
fn effect_words_stack_on_one_operand() {
	// fonts/README.md: one geometry and one color combine in either order
	converts("<:mirror red A>", "A\u{E0072}\u{E004D}");
	converts("<:red mirror A>", "A\u{E004D}\u{E0072}");
	converts("<:reverse red R>", "R\u{E0072}\u{E004D}"); // reverse is mirror
	converts("<:mirror red A b>", "A\u{E0072}\u{E004D}b\u{E0072}\u{E004D}");
	converts("<:mirror red circle>", "🔴\u{E004D}");
	spells("A\u{E0072}\u{E004D} 🔴\u{E004D}", "<:mirror red A> <:mirror red circle>");
}

#[test]
fn groups_join_hieroglyphs_and_compose_ideographs() {
	converts("<:above 𓀀 𓁐>", "𓀀\u{13430}𓁐");
	converts("<:beside 犭 句>", "⿰犭句");
}

#[test]
fn the_marker_is_escaped_by_single_character_entities() {
	converts("<:<> <::> <<::>", "< : <:");
	converts("<:less>:", "<:");
}

/// `<:uniscript version="…">` at the start of a file declares it uniscript; the header and its line break convert to nothing
#[test]
fn the_header_declares_uniscript_and_its_version() {
	let header = "<:uniscript version=\\\"https://uniscript.org/v1\\\">";
	converts(&format!("{header}\\n<:alpha>"), "α");
	converts(&format!("{header}\\r\\n<:alpha>"), "α");
	converts(&format!("{header} <:alpha>"), " α");
	converts("<:uniscript><:alpha>", "α");
	warns("<:uniscript version=\\\"https://uniscript.org/v9\\\">A", "A", "uniscript: unsupported uniscript version https://uniscript.org/v9 at byte 0");
	is!(&format!("use uniscript; uniscript(\"x {header}\")"), error("unknown uniscript entity: uniscript version=\"https://uniscript.org/v1\""));
}

#[test]
fn an_unknown_entity_is_an_error() {
	is!("use uniscript; uniscript(\"<:nosuchthing> x\")", error("unknown uniscript entity: nosuchthing"));
}

#[test]
fn unicode_spells_back_as_uniscript() {
	spells("α Ω 𝔄 ∞ ℝ", "<:alpha> <:Omega> <:fracture A> <:infinity> <:double R>");
	spells("A\u{E0072} 🔴 xᵃ", "<:red A> <:red circle> x<:upper a>");
	spells("a <: b", "a <<::> b");
}

#[test]
fn spelling_back_round_trips() {
	let text = "∀x∈ℝ: 𝔄 A\u{E0072} 𓀀\u{13440} <: é";
	is!(&format!("use uniscript; t=\"{text}\"; uniscript(unicode_to_uniscript(t)) == t"), true);
}

/// The binary index and the readable entity file agree entry by entry
#[test]
fn the_index_matches_the_readable_entities() {
	let package = warp::modules::fetch_package("uniscript").unwrap();
	let check = Command::new("cargo").args(["run", "--quiet", "--release", "--", "check"]).current_dir(package).output().expect("cargo");
	assert!(check.status.success(), "{}{}", String::from_utf8_lossy(&check.stdout), String::from_utf8_lossy(&check.stderr));
}
