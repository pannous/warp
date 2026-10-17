//! card operator-spelling: every spelling of an operator, symbol, glyph or word (wiki/alias.md, wiki/operator.md), comes
//! from the one table operators::OPERATOR_SPELLINGS and computes what its canonical spelling computes
use crate::is;
use warp::Node;

#[test]
fn word_spellings_compute_as_their_symbols() {
	is!("3 plus 4", 7);
	is!("3 minus 1", 2);
	is!("6 divided by 2", 3);
	is!("7 mod 3", 1);
	is!("7 modulo 3", 1);
	is!("3 equals 3", true);
	is!("3 is 3", true);
	is!("3 same 3", true);
	is!("3 same as 3", true);
	is!("3 is the same as 3", true);
	is!("3 is not the same as 3", false);
	is!("3 is greater than 2", true);
	is!("3 greater than 2", true);
	is!("2 less than 3", true);
	is!("3 is less than 2", false);
	is!("3 is at least 3", true);
	is!("3 is at most 2", false);
	is!("1.0 circa 1.0", true);
	is!("1.0 approximately 1.0", true);
	is!("count(1 through 3)", 3);
}

#[test]
fn logical_spellings_compute_alike() {
	for and in ["and", "&&", "∧", "⋀"] {
		is!(&format!("yes {and} no"), false);
		is!(&format!("yes {and} yes"), true);
	}
	for or in ["or", "||", "∨", "⋁"] {
		is!(&format!("no {or} yes"), true);
		is!(&format!("no {or} no"), false);
	}
	for not in ["not ", "!", "¬"] {
		is!(&format!("{not}yes"), false);
	}
	for xor in ["xor", "⊻"] {
		is!(&format!("yes {xor} no"), true);
	}
}

#[test]
fn glyph_spellings_compute_alike() {
	for minus in ["-", "‐", "‑", "‒", "–", "—", "―", "−"] {
		is!(&format!("5 {minus} 3"), 2);
	}
	for equals in ["==", "≟", "≡", "﹦"] {
		is!(&format!("3 {equals} 3"), true);
	}
	for times in ["*", "×", "·", "⋅"] {
		is!(&format!("2 {times} 3"), 6);
	}
	is!("6 ÷ 2", 3);
	is!("2 ** 3", 8);
	is!("3 ≠ 4", true);
	is!("3 ≤ 3", true);
	is!("3 ≥ 4", false);
}

/// `and=` and `or=` are the word spellings of `&&=` and `||=` (Op::AndAssign, Op::OrAssign)
#[test]
fn word_compound_assignments() {
	is!("x = yes; x and= no; x", false);
	is!("x = no; x or= yes; x", true);
}

/// `xor` of two bools is a bool, as `and` and `or` are: `yes xor no` prints yes, not 1
#[test]
fn xor_of_bools_is_a_bool() {
	assert_eq!(warp::wasm_emitter::eval("yes xor no"), Node::True);
	assert_eq!(warp::wasm_emitter::eval("yes xor yes"), Node::False);
}

/// a word spelling stays a name where no operator can stand: `def mod(a, b)`, a variable `plus`
#[test]
fn word_spellings_stay_names() {
	is!("def mod(a, b) := a - b; mod(7, 3)", 4);
	is!("plus = 3; plus + 1", 4);
	is!("minus = 5; minus", 5);
	is!("same = 2; same", 2);
}

/// a negated spelling negates its operator: `x is not 4` is `not (x is 4)`, `a nand b` is `not (a and b)`; `3 is not 4`
/// was `3 is (not 4)`, no
#[test]
fn negated_spellings() {
	is!("3 is not 4", true);
	is!("x = 3; x is not 3", false);
	is!("x = 3; if x is not 4 {1} else {2}", 1);
	is!("x = 3; x is not text", true);
	is!("x = 3; x is not int", false);
	for nand in ["nand", "¬&", "⊼"] {
		is!(&format!("yes {nand} yes"), false);
		is!(&format!("yes {nand} no"), true);
	}
}
