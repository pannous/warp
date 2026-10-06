//! Uniscript (wiki/uniscript.md) in wasp: the uniscript package (packages.wasp): its uniscript.wasp over its data/entities.idx

use warp::diagnostic::take_runtime_warnings;
use warp::node::Node;
use warp::wasm_emitter::eval;
use warp::error;
use crate::is;

/// `use uniscript` makes compiling a program take most of a test's time: one program converts all `cases`
/// (uniscript, unicode), each checked on its own
fn converts(cases: &[(&str, &str)]) {
	each_gives("uniscript", cases);
}

/// The same for unicode_to_uniscript: `cases` are (unicode, uniscript)
fn spells(cases: &[(&str, &str)]) {
	each_gives("unicode_to_uniscript", cases);
}

/// `function(input)` of every (input, expected) case in one program: [function("a"), function("b"), …]
fn each_gives(function: &str, cases: &[(&str, &str)]) {
	let calls: Vec<String> = cases.iter().map(|(input, _)| format!("{function}(\"{input}\")")).collect();
	let results = eval(&format!("use uniscript; [{}]", calls.join(", ")));
	let Node::List(results, _, _) = results.drop_meta() else { panic!("{function} of {cases:?} gave {results:?}") };
	assert_eq!(results.len(), cases.len(), "{results:?}");
	for ((input, expected), result) in cases.iter().zip(results) {
		assert_eq!(result, expected, "{function}(\"{input}\")");
	}
}

#[test]
fn entities_become_characters() {
	converts(&[
		("<:alpha>", "α"),
		("\\\\:infinity", "∞"),
		("<:greek small letter alpha>", "α"),
		("<:dopf>", "𝕕"), // HTML name, backwards compatible
		("<:alpha> > <:beta>", "α > β"),
		("<:forall> x <:in> <:double R>", "∀ x ∈ ℝ"),
	]);
}

#[test]
fn block_types_style_their_operands() {
	converts(&[
		("<:fracture A>", "𝔄"),
		("<:fracture A b c >", "𝔄𝔟𝔠"),
		("<:fracture> A b c <:>", " 𝔄 𝔟 𝔠 "),
		("<:greek> a b g d <:/greek>", " α β γ δ "),
		("<:double d>", "𝕕"),
		("<:double-d>", "𝕕"),
		("x<:upper a>", "xᵃ"),
		("<:ligature ae>", "æ"),
		("<:reverseInPlace e>", "ɘ"),
		("<:iconic ⚠>", "⚠\u{FE0F}"),
	]);
}

#[test]
fn greek_is_transliterated_phonetically() {
	converts(&[
		("<:greek> athos <:/greek>", " αθοσ "), // th is one letter
		("<:greek th ch ps>", "θχψ"),
		("<:greek eta Omega lambda>", "ηΩλ"),
	]);
}

/// Full block tags keep their text as written, spaces and line breaks included; inline tags drop the spaces between operands
#[test]
fn full_blocks_keep_their_spaces() {
	converts(&[
		("<:greek> filosofia kosmos<:/greek>", " φιλοσοφια κοσμοσ"),
		("<:greek a kosmos>", "ακοσμοσ"),
		("<:greek phi chi>", "φχ"),
		("<:greek>\\nkosmos\\n<:/greek>", "\nκοσμοσ\n"),
	]);
}

/// A character or combination without a Unicode counterpart stays plain, with a warning naming it and its position
fn warns(uniscript: &str, unicode: &str, warning: &str) {
	take_runtime_warnings();
	converts(&[(uniscript, unicode)]);
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
	converts(&[
		("<:red circle>", "🔴"),
		("<:brown heart>", "🤎"),
		("<:red A>", "A\u{E0072}"),
		("<:mirror e>", "e\u{E004D}"),
		("<:mirror 𓀀>", "𓀀\u{13440}"),
	]);
}

#[test]
fn effect_words_stack_on_one_operand() {
	// fonts/README.md: one geometry and one color combine in either order
	converts(&[
		("<:mirror red A>", "A\u{E0072}\u{E004D}"),
		("<:red mirror A>", "A\u{E004D}\u{E0072}"),
		("<:reverse red R>", "R\u{E0072}\u{E004D}"), // reverse is mirror
		("<:mirror red A b>", "A\u{E0072}\u{E004D}b\u{E0072}\u{E004D}"),
		("<:mirror red circle>", "🔴\u{E004D}"),
	]);
	spells(&[("A\u{E0072}\u{E004D} 🔴\u{E004D}", "<:mirror red A> <:mirror red circle>")]);
}

#[test]
fn groups_join_hieroglyphs_and_compose_ideographs() {
	converts(&[
		("<:above 𓀀 𓁐>", "𓀀\u{13430}𓁐"),
		("<:beside 犭 句>", "⿰犭句"),
	]);
}

#[test]
fn the_marker_is_escaped_by_single_character_entities() {
	converts(&[
		("<:<> <::> <<::>", "< : <:"),
		("<:less>:", "<:"),
	]);
}

/// `<:uniscript version="…">` at the start of a file declares it uniscript; the header and its line break convert to nothing
#[test]
fn the_header_declares_uniscript_and_its_version() {
	let header = "<:uniscript version=\\\"https://uniscript.org/v1\\\">";
	converts(&[
		(&format!("{header}\\n<:alpha>"), "α"),
		(&format!("{header}\\r\\n<:alpha>"), "α"),
		(&format!("{header} <:alpha>"), " α"),
		("<:uniscript><:alpha>", "α"),
	]);
	warns("<:uniscript version=\\\"https://uniscript.org/v9\\\">A", "A", "uniscript: unsupported uniscript version https://uniscript.org/v9 at byte 0");
	is!(&format!("use uniscript; uniscript(\"x {header}\")"), error("unknown uniscript entity: uniscript version=\"https://uniscript.org/v1\""));
}

#[test]
fn an_unknown_entity_is_an_error() {
	is!("use uniscript; uniscript(\"<:nosuchthing> x\")", error("unknown uniscript entity: nosuchthing"));
}

#[test]
fn unicode_spells_back_as_uniscript() {
	spells(&[
		("α Ω 𝔄 ∞ ℝ", "<:alpha> <:Omega> <:fracture A> <:infinity> <:double R>"),
		("A\u{E0072} 🔴 xᵃ", "<:red A> <:red circle> x<:upper a>"),
		("a <: b", "a <<::> b"),
	]);
}

#[test]
fn spelling_back_round_trips() {
	let text = "∀x∈ℝ: 𝔄 A\u{E0072} 𓀀\u{13440} <: é";
	is!(&format!("use uniscript; t=\"{text}\"; uniscript(unicode_to_uniscript(t)) == t"), true);
}

/// The binary index and the readable entity file agree entry by entry: the package's own checker, its prebuilt
/// uniscript.wasm (never `cargo run` in the package: that built it into the shared cargo target directory)
#[test]
fn the_index_matches_the_readable_entities() {
	let check = warp::package_tools::run_package_tool("uniscript", &["check"]).unwrap();
	assert!(check.success(), "{}{}", check.stdout, check.stderr);
}
