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
fn test_increment_changes_variable() {
	is!("x=1;x++;x", 2);
}

#[test]
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
fn test_braceless_call_as_operand() {
	is!("f := it*10; 1 + f 3", 31);
}

#[test]
fn test_not_binds_weaker_than_comparison() {
	is!("not 1==2", true); // C: !1 == 2 → 0
	is!("!1==1", false);
	is!("not 1==2 and 2==2", true);
}

#[test]
fn test_chained_comparison() {
	is!("3>2>1", true); // C, JS: (3>2)>1 → false
	is!("1<3<2", false);
	is!("1<2<3", true);
	is!("1<2==2", true);
	is!("(3>2)>1", false); // explicit grouping does not chain
}

#[test]
fn test_equals_in_condition_compares() {
	is!("x=1;if x=2 {3} else {4}", 4); // C, JS: assigns, always true
	is!("x=2;if x=2 {3} else {4}", 3);
	is!("x=1;if x=2 {3};x", 1);
	is!("if 1=2 {3} else {4}", 4);
}

#[test]
fn test_prefix_increment() {
	is!("i=1;++i;i", 2);
	is!("i=1;++i", 2);
	is!("i=3;--i;i", 2);
}

#[test]
fn test_negative_literals_after_power_fix() {
	is!("-2*3", -6);
	is!("x=-1;x", -1);
	is!("3 - -2", 5);
	is!("(-2)^2", 4);
}

#[test]
#[ignore = "next"] // DESIGN.md ownership: mutation requires a unique place or copy-on-write
fn test_mutation_through_alias_is_not_visible() {
	is!("x=\"ab\";y=x;y#1=\"z\";x", "ab");
	is!("a=(1 2);b=a;b#1=9;a#1", 1);
}

#[test]
fn test_index_out_of_bounds_is_an_error() {
	assert!(matches!(eval("x=[1 2 3]; x[3]"), Node::Error(_)));
	assert!(matches!(eval("x=(1 2 3);x#0"), Node::Error(_)));
}

#[test]
fn test_negative_index_is_an_error() {
	fails_with("x=[1 2 3];x[-1]", "index out of range"); // Python: 3, silently wraps
	fails_with("x=\"ab\";x#3", "index out of range"); // C: reads past the end
	fails_with("x=(1 2 3);x#4=0", "index out of range");
}

#[test]
fn test_scientific_notation() {
	is!("1e3", 1000);
}

#[test]
fn test_scientific_notation_forms() {
	is!("1e3+1", 1001);
	is!("1.5e3", 1500.0);
	is!("2E-3", 0.002);
	is!("1e+2", 100);
	is!("1e20 == 100000000000000000000", true);
}

#[test]
fn test_digit_separators() {
	is!("1_000_000", 1000000);
	is!("1_000.000_5", 1000.0005);
	is!("x=1_000;x+1", 1001);
}

#[test]
fn test_leading_dot_literal() {
	is!(".5", 0.5);
	is!(".5+1", 1.5);
	is!("-.5+1", 0.5);
	is!("i=3.5;.5+i", 4.0);
}

#[test]
fn test_norm_bars_are_brackets() {
	is!("‖-5‖", 5);
	is!("x=-5;‖x‖", 5);
	is!("‖-5‖+1", 6);
	is!("‖3-5‖*2", 4);
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
fn test_proof_model_matches_unbounded_int() {
	let reports = warp::law::verify("square(x) := x*x\nlaw square(x) >= 0");
	assert_eq!(reports[0].assurance, warp::law::Assurance::Proved, "{}", reports[0]);
}

#[test]
#[ignore = "next"] // unverified: written while the build was blocked; lists are heterogeneous, no static element type
fn test_no_array_store_exception() {
	// Java: Object[] a = new String[1]; a[0] = 1; → ArrayStoreException at runtime
	is!("a=(\"x\" \"y\");a#1=1;a#1", 1);
	is!("a=(\"x\" \"y\");a#1=1;a#2", "y");
}

// Dates and time zones: spec for the Decision in Footguns.md (NOT YET → Dates and time zones).
// Literals are RFC 3339 / RFC 9557 only; `+` on calendar units rejects overflow, clamping is spelled out.

#[test]
#[ignore = "next"] // no date type yet
fn test_months_are_one_based() {
	is!("2024-02-29.month", 2); // JS getMonth(): 1, Java Date.getYear(): 124
	is!("2024-02-29.year", 2024);
	is!("date(2024,2,29).day", 29);
	fails_with("date(2024,0,1)", "month out of range"); // JS: Dec 1 2023
	fails_with("date(2024,2,30)", "day out of range"); // JS new Date(2024,1,30): March 1
}

#[test]
#[ignore = "next"] // no date type yet
fn test_calendar_overflow_is_explicit() {
	fails_with("2024-01-31 + 1 month", "2024-02-31"); // JS setMonth: March 2, Java plusMonths: Feb 29 silently
	is!("add(2024-01-31, 1 month, overflow: clamp) == 2024-02-29", true);
	is!("2024-01-15 + 1 month == 2024-02-15", true);
	is!("2024-03-01 - 2024-02-01", 29); // date - date → days
}

#[test]
#[ignore = "next"] // no date/time types yet
fn test_no_implicit_time_zone() {
	fails_with("now.hour", "instant has no hour"); // an instant needs a zone before it has a wall clock
	is!("(2024-03-31T00:30Z in \"Europe/Berlin\").hour", 1);
	is!("(2024-03-31T01:30Z in \"Europe/Berlin\").hour", 3); // DST: 02:00-03:00 does not exist
	fails_with("2024-03-31T02:30[Europe/Berlin]", "does not exist"); // gap, no silent shift
}

#[test]
#[ignore = "next"] // no date/time types yet
fn test_date_and_time_types_are_distinct() {
	fails_with("2024-01-31 < 2024-01-31T10:00", "date"); // date vs local time: no implicit midnight
	fails_with("2024-01-31T10:00 < 2024-01-31T10:00Z", "local time"); // local time vs instant: no implicit zone
	is!("2024-01-31T10:00+01:00 == 2024-01-31T09:00Z", true); // offsets denote instants
}
