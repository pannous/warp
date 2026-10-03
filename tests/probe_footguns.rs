//! Footguns of other languages, checked against Warp (see footguns.md).
//! Passing tests back the "Solved" section; `#[ignore = "next"]` tests are the "NOT YET" section's clear-cut fixes.

use crate::common::fails_with;
use warp::wasm_emitter::{eval, eval_untrusted};
use warp::{is, parse_data, Node};

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
fn test_undefined_variable_is_an_error() {
	fails_with("a+1", "undefined variable: a"); // JS: NaN (or an implicit global on assignment)
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
fn test_norway_problem_in_data() {
	assert_eq!(parse_data("country: NO").serialize(), "country:NO"); // YAML 1.1: false
	assert_eq!(parse_data("answer: yes").serialize(), "answer:yes"); // YAML 1.1: true
	assert_eq!(parse_data("[de gb no]").serialize(), "[de gb no]");
	assert_eq!(parse_data("flag: true").serialize(), "flag:true");
	assert_eq!(parse_data("missing: null")["missing"], Node::Empty);
}

#[test]
fn test_data_keeps_number_literals() {
	assert_eq!(parse_data("version: 1.10").serialize(), "version:1.10"); // YAML: 1.1
	assert_eq!(parse_data("zip: 01234").serialize(), "zip:01234"); // CSV importers: 1234
	assert_eq!(parse_data("zip: 01234")["zip"].drop_meta(), &Node::int(1234)); // still a number
	assert_eq!(parse_data("n: 12").serialize(), "n:12");
	assert_eq!(parse_data("mask: 0xFF").serialize(), "mask:0xFF");
	assert_eq!(parse_data("big: 1_000").serialize(), "big:1_000");
}

#[test]
fn test_data_does_not_execute() {
	let loaded = parse_data("secret = fetch https://evil.example/x\nsecret");
	assert!(loaded.serialize().contains("fetch"), "{}", loaded.serialize());
	assert!(matches!(eval_untrusted("puts('pwned')"), Node::Error(_)));
	assert!(matches!(eval_untrusted("use m;floor(4.5)"), Node::Error(_)));
	assert_eq!(eval_untrusted("x:=3;x*x"), Node::int(9));
}

#[test]
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
fn test_explicit_wrap_inside_function() {
	is!("f(x) := (x*x) as i64; f(3037000500)", -9223372036709301616i64);
}

#[test] // Java: new String("abc") == "abc" is false
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
fn test_logic_binds_weaker_than_comparison() {
	is!("not 1==2", true);
	// C reads (3 & (4==4)), Python ((3&4)==4): `&` stays logical `and` (wiki/&.md) and the ungrouped mix is rejected
	fails_with("3 & 4 == 4", "ambiguous");
}

#[test]
fn test_symbolic_logic_next_to_comparison_needs_grouping() {
	fails_with("1==1 | 2==3", "ambiguous");
	fails_with("2==3 & 1", "ambiguous");
	is!("(1==1) | (2==3)", true);
	is!("1==1 and 2==2", true); // the word forms read unambiguously
	is!("1==2 or 2==2", true);
}

#[test]
fn test_braceless_argument_extent_is_consistent() {
	is!("f := it*10; f 3-1", 20);
	is!("f := it*10; 1 + f 3-1", 21); // was 30: (1 + f 3) - 1
	is!("f := it*10; 2 * f 3-1", 40);
	is!("f := it*10; f 3-1 > 15", true); // the argument stops at a comparison
}

#[test]
fn test_braceless_call_in_argument_is_ambiguous() {
	// wiki/precedence.md: square(3 + square 3) or (square 3) + square 3
	fails_with("square := it*it; square 3 + square 3", "ambiguous");
	// wiki/Bad.md: would silently be fib(it-1 + fib(it-2)); was `Undefined variable: it`
	fails_with("fib := it<2 ? it : fib it-1 + fib it-2; fib 10", "fib(it - 1) + fib(it - 2)");
	is!("fib := it<2 ? it : fib(it-1) + fib(it-2); fib 10", 55);
}

#[test]
fn test_braceless_recursive_call_takes_identifier_argument() {
	is!("fac := it<2 ? 1 : it * fac it-1; fac 5", 120);
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
	is!("(3>2)>1", false); // explicit grouping does not chain
}

#[test]
fn test_equality_does_not_chain() {
	// user decision 2026-09-28: only < <= > >= chain; == and != bind weaker and compare the results
	is!("1<2 == 2<3", true);
	is!("1<2 == 3<2", false);
	is!("1+1 == 2", true);
	fails_with("1==1==1", "ambiguous"); // Python: chained (true); C: (1==1)==1 (true by accident)
	fails_with("2==2!=3", "ambiguous");
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
fn test_mutation_through_alias_is_not_visible() {
	is!("x=\"ab\";y=x;y#1=\"z\";x", "ab");
	is!("a=(1 2);b=a;b#1=9;a#1", 1);
}

#[test]
fn test_equal_literals_are_not_shared() {
	is!("x=\"ab\";y=\"ab\";y#1=\"z\";x", "ab"); // the string table deduplicates literals
	is!("x=\"ab\";y=x;y#1=\"z\";y", "zb");
	is!("a=(1 2);b=a;b#2=9;b#2", 9);
}

#[test]
fn test_compound_index_assignment() {
	is!("a=(1 2);a#1 += 1;a#1", 2);
	is!("a=(1 2);b=a;a#2 *= 5;b#2", 2);
}

#[test]
fn test_text_plus_number_is_a_type_error() {
	is!("\"5\"+3", "53"); // JS: "53", C: '5'+3 = 56; user decision 2026-10-02: a number joins a text in its text form
	fails_with("\"5\"*3", "type error"); // JS: 15
	is!("3 + \"4\"", "34");
	is!("\"ab\"+3", "ab3");
	is!("int(\"5\") + 3", 8);
}

#[test]
fn test_list_plus_concatenates() {
	is!("[1 2]+[3]", warp::ints(vec![1, 2, 3])); // JS: "1,23"
	is!("a=(1 2);b=(3 4);c=a+b;#c", 4);
	is!("a=(1 2);b=(3 4);c=a+b;a#2", 2);
	fails_with("[1 2 3]*2", "type error"); // Python: repeats, NumPy: scales
}

#[test]
fn test_append_method_rebinds_the_list() {
	is!("pixel=(1 2);pixel.add(5);pixel", warp::ints(vec![1, 2, 5])); // was unchanged
	is!("pixel=[1 2 3];pixel.add(4);pixel#4", 4);
	is!("a=(1 2);b=a;a.add(3);#b", 2); // value semantics: b keeps its value
	is!("x=[4];x#1", 4); // a one-element list stays a list
}

#[test]
fn test_const_is_single_assignment() {
	fails_with("const x=5;x=6;x", "x is const"); // was silently 6
	fails_with("const x=5;x+=1", "x is const");
	fails_with("const a=(1 2);a#1=3", "a is const");
	is!("const x=5;x", 5);
	is!("a=1;const x=5;x+a", 6);
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
fn test_invalid_number_text_is_an_error() {
	fails_with("int(\"12a\")", "invalid number"); // C atoi: 12, PHP (int)"abc": 0
	fails_with("float(\"1.5x\")", "invalid number");
	fails_with("int(\"\")", "invalid number");
	is!("int(\" -12 \")", -12);
	is!("int(\"2.7\")", 2);
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

#[test] // wiki/unicode.md: strings are UTF-8, `#` indexes characters (Go, C: bytes)
fn test_character_indexing_is_unicode_safe() {
	is!("'héllo'#2", 'é');
}

#[test] // Go: len("👍🏽") is 8 bytes, JS: 4 UTF-16 units, Python: 2 code points; a reader sees 1
fn test_text_is_indexed_by_grapheme() {
	is!("'👍🏽'#1", "👍🏽"); // the skin tone modifier stays with its thumb
	is!("x='a👍🏽b';x#3", 'b');
	is!("x=\"👍🏽\";#x", 1);
	is!("count \"🇩🇪🇫🇷\"", 2); // a flag is a pair of regional indicators
	is!("x=\"héllo\";x.length", 5);
	is!("x=\"q\u{301}\";x.length", 1); // no precomposed q́: one grapheme, two code points
	fails_with("x=\"👍🏽\";x#2", "index out of range");
	assert_eq!(warp::grapheme_clusters("e\u{301}👨\u{200d}👩\u{200d}👧🇩🇪\r\n"), ["e\u{301}", "👨\u{200d}👩\u{200d}👧", "🇩🇪", "\r\n"]);
}

#[test] // size is a synonym for count; bytes only via byte count / .bytes, .chars .graphemes name the unit
fn test_text_units_are_explicit() {
	is!("size \"👍🏽\"", 1);
	is!("x=\"👍🏽\";x.size", 1);
	is!("byte count of \"👍🏽\"", 8);
	is!("x=\"👍🏽\";x.bytes", 8);
	is!("x=\"👍🏽\";x.chars", 2);
	is!("x=\"👍🏽\";x.graphemes", 1);
	is!("pixels=(1,2,3);size(pixels)", 3); // size is a synonym for count
	is!("pixels=(1,2,3);byte count of pixels", 24); // lists keep 8 bytes per element
}

#[test] // Swift: s.unicodeScalars.count; a char is a code point, as x.chars and Rust's chars()
fn test_number_of_chars_in_text() {
	is!("number of chars in \"héllo\"", 5);
	is!("number of chars in \"👍🏽\"", 2);
	is!("t=\"héllo\";number of chars in t", 5);
	is!("#(char in \"héllo\")", 5);
}

#[test] // Swift: s.count; the user-perceived character, the unit of # and count
fn test_number_of_graphemes_in_text() {
	is!("number of graphemes in \"👍🏽\"", 1);
	is!("t=\"👍🏽\";number of graphemes in t", 1);
	is!("t=\"🇩🇪🇫🇷\";#(t as graphemes)", 2);
}

#[test] // Go: len(s); Swift: s.utf8.count
fn test_number_of_bytes_in_text() {
	is!("number of bytes in \"héllo\"", 6);
	is!("#(byte in \"héllo\")", 6);
	is!("#(\"héllo\" as bytes)", 6);
	is!("t=\"héllo\";number of bytes in t", 6);
	is!("t=\"héllo\";#(byte in t)", 6);
	is!("t=\"héllo\";#(t as bytes)", 6);
	is!("count of bytes in \"👍🏽\"", 8);
}

#[test] // Python 3: len(s)
fn test_number_of_codepoints_in_text() {
	is!("number of codepoints in \"👍🏽\"", 2);
	is!("t=\"q\u{301}\";number of codepoints in t", 2);
	is!("#(\"👍🏽\" as codepoints)", 2);
}

#[test] // without a unit: number/count/length of a text count graphemes, size is count, as #x and size x
fn test_count_of_text_without_unit() {
	is!("count of \"👍🏽\"", 1);
	is!("t=\"héllo\";number of t", 5);
	is!("t=\"héllo\";length of t", 5);
	is!("t=\"héllo\";size of t", 5);
	is!("pixels=[1 2 4];number of pixels", 3);
}

#[test] // JS: "i".toUpperCase() → "I" everywhere, only toLocaleUpperCase("tr") → "İ": no locale unless one is given
fn test_case_mapping_is_locale_independent() {
	use warp::StringExtensions;
	assert_eq!("i".upper(), "I");
	assert_eq!("ı".upper(), "I");
	assert_eq!("İ".to_lowercase(), "i\u{307}"); // Unicode's default mapping keeps the dot as a combining mark
	assert_eq!("straße".upper(), "STRASSE");
}

#[test] // index assignment writes the character's UTF-8, not one byte of it
fn test_index_assignment_of_multi_byte_character() {
	is!("x=\"ab\";x#1='é';x", "éb");
	is!("x=\"héllo\";x#2='e';x", "hello");
	is!("x=\"a👍🏽c\";x#2='b';x", "abc"); // replaces the whole grapheme
	is!("x=\"ab\";x#2='👍';x#2", '👍');
	is!("x=\"ab\";x#2='€';x.bytes", 4);
}

#[test] // NFC 'é' vs NFD 'e'+U+0301 compare equal: source text is normalized to NFC
fn test_unicode_normalization() {
	is!("'\u{e9}'=='e\u{301}'", true);
}

#[test] // Python: `a is b` is identity, true for 256 but not for 257
fn test_is_compares_by_value() {
	is!("\"abc\" is \"abc\"", true);
	is!("\"abc\" is \"abd\"", false);
	is!("x=257;x is 257", true);
}

#[test] // JS: 0=="" and [1,2]==[1,2] is false (identity), null==false is false
fn test_equality_across_kinds_is_structural() {
	is!("0==\"\"", false);
	is!("null==false", false);
	is!("[1 2]==[1 2]", true);
	is!("[1 2]==[3]", false);
	is!("\"abc\"!=\"abd\"", true);
	is!("'héllo'#2=='é'", true);
}

#[test] // JSON, JS, Python json: {"a":1,"a":2} silently keeps the last value
fn test_duplicate_keys_are_reported() {
	fails_with("{a:1 a:2}", "duplicate key 'a'");
	fails_with("x={a:1, b:2, a:3};x", "duplicate key 'a'");
	is!("x={a:1 b:2};3", 3);
}

#[test] // truthiness of empty values awaits a decision (Footguns.md → Empty values), but must not crash
fn test_empty_condition_does_not_panic() {
	for code in ["if \"\" {1} else {2}", "if [] {1} else {2}", "x=\"\";if x {1} else {2}"] {
		assert!(matches!(eval(code), Node::Number(_)), "{code}");
	}
}

#[test]
fn test_proof_model_matches_unbounded_int() {
	crate::requires!(crate::common::LEAN);
	let reports = warp::law::verify("square(x) := x*x\nlaw square(x) >= 0");
	assert_eq!(reports[0].assurance, warp::law::Assurance::Proved, "{}", reports[0]);
}

#[test]
fn test_no_array_store_exception() {
	// Java: Object[] a = new String[1]; a[0] = 1; → ArrayStoreException at runtime
	is!("a=(\"x\" \"y\");a#1=1;a#1", 1);
	is!("a=(\"x\" \"y\");a#1=1;a#2", "y");
}

// Dates and time zones: spec for the Decision in Footguns.md (NOT YET → Dates and time zones).
// Literals are RFC 3339 / RFC 9557 only; `+` on calendar units rejects overflow, clamping is spelled out.

#[test]
fn test_months_are_one_based() {
	is!("2024-02-29.month", 2); // JS getMonth(): 1, Java Date.getYear(): 124
	is!("2024-02-29.year", 2024);
	is!("date(2024,2,29).day", 29);
	fails_with("date(2024,0,1)", "month out of range"); // JS: Dec 1 2023
	fails_with("date(2024,2,30)", "day out of range"); // JS new Date(2024,1,30): March 1
}

#[test]
fn test_calendar_overflow_is_explicit() {
	fails_with("2024-01-31 + 1 month", "2024-02-31"); // JS setMonth: March 2, Java plusMonths: Feb 29 silently
	is!("add(2024-01-31, 1 month, overflow: clamp) == 2024-02-29", true);
	is!("2024-01-15 + 1 month == 2024-02-15", true);
	is!("2024-03-01 - 2024-02-01", 29); // date - date → days
}

#[test]
fn test_no_implicit_time_zone() {
	fails_with("now.hour", "instant has no hour"); // an instant needs a zone before it has a wall clock
	is!("(2024-03-31T00:30Z in \"Europe/Berlin\").hour", 1);
	is!("(2024-03-31T01:30Z in \"Europe/Berlin\").hour", 3); // DST: 02:00-03:00 does not exist
	fails_with("2024-03-31T02:30[Europe/Berlin]", "does not exist"); // gap, no silent shift
}

// Future civil time (Footguns.md → Truly impossible): wall times a transition repeats or skips need an explicit choice,
// modelled on JavaScript Temporal's `disambiguation`; the value is the wall time plus zone, the instant is derived.

#[test]
fn test_repeated_local_time_needs_disambiguation() {
	// Python: datetime(2030,10,27,2,30,tzinfo=ZoneInfo("Europe/Berlin")) silently takes fold=0; JS Date takes one silently
	fails_with("2030-10-27T02:30[Europe/Berlin]", "occurs twice");
	fails_with("2030-10-27T02:30[Europe/Berlin]", "2030-10-27T02:30+02:00[Europe/Berlin] (earlier)"); // fix-its name both instants
	fails_with("2030-10-27T02:30[Europe/Berlin]", "2030-10-27T02:30+01:00[Europe/Berlin] (later)");
	is!("t=2030-10-27T02:30+02:00[Europe/Berlin]; t.offset", 7200); // the RFC 9557 offset picks one
	is!("t=zoned(2030-10-27T02:30, \"Europe/Berlin\", disambiguation: earlier); t.offset", 7200);
	is!("t=zoned(2030-10-27T02:30, \"Europe/Berlin\", disambiguation: later); t.offset", 3600);
	fails_with("zoned(2030-10-27T02:30, \"Europe/Berlin\", disambiguation: reject)", "occurs twice");
}

#[test]
fn test_skipped_local_time_can_be_chosen_explicitly() {
	fails_with("2030-03-31T02:30[Europe/Berlin]", "skipped by a daylight saving transition"); // Python: silently 01:30 UTC
	is!("t=zoned(2030-03-31T02:30, \"Europe/Berlin\", disambiguation: earlier); t.hour", 1); // Temporal: 01:30+01:00
	is!("t=zoned(2030-03-31T02:30, \"Europe/Berlin\", disambiguation: later); t.hour", 3); // Temporal: 03:30+02:00
}

#[test]
fn test_zoned_time_records_its_rules_version() {
	is!("t=2030-07-01T10:00[Europe/Berlin]; t.tzdata", "warp-2026a");
	is!("t=2030-07-01T10:00+02:00[Europe/Berlin][_tzdata=warp-2026a]; t.hour", 10); // RFC 9557 suffix round-trips
}

static BERLIN_WITHOUT_DST: [warp::time::Zone; 1] =
	[warp::time::calendar::zone("Europe/Berlin", 3600, warp::time::calendar::Dst::None)];
/// Simulated rule change: the EU abolishes daylight saving before 2030
static RULES_WITHOUT_DST: warp::time::TzRules = warp::time::TzRules {
	version: "sim-2031a",
	published: warp::time::calendar::Date { year: 2031, month: 1, day: 1 },
	zones: &BERLIN_WITHOUT_DST,
};

#[test]
fn test_rule_change_is_not_silent() {
	let saved = "2030-07-01T10:00+02:00[Europe/Berlin][_tzdata=warp-2026a]"; // written under the built-in rules
	warp::time::with_rules(&RULES_WITHOUT_DST, || {
		fails_with(saved, "warp-2026a"); // names both versions
		fails_with(saved, "sim-2031a");
		fails_with(saved, "keep the wall time → 2030-07-01T10:00+01:00[Europe/Berlin]");
		fails_with(saved, "keep the instant → 2030-07-01T09:00+01:00[Europe/Berlin]");
		// without an offset the wall time is the value: re-resolved with the new rules
		is!("t=2030-07-01T10:00[Europe/Berlin][_tzdata=warp-2026a]; t.offset", 3600);
		is!("t=2030-07-01T10:00[Europe/Berlin]; t.tzdata", "sim-2031a");
	});
	is!("t=2030-07-01T10:00[Europe/Berlin]; t.tzdata", "warp-2026a"); // rules restored
}

#[test]
fn test_calendar_day_is_not_24_hours() {
	// Temporal: add({days:1}) keeps the wall time, add({hours:24}) the elapsed time; moment.js add(1,'day') likewise
	is!("t=2030-03-30T12:00[Europe/Berlin] + 1 day; t.hour", 12);
	is!("t=2030-03-30T12:00[Europe/Berlin] + 24 hours; t.hour", 13);
	is!("(2030-03-30T12:00[Europe/Berlin] + 1 day) - 2030-03-30T12:00[Europe/Berlin] == 23 hours", true);
	fails_with("1 day == 24 hours", "equal on some dates and not on others");
	is!("1 hour == 60 minutes", true);
	is!("1 day == 2 days", false);
	fails_with("2030-10-26T02:30[Europe/Berlin] + 1 day", "occurs twice"); // lands in the repeated hour
	is!("t=add(2030-10-26T02:30[Europe/Berlin], 1 day, disambiguation: later); t.offset", 3600);
}

#[test]
fn test_date_and_time_types_are_distinct() {
	fails_with("2024-01-31 < 2024-01-31T10:00", "date"); // date vs local time: no implicit midnight
	fails_with("2024-01-31T10:00 < 2024-01-31T10:00Z", "local time"); // local time vs instant: no implicit zone
	is!("2024-01-31T10:00+01:00 == 2024-01-31T09:00Z", true); // offsets denote instants
}

#[test]
fn test_type_annotation_is_enforced() {
	fails_with("x:int=5;x=\"five\";x", "x is declared int"); // was a compiler panic
	fails_with("x:int=\"five\"", "fix: x=int(\"five\")"); // was silently 0
	fails_with("x:int=5\nx=2.5", "at 2:1"); // a lossy conversion is never inserted silently
	is!("x:float=5;x", 5.0); // widening is fine
}

#[test]
fn test_closures_capture_values() {
	is!("x=1;f(y):=x+y;f(1)", 2);
	is!("x=1;f(y):=x+y;x=5;f(1)", 2); // captured at definition; by reference (JS, Python): 6
	is!("xs=(1 2 3);f(i):=xs#i;f(2)", 2);
}

#[test]
fn test_closures_in_loop_capture_each_iteration() {
	// Python `[lambda: i for i in range(3)]`, JS `var`, Go < 1.22: every closure sees the last value
	is!("x=0;r=0;i=0;while i<3 { i+=1; x=i; f(y):=x+y; x=100; r+=f(0) }; r", 6); // by reference: 300
	is!("i=0;while i<3 { i+=1; f(y):=i*y }; i=10; f(1)", 3); // by reference: 10
}

#[test]
fn test_default_argument_is_fresh_per_call() {
	// Python `def f(a=[])` shares one list across calls: the second call sees the first call's mutation
	is!("def f(a=(0 0)){ a#1 = a#1 + 1; a#1 }; f()+f()", 2); // shared default: 1+2=3
	is!("f(a=(0 0)) := { a#1 = a#1 + 1; a#1 }; f(); f()", 1);
}

#[test]
fn test_empty_values_are_falsy() {
	is!("if \"\" {1} else {2}", 2); // Python: falsy, JS: falsy; was a compiler panic
	is!("if [] {1} else {2}", 2); // Python: falsy, JS: truthy
	is!("if ø {1} else {2}", 2);
	is!("if 0.5 {1} else {2}", 1); // was truncated to 0
	is!("x=0.5;if x {1} else {2}", 1);
	is!("if 2^32 {1} else {2}", 1); // was wrapped to i32 0
}

#[test]
fn test_and_or_ternary_is_linted() {
	let warnings = warp::analyzer::lint(&warp::parse("1 and 0 or 2"));
	assert_eq!(warnings[0].fix.as_deref(), Some("if 1 then 0 else 2"), "{warnings:?}");
	is!("1 and 0 or 2", 2); // Python, Lua: y when x is falsy; well defined, so only a warning
	assert!(warp::analyzer::lint(&warp::parse("if 1 then 0 else 2")).is_empty());
}

#[test]
fn test_null_needs_a_check() {
	fails_with("x=ø; x+1", "x may be ø"); // Java NPE, JS TypeError; was a compiler panic
	is!("x=ø; x.size", 0); // ø is the empty list: its size is 0 (decided 2026-10-02, open_decisions #28)
	let accepted = |code: &str| warp::analyzer::diagnose(&warp::parse(code)).is_none();
	assert!(accepted("x=ø; if x {x+1} else {2}")); // checked: narrowed inside the branch
	assert!(accepted("x=ø; x=3; x+1"));
}

#[test] // a local first assigned ø is held as a Node until checked; was a compiler panic: Cannot extract numeric value from ø
fn test_optional_local_runs() {
	is!("x=ø; if x {x+1} else {2}", 2);
	is!("x=ø; x=3; x+1", 4);
}

#[test] // Swift/Kotlin `Int?`: T? declares an optional, a plain T refuses ø
fn test_optional_type_declaration() {
	is!("x:int?=ø; if x {x+1} else {2}", 2); // was a parse error: Unexpected character '='
	is!("x:int?=ø; x=4; x+1", 5);
	fails_with("x:int?=ø; x+1", "x may be ø");
	fails_with("x:int=ø", "declare x:int? to allow ø");
}

#[test] // Decision: `()` is ø and ø is the empty list (Footguns.md → Null: optional types)
fn test_empty_parens_is_the_empty_list() {
	is!("a=();a.add(1)", warp::ints(vec![1]));
	is!("a=();a.add(1);a.add(2);a", warp::ints(vec![1, 2]));
}

#[test] // Go `v, _ := f()`, Java `catch {}`: compiler failures come back as error values with a position
fn test_compiler_failures_are_error_values() {
	fails_with("1+(a:2)", "cannot extract a numeric value from a:2 at 1:4"); // was a compiler panic
	is!("x=0.5;x=0", 0); // was a WASM validation error
}

#[test]
fn test_exact_rationals_stay_exact() {
	is!("1/3", Node::Number(warp::Number::Quotient(1, 3))); // Python/JS: 0.3333333333333333
	is!("x=0.1;x*3", 0.3);
	is!("2^-2", 0.25); // was a wasm trap: negative exponents need rationals
	is!("1/3 < 0.34", true);
}

#[test]
fn test_division_by_zero_is_extended_rational() {
	is!("1/0 > 10^100", true); // exact infinity = 1/0, not IEEE's float Infinity
	is!("1/(1/0)", 0);
	is!("1/0 - 1/0", Node::Number(warp::Number::Nan));
	is!("x=0/0; x==x", true); // IEEE: NaN != NaN
	is!("0/0 == 1", false);
}

#[test]
fn test_sql_holes_are_parameters_not_text() {
	use warp::injection::{template, Language::Sql};
	let query = template(Sql, "SELECT * FROM t WHERE name = $name AND age > ${min+1}").unwrap();
	assert_eq!(query.query(), Some("SELECT * FROM t WHERE name = ? AND age > ?"));
	assert_eq!(query.holes.len(), 2);
	assert_eq!(query.holes[0], Node::Symbol("name".into()));
	assert_eq!(template(Sql, "SELECT '$$5'").unwrap().query(), Some("SELECT '$5'"));
	// every language with string building: name = "x' OR '1'='1" → all rows
	let injected = "name=\"x' OR '1'='1\";q=sql \"SELECT * FROM t WHERE name = $name\"";
	is!(&format!("{injected};q#1"), "SELECT * FROM t WHERE name = ?"); // the query keeps its shape
	is!(&format!("{injected};q#2"), "x' OR '1'='1"); // the value stays a value
}

#[test]
fn test_sql_from_built_text_is_rejected() {
	fails_with("name=\"x\";sql(\"SELECT * FROM t WHERE name='\" + name + \"'\")", "sql takes a literal template");
	fails_with("name=\"x\";sql \"SELECT * FROM t WHERE name = '$name'\"", "a parameter is a value");
	fails_with("q=\"SELECT * FROM users\";execute q", "execute takes a sql template");
}

#[test]
fn test_shell_holes_are_whole_arguments() {
	use warp::injection::{template, Language::Shell};
	let command = template(Shell, "rm -f $file").unwrap();
	assert_eq!(command.text, [Some("rm".to_string()), Some("-f".to_string()), None]);
	assert_eq!(command.holes, [Node::Symbol("file".into())]);
	is!("file=\"a; rm -rf ~\";c=sh \"rm -f $file\";c#3", "a; rm -rf ~"); // one argument, no shell parses it
	fails_with("sh \"ls | grep x\"", "no shell");
	fails_with("f=\"a\";sh \"cp --out=$f b\"", "whole argument");
	fails_with("exec \"rm -rf /\"", "exec takes a sh template");
}

#[test]
fn test_running_queries_and_commands_needs_a_capability() {
	is!("lookup(n) := execute sql \"SELECT * FROM t WHERE id = $n\"\neffects of lookup", Node::Symbol("IO".into()));
	fails_with("c=sh \"ls -l\";exec c", "exec needs the process capability");
	fails_with("execute sql \"SELECT 1\"", "execute needs the sql capability");
	fails_with("lookup(n) := execute sql \"SELECT * FROM t WHERE id = $n\" ! Pure\nlookup(1)", "lookup is declared ! Pure but performs IO");
}

#[test]
fn test_date_literal_needs_strict_form() {
	is!("2024-1-31", 1992); // not RFC 3339: arithmetic
	is!("2024 - 01 - 31", 1992);
	is!("x = 2024-02-29; x.month", 2);
	is!("(2024-01-31T23:30 + 1 hour).day", 1);
	fails_with("2024-02-29 + 1 year", "2025-02-29");
	fails_with("2024-02-30", "day out of range");
}

#[test]
fn test_modulo_and_remainder_are_both_named() {
	// % is Euclidean as in mathematics: a == b*q + r with 0 ≤ r < |b| (C/JS/Java truncate, Python floors)
	is!("-7 % 3", 2); // C, JS, Java: -1
	is!("7 % -3", 1); // Python: -2
	is!("-7 % -3", 2);
	is!("7 % 3", 1);
	is!("-6 % 3", 0);
	is!("x=-7; x % 3", 2);
	is!("x=-7; x %= 3; x", 2);
	is!("x=-7; x /= 3; x", -3); // integer /= is the Euclidean quotient: -7 == 3*-3 + 2
	is!("x=7; x /= -3; x", -2); // 7 == -3*-2 + 1
	is!("-123456789012345678901234567890 % 1000", 110);
	is!("(-7/2 % 3) * 2", 5); // exact ratios too: -3.5 % 3 == 2.5
	// mod is the same operation as %
	is!("-7 mod 3", 2);
	is!("7 mod -3", 1);
	is!("x=-7; x mod 3", 2);
	is!("1 + -7 mod 3", 3); // binds like %
	is!("-123456789012345678901234567890 mod 1000", 110);
	// rem is the truncated remainder, sign of the dividend (C, JS, Java, Rust %)
	is!("-7 rem 3", -1);
	is!("7 rem -3", 1);
	is!("x=-7; x rem 3", -1);
	is!("-123456789012345678901234567890 rem 1000", -890);
}

#[test]
fn test_negative_modulo_is_linted() {
	let warnings = warp::analyzer::lint(&warp::parse("-7 % 3"));
	let message = &warnings.first().expect("a warning for a negative % operand").message;
	assert!(message.contains("`-7 % 3` is 2") && message.contains("give -1") && message.contains("rem"), "{message}");
	assert!(!warp::analyzer::lint(&warp::parse("x=5; y=-x % 3")).is_empty());
	assert!(warp::analyzer::lint(&warp::parse("7 % 3")).is_empty());
	assert!(warp::analyzer::lint(&warp::parse("-7 rem 3")).is_empty());
}

#[test]
fn test_rounding_mode_is_named() {
	is!("round(2.5)", 2); // round is round half even (IEEE 754, Python 3, .NET)
	is!("round(3.5)", 4);
	is!("round_half_even(2.5)", 2);
	is!("round_half_up(2.5)", 3); // JS Math.round, Excel
	is!("round_half_up(0.5)", 1);
	is!("round_half_up(-2.5)", -2); // up means toward +∞, like Math.round
	is!("round_half_up(2.4)", 2);
	is!("round_half_up(5/2)", 3);
}

#[test]
fn test_booleans_are_not_numbers() {
	fails_with("true + true", "arithmetic on a boolean"); // Python True + True → 2
	fails_with("false * 3", "fix: int(false) * 3");
	fails_with("(1<2) + 1", "arithmetic on a boolean");
	fails_with("(not 1) + 2", "arithmetic on a boolean");
	let accepted = |code: &str| warp::analyzer::diagnose(&warp::parse(code)).is_none();
	assert!(accepted("int(true) + int(true)"));
	assert!(accepted("x = 1 < 2; if x {1} else {2}"));
}

#[test] // IEEE 754: (0.1+0.2)+0.3 != 0.1+(0.2+0.3); number literals are exact by default
fn test_addition_is_associative_by_default() {
	is!("(0.1+0.2)+0.3 == 0.1+(0.2+0.3)", true);
}

#[test] // IEEE floats are the explicit opt-in, and then the law is weaker
fn test_fast_floats_are_not_associative() {
	is!("fast a=0.1; fast b=0.2; fast c=0.3; (a+b)+c == a+(b+c)", false);
	is!("a=0.1 as float; b=0.2 as float; c=0.3 as float; (a+b)+c == a+(b+c)", false);
}

#[test] // user decision 2026-09-28: `as` converts the whole arithmetic expression to its left (C#, TypeScript), not
// just the nearest operand (Rust, Kotlin); an ungrouped mix gets a warning with both readings as fix-it
fn test_as_converts_the_whole_expression() {
	is!("2 * 1.5 as int", 3); // Rust: 2 * (1.5 as i32) → 2
	is!("2 * (1.5 as int)", 2);
	is!("x=3.3 as float; x", 3.3);
	let sum = eval("0.1 as float + 0.2 as float");
	assert!(matches!(sum.drop_meta(), Node::Number(warp::Number::Float(f)) if *f == 0.30000000000000004), "{sum:?}");
	let warnings = warp::analyzer::lint(&warp::parse("2 * 1.5 as int"));
	assert!(warnings.iter().any(|w| w.fix.as_deref() == Some("(2*1.5) as int or 2 * 1.5:int")), "{warnings:?}");
	assert!(warp::analyzer::lint(&warp::parse("(2 * 1.5) as int")).is_empty());
	assert!(warp::analyzer::lint(&warp::parse("x=3.3 as float")).is_empty());
}

#[test] // user decision 2026-09-28: a tight conversion is written on the literal itself, `0.1:float` or C's `0.1f`
fn test_typed_literals_bind_tightly() {
	is!("0.1 + 0.2 == 0.3", true); // exact by default
	is!("0.1:float + 0.2:float == 0.3", false);
	is!("0.1f + 0.2f == 0.3", false);
	is!(".1f + .2f == .3", false);
	is!("x=0.1f; x+0.2 == 0.3", false);
	is!("3f", 3.0);
	is!("0.1F + 0.2F == 0.3", false);
	is!("0.1d + 0.2D == 0.3", false); // Java, C#: double
	is!("0.1l + 0.2l == 0.3", true); // C's long double suffix: exact, as without suffix
	is!(".1L + .2L == .3", true);
	is!("2 * 1.5:int", 2);
	assert!(warp::analyzer::lint(&warp::parse("2 * 1.5:int")).is_empty(), "typed literals are not ambiguous");
}

#[test] // real/exact and float/fast/f64 are aliases, prefix and suffix declarations mean the same
fn test_number_declaration_spellings_agree() {
	let exact = ["x=3.3;x", "real x=3.3;x", "exact x=3.3;x", "x:real=3.3;x", "x:exact=3.3;x", "x=3.3 as real;x", "x=3.3 as exact;x"];
	for code in exact {
		let result = eval(code);
		assert!(matches!(result.drop_meta(), Node::Number(warp::Number::Quotient(33, 10))), "{code} → {result:?}");
	}
	let fast = ["fast x=3.3;x", "float x=3.3;x", "x:float=3.3;x", "x:fast=3.3;x", "x=3.3 as float;x", "x=3.3 as fast;x", "x:f64=3.3;x"];
	for code in fast {
		let result = eval(code);
		assert!(matches!(result.drop_meta(), Node::Number(warp::Number::Float(f)) if *f == 3.3), "{code} → {result:?}");
	}
}

// Exact real numbers: user decision 2026-09-28, a sparse polynomial with rational coefficients over π, ℯ, ⅈ and roots

#[test]
fn test_square_roots_multiply_exactly() {
	is!("√2*√2 == 2", true); // IEEE: 2.0000000000000004
	is!("√2*√2", 2);
}

#[test]
fn test_square_roots_are_reduced() {
	is!("sqrt(8) == 2*√2", true);
	assert_eq!(eval("√8").serialize(), "2√2");
}

#[test]
fn test_cube_roots_are_exact() {
	is!("∛27 == 3", true); // IEEE: 27^(1/3) = 3.0000000000000004
	is!("∛27", 3);
	assert_eq!(eval("∛16").serialize(), "2∛2");
}

#[test]
fn test_sine_at_rational_multiple_of_pi() {
	is!("sin(π/6) == 1/2", true); // IEEE: 0.49999999999999994
}

#[test]
fn test_cosine_of_pi() {
	is!("cos(π) == -1", true);
}

#[test]
fn test_pi_compares_by_interval_arithmetic() {
	is!("π > 3.14", true);
	is!("π < 3.15", true);
	is!("π < 355/113", true); // agree to 7 digits, decided with more precision
}

#[test]
fn test_pi_plus_euler_prints_symbolically() {
	assert_eq!(eval("π+ℯ").serialize(), "π+ℯ");
	assert_eq!(eval("pi/2").serialize(), "π/2");
	assert_eq!(eval("3+√2").serialize(), "3+√2");
}

#[test]
fn test_sum_of_different_roots_stays_exact() {
	assert_eq!(eval("sqrt(2)+sqrt(3)").serialize(), "√2+√3");
	is!("(sqrt(2)+sqrt(3))^2 == 5+2*√6", true);
}

#[test]
fn test_approximation_is_marked() {
	assert_eq!(eval("sin(1)").serialize(), "≈0.8414709848078965");
	fails_with("sin(1) == sin(1)", "undecidable");
}

#[test]
fn test_as_float_converts_exact_values() {
	let pi = eval("π as float");
	assert!(matches!(pi.drop_meta(), Node::Number(warp::Number::Float(f)) if *f == 3.141592653589793), "{pi:?}");
}

#[test]
fn test_euler_identity() {
	is!("ℯ^(ⅈ*π) == -1", true);
	is!("ln(ℯ) == 1", true);
	is!("ln(1)", 0);
	is!("exp(0)", 1);
}

// ── Termination and determinism (work area 'termination-determinism') ──────────────────────────

/// Bits of `a / b` computed by WASM on this CPU through the project's engine
fn wasm_divide_bits(a: f64, b: f64) -> u64 {
	use wasmtime::{Instance, Module};
	let engine = warp::util::gc_engine();
	let module = Module::new(&engine, r#"(module (func (export "div") (param f64 f64) (result f64) local.get 0 local.get 1 f64.div))"#)
		.expect("valid module");
	let mut store = warp::util::fueled_store(&engine, ());
	let instance = Instance::new(&mut store, &module, &[]).expect("instantiates");
	let divide = instance.get_typed_func::<(f64, f64), f64>(&mut store, "div").expect("exported");
	divide.call(&mut store, (a, b)).expect("runs").to_bits()
}

#[test] // x86 produces the negative NaN 0xfff8…, ARM the positive one: results differ by CPU unless canonicalized
fn test_nan_bits_are_canonical() {
	const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;
	assert_eq!(wasm_divide_bits(0.0, 0.0), CANONICAL_NAN);
	assert_eq!(wasm_divide_bits(-0.0, 0.0), CANONICAL_NAN);
	assert_eq!(wasm_divide_bits(f64::from_bits(0xfff8_0000_0000_0001), 1.0), CANONICAL_NAN, "NaN payloads do not leak");
	assert_eq!(wasm_divide_bits(1.0, 4.0), 0.25f64.to_bits(), "ordinary results are untouched");
}

#[test] // Truly impossible to decide (halting problem); at runtime a fuel budget turns a hang into a loud error
fn test_infinite_loop_runs_out_of_fuel() {
	match warp::util::with_fuel(1_000_000, || eval("while 1 {}")) {
		Node::Error(message) => assert!(format!("{message}").contains("out of fuel after 1000000 steps"), "{message}"),
		other => panic!("expected out of fuel, got {other:?}"),
	}
}

#[test] // the budget is a limit, not a guess: a long but finite run passes once it is raised
fn test_fuel_budget_can_be_raised() {
	let counting = "i=0; while i<1000 { i++ }; i";
	match warp::util::with_fuel(100, || eval(counting)) {
		Node::Error(message) => assert!(format!("{message}").contains("out of fuel after 100 steps"), "{message}"),
		other => panic!("expected out of fuel, got {other:?}"),
	}
	assert_eq!(eval(counting), 1000);
	assert!(warp::util::DEFAULT_FUEL >= 1_000_000_000, "the default is generous");
}

fn effects(code: &str, function: &str) -> warp::effects::EffectSet {
	warp::effects::effects_of(code, function).unwrap_or_else(|| panic!("{function} unresolved in {code}"))
}

#[test] // Koka's div: recursion that shrinks a parameter toward a guarded base case is total
fn test_shrinking_recursion_is_total() {
	use warp::effects::EffectSet;
	for (code, name) in [
		("fib(n) := n<2 ? n : fib(n-1)+fib(n-2)", "fib"),
		("fac(n) := n<2 ? 1 : n*fac(n-1)", "fac"),
		("up(n) := n>=10 ? n : up(n+1)", "up"),
		("down(n) := n>0 ? down(n-1) : 0", "down"),
	] {
		assert_eq!(effects(code, name), EffectSet::PURE, "{code} parsed as {:?}", warp::parse(code));
	}
	is!("fib(n) := n<2 ? n : fib(n-1)+fib(n-2) ! Pure\nfib(10)", 55);
}

#[test] // when unsure, Div: unguarded, unbounded (fac(-1)), wrong-way and mutual recursion may diverge
fn test_unproven_recursion_may_diverge() {
	use warp::effects::Effect::Div;
	for (code, name) in [
		("f(n) := f(n)", "f"),
		("f(n) := n==0 ? 0 : n*f(n-1)", "f"),
		("f(n) := n<2 ? n : f(n+1)", "f"),
		("f(n) := n<2 ? n : f(n)", "f"),
		("f(n) := n<2 ? n : f(n-0)", "f"),
		("even(n) := n<1 ? 1 : odd(n-1)\nodd(n) := n<1 ? 0 : even(n-1)", "even"),
		("spin(n) := spin(n)\ncaller(n) := spin(n)+1", "caller"),
	] {
		assert!(effects(code, name).contains(Div), "{code}: {}", effects(code, name));
	}
}

#[test] // a while loop has an unknown condition in general: Div
fn test_while_loop_may_diverge() {
	use warp::effects::Effect::Div;
	assert!(effects("i=0; while i<3 { i++ }; i", "main").contains(Div));
	assert!(!effects("x=3; x*2", "main").contains(Div));
}

#[test] // `effects of f` reports Div and `f ! Pure` rejects a function that may diverge
fn test_pure_rejects_divergence() {
	is!("f(n) := f(n)\neffects of f", Node::Symbol("Div".into()));
	fails_with("f(n) := n==0 ? 0 : n*f(n-1) ! Pure\nf(3)", "f is declared ! Pure but performs Div via f");
	fails_with("spin(n) := spin(n)\ncaller(n) := spin(n)+1 ! Pure\ncaller(1)", "caller → spin");
	is!("f(n) := n==0 ? 1 : n*f(n-1) ! Div\nf(3)", 6); // declared divergence is allowed
}

/// Local HTTP stub answering every request with `status` and `body`: the fetch tests need no network
fn serve(status: &'static str, body: &'static str) -> String {
	use std::io::{Read, Write};
	let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
	let address = listener.local_addr().unwrap();
	std::thread::spawn(move || {
		for mut stream in listener.incoming().flatten() {
			let mut request = [0u8; 4096];
			let _ = stream.read(&mut request);
			let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
			let _ = stream.write_all(response.as_bytes());
		}
	});
	format!("http://{address}/data")
}

const UNREACHABLE: &str = "http://127.0.0.1:9/"; // discard port, nothing listens: connection refused

#[test] // JS wrappers resolve to "", Go drops err, PHP file_get_contents returns false: a failed fetch is an Error value
fn test_failed_fetch_is_an_error_value() {
	fails_with(&format!("fetch \"{UNREACHABLE}\""), "fetch http://127.0.0.1:9/ failed");
	let missing = serve("404 Not Found", "no such page");
	fails_with(&format!("fetch \"{missing}\""), "HTTP status 404");
	fails_with(&format!("x = fetch \"{missing}\"; x"), "HTTP status 404");
	is!(&format!("fetch \"{}\"", serve("200 OK", "hello")), "hello\n");
}

#[test] // Java/Python/JS: a fetch result is used like a local value; Rust/Swift: the Result must be unwrapped first
fn test_fetch_result_needs_a_check() {
	let code = format!("x = fetch \"{UNREACHABLE}\"; x + \"!\"");
	fails_with(&code, "x may be an error");
	fails_with(&code, "fix: if x {");
	fails_with(&format!("x = fetch \"{UNREACHABLE}\"; x.size"), "x may be an error");
	fails_with(&format!("1 + fetch \"{UNREACHABLE}\""), "may be an error"); // the fix binds it: result = fetch …; if result {…}
	is!(&format!("x = fetch \"{UNREACHABLE}\"; if x {{x}} else {{\"offline\"}}"), "offline"); // an error is falsy
	let ok = serve("200 OK", "hello");
	is!(&format!("x = fetch \"{ok}\"; if x {{x}} else {{\"offline\"}}"), "hello\n");
}

#[test] // browsers, curl, Python requests: no timeout by default, a stalled server hangs the caller forever
fn test_fetch_times_out_loudly() {
	assert!(warp::host::FETCH_TIMEOUT <= std::time::Duration::from_secs(30), "default timeout");
	let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap(); // the kernel accepts, nobody ever answers
	let url = format!("http://{}/", silent.local_addr().unwrap());
	fails_with(&format!("fetch \"{url}\" timeout 0.5"), "timeout after 500 ms");
	fails_with(&format!("x = fetch \"{url}\" timeout 0.5; x"), "timeout after 500 ms");
	drop(silent);
}

#[test] // Unison: definitions are named by the hash of their normalized AST, parameter names do not count
fn test_function_equality_up_to_renaming() {
	is!("f(x):=x+1; g(y):=y+1; f==g", true);
	is!("f(x):=x+1; g(y):=y+1; f!=g", false);
	is!("f(x):=x+1; f==f", true);
	is!("fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fibo(m) = m < 2 ? m : fibo(m - 1) + fibo(m - 2); fib==fibo", true);
	is!("f(x):=x+1; g(x,y):=x+1; f==g", false); // different arity, different domain
}

#[test] // polynomials over exact numbers have a normal form: expand and collect
fn test_polynomial_function_equality() {
	is!("f(x):=(x+1)^2; g(x):=x^2+2*x+1; f==g", true); // panicked the compiler
	is!("f(x):=x*x; g(x):=x+x; f==g", false);
	is!("f(x):=x*x; g(x):=x+x; f!=g", true);
	is!("f(x,y):=(x - y)*(x+y); g(a,b):=a^2-b^2; f==g", true);
	is!("f(x):=x/2+x/2; g(x):=x; f==g", true); // exact division
}

#[test] // a finite domain is compared input by input
fn test_finite_domain_function_equality() {
	is!("f(a:bool,b:bool):=not (a or b); g(a:bool,b:bool):=(not a) and (not b); f==g", true); // De Morgan
	is!("f(a:bool,b:bool):=a and b; g(a:bool,b:bool):=a or b; f==g", false);
}

#[test] // Rice: equality of arbitrary functions is undecidable, so it is an error, never a guess
fn test_undecidable_function_equality_is_an_error() {
	fails_with("f(x):=x%2; g(x):=x%3; f==g", "undecidable: f == g");
	fails_with("f(x):=x%2; g(x):=x%3; f==g", "counterexample x=");
	fails_with("f(x):=x%2; g(x):=(x*3)%2; f==g", "undecidable: f == g");
}
