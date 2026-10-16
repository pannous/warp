//! Decision operator-alias (card standalone-math-units): `operator × := *` spells an operator another way, keeping its
//! precedence, `prefix operator √ := sqrt` a prefix one; lib/math.warp's alias lines are the math spellings every
//! program reads, in place of the parser's Rust table
use crate::is;
use warp::operators::{math_aliases, spelled_operator, Op};
use warp::parse;

#[test]
fn the_math_spellings_come_from_lib_math() {
	assert!(math_aliases().iter().any(|alias| alias.spelling == "×" && alias.op == Op::Mul));
	assert_eq!(spelled_operator("≤"), Some(Op::Le));
	assert_eq!(spelled_operator("*"), Some(Op::Mul));
}

#[test]
fn a_math_spelling_keeps_its_operator_and_precedence() {
	is!("1 + 2 × 3", 7);
	is!("6 ÷ 3 − 1", 1);
	is!("2 ≤ 3 ∧ 4 ≠ 5", true);
	is!("√16 + ∛27", 7.0);
}

#[test]
fn a_program_declares_its_own_alias() {
	is!("operator ⊕ := +\n1 ⊕ 2 * 3", 7);
	is!("prefix operator ∂ := sqrt\n∂16", 4.0);
	is!("operator ⊕ := + // plus\n1 ⊕ 2", 3);
}

/// reviewer: an alias line in a text or a comment declares nothing
#[test]
fn an_alias_line_in_a_text_is_text() {
	is!("note = \"\noperator ⊕ := +\n\"\n// operator ⊗ := *\nsize(note)", 17);
	assert!(warp::operators::source_aliases("note = \"\noperator ⊕ := +\n\"").is_empty());
}

#[test]
fn lib_math_with_its_alias_lines_still_loads() {
	is!("use math; 2 × square 3", 18);
	is!("include math; 2 × square 3", 18);
}

/// card user-listed: the arrows that spell ->, =>, <-, ↑ and ↓, the rest of the listed arrows stay unread
#[test]
fn arrow_glyphs_spell_the_arrow_operators() {
	is!("f = x → x*2; f(3)", 6);
	is!("f = x ⇒ x*2; f(3)", 6);
	is!("x ← 3; x+1", 4);
	is!("2 ⬆ 3", 8);
	is!("8 ⬇ 2", 3);
	assert_eq!(spelled_operator("⟶"), Some(Op::Arrow));
	assert_eq!(spelled_operator("↯"), None);
}

/// decision unicode-symbols: a symbol character nothing defines parses as a free operator or symbol name, an undefined
/// operator says how to define it
#[test]
fn free_unicode_symbols_parse() {
	is!("infix operator ⇔(a: bool, b: bool) := a == b; (1 > 0) ⇔ (2 > 0)", true);
	is!("x = ↯; x", parse("↯"));
	let error = warp::wasm_emitter::eval("1 ⇔ 1").serialize();
	assert!(error.contains("operator ⇔ is not defined (define it with infix operator ⇔(a, b) := …)"), "{error}");
}

/// reviewer: a free glyph is infix only before an operand on its line, else a value; the error points at the glyph
#[test]
fn a_free_glyph_without_right_operand_is_a_value() {
	is!("[↯ ↯]", parse("[↯ ↯]"));
	is!("f(x) := x; f ↯", parse("↯"));
	let error = warp::wasm_emitter::eval("1 ⇔ 1").serialize();
	assert!(error.contains("at 1:3"), "{error}");
}

/// card parse-free: the set glyphs mean what math writes, as alias lines and list.warp's set words
#[test]
fn set_glyphs_are_membership_and_set_words() {
	is!("1 ∈ [1 2]", true);
	is!("3 ∉ [1 2]", true);
	is!("3 not in [1 2]", true);
	is!("[1 2] ∋ 2", true);
	is!("[1 2] ∪ [2 3]", parse("[1 2 3]"));
	is!("[1 2] ∩ [2 3]", parse("[2]"));
	is!("[1] ⊂ [1 2]", true);
	is!("[1 2] ⊂ [1 2]", false);
	is!("[1 2] ⊆ [1 2]", true);
	is!("[1 2] ⊃ [1]", true);
	is!("[1 2] ⊇ [3]", false);
	is!("subset([1], [1 2])", true);
}
