//! Footguns of other languages, checked against Warp (see footguns.md).
//! Passing tests back the "Solved" section; `#[ignore = "next"]` tests are the "NOT YET" section's clear-cut fixes.

use warp::wasm_emitter::eval;
use warp::{is, Node};

fn fails_with(code: &str, needle: &str) {
	match eval(code) {
		Node::Error(message) => assert!(format!("{message}").contains(needle), "{message}"),
		other => panic!("expected an error containing {needle:?} for {code}, got {other:?}"),
	}
}

#[test]
fn test_division_is_not_truncating() {
	is!("7/2", 3.5); // C, Java, Python 2: 3
}

#[test]
fn test_leading_zero_is_not_octal() {
	is!("010", 10); // C, sloppy JS: 8
}

#[test]
fn test_no_loose_equality() {
	is!("1==\"1\"", false); // JS: true
}

#[test]
fn test_integers_beyond_double_precision() {
	is!("9007199254740993", 9007199254740993i64); // JS: 9007199254740992
	is!("x=2147483647;x+1", 2147483648i64); // Java/C int: -2147483648
}

#[test]
fn test_power_is_right_associative() {
	is!("2^3^2", 512); // Excel: 64
}

#[test]
fn test_newline_ends_statement() {
	is!("x=1\n-1\nx", 1); // JS: `x = 1\n-1` is x = 0 (no semicolon insertion before '-')
}

#[test]
fn test_braceless_call_takes_whole_argument() {
	is!("f := it*10; f 3-1", 20); // wiki/Bad.md feared (f 3)-1 == 29
}

#[test]
fn test_parameter_shadows_outer_variable() {
	is!("x=1;f(x):=x*2;f(5)+x", 11);
}

#[test]
fn test_zero_string_is_truthy() {
	is!("if \"0\" {1} else {2}", 1); // PHP, Perl: \"0\" is falsy
	is!("if 0 {1} else {2}", 2);
}

#[test]
#[should_panic(expected = "Undefined variable")]
fn test_undefined_variable_is_an_error() {
	eval("a+1"); // JS: NaN (or an implicit global on assignment)
}

#[test]
fn test_law_holds_because_integers_do_not_wrap() {
	is!("square(x) := x*x\nlaw square(x) >= 0\nsquare(3037000500) > 2^63", true); // i64: -9223372036709301616
}

#[test]
fn test_law_catches_a_false_property() {
	fails_with("dec(x) := x-1\nlaw dec(x) >= 0\ndec(0)", "counterexample x=0");
}

#[test]
fn test_hidden_side_effect_is_rejected() {
	fails_with("log(x) := puts x\nsquare(x) := log(x) ! Pure\nsquare(3)", "square → log → puts");
}

#[test]
#[ignore = "next"] // DESIGN.md "Exact numbers by default"
fn test_exact_decimal_arithmetic() {
	is!("0.1+0.2==0.3", true);
	is!("1/3*3==1", true);
}

#[test]
#[ignore = "next"] // bug: comparisons see 0.5, but the returned sum is typed Int and truncated to 0
fn test_sum_of_quotients_is_not_truncated() {
	is!("1/4+1/4", 0.5);
}

#[test]
fn test_integer_overflow_does_not_wrap() {
	is!("2^64 > 2^63", true); // C, Java, Go, Rust --release: 2^64 wraps to 0
	is!("abs(-2^63) > 0", true); // Java: Math.abs(Long.MIN_VALUE) < 0
	is!("100000000000000000000 > 2^64", true); // was a string of NUL bytes
}

#[test]
#[ignore = "next"] // `as i64` works at top level but panics inside a function body: Cannot extract numeric value from (x*x)asi64
fn test_explicit_wrap_inside_function() {
	is!("f(x) := (x*x) as i64; f(3037000500)", -9223372036709301616i64);
}

#[test]
#[ignore = "next"] // panics: Cannot extract numeric value from 'abc'
fn test_string_equality_is_by_value() {
	is!("\"abc\"==\"abc\"", true);
	is!("x=\"abc\";x==\"abc\"", true);
}

#[test]
#[ignore = "next"] // wiki/equality.md: increment is immediate
fn test_increment_changes_variable() {
	is!("x=1;x++;x", 2);
}

#[test]
#[ignore = "next"] // unary minus binds weaker than power in maths and Python
fn test_negative_power_precedence() {
	is!("-2^2", -4);
}

#[test]
#[ignore = "next"] // C precedence footguns: `not`/`&` must bind weaker than comparisons
fn test_logic_binds_weaker_than_comparison() {
	is!("not 1==2", true);
	is!("3 & 4 == 4", false);
}

#[test]
#[ignore = "next"] // bug: returns 3
fn test_braceless_call_as_operand() {
	is!("f := it*10; 1 + f 3", 31);
}

#[test]
#[ignore = "next"] // DESIGN.md ownership: mutation requires a unique place or copy-on-write
fn test_mutation_through_alias_is_not_visible() {
	is!("x=\"ab\";y=x;y#1=\"z\";x", "ab");
	is!("a=(1 2);b=a;b#1=9;a#1", 1);
}

#[test]
#[ignore = "next"] // silently returns the unevaluated program
fn test_index_out_of_bounds_is_an_error() {
	assert!(matches!(eval("x=[1 2 3]; x[3]"), Node::Error(_)));
	assert!(matches!(eval("x=(1 2 3);x#0"), Node::Error(_)));
}

#[test]
#[ignore = "next"] // parsed as the list `1 e3`
fn test_scientific_notation() {
	is!("1e3", 1000);
}

#[test]
#[ignore = "next"] // wiki/unicode.md: strings are UTF-8, `#` indexes characters
fn test_character_indexing_is_unicode_safe() {
	is!("'héllo'#2", 'é');
}

#[test]
#[ignore = "next"] // NFC 'é' vs NFD 'e'+U+0301 panics instead of comparing equal
fn test_unicode_normalization() {
	is!("'\u{e9}'=='e\u{301}'", true);
}

#[test]
#[ignore = "next"] // since fcbd300b Int is unbounded, but Lean still exports BitVec 64: "lean counterexample x=-4611686018427388111"
fn test_proof_model_matches_unbounded_int() {
	let reports = warp::law::verify("square(x) := x*x\nlaw square(x) >= 0");
	assert!(!reports[0].failed(), "{}", reports[0]);
}
