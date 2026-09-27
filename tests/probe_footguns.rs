//! Footguns of other languages, checked against Warp (see footguns.md).
//! Passing tests back the "Solved" section; `#[ignore = "next"]` tests are the "NOT YET" section's clear-cut fixes.

use warp::wasm_emitter::{eval, eval_untrusted};
use warp::{is, parse_data, Node};

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
#[ignore = "next"] // `as i64` works at top level but panics inside a function body: Cannot extract numeric value from (x*x)asi64
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
	fails_with("\"5\"+3", "type error"); // JS: "53", C: '5'+3 = 56
	fails_with("\"5\"*3", "type error"); // JS: 15
	fails_with("3 + \"4\"", "type error");
	fails_with("\"ab\"+3", "type error");
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

#[test]
fn test_date_and_time_types_are_distinct() {
	fails_with("2024-01-31 < 2024-01-31T10:00", "date"); // date vs local time: no implicit midnight
	fails_with("2024-01-31T10:00 < 2024-01-31T10:00Z", "local time"); // local time vs instant: no implicit zone
	is!("2024-01-31T10:00+01:00 == 2024-01-31T09:00Z", true); // offsets denote instants
}

#[test]
fn test_type_annotation_is_enforced() {
	fails_with("x:int=5;x=\"five\";x", "x is declared int"); // was a compiler panic
	fails_with("x:int=\"five\"", "fix: x=int('five')"); // was silently 0
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
	fails_with("x=ø; x.size", "fix: if x {");
	let accepted = |code: &str| warp::analyzer::diagnose(&warp::parse(code)).is_none();
	assert!(accepted("x=ø; if x {x+1} else {2}")); // checked: narrowed inside the branch
	assert!(accepted("x=ø; x=3; x+1"));
}

#[test]
#[ignore = "next"] // a local first assigned ø has no runtime representation yet: Cannot extract numeric value from ø
fn test_optional_local_runs() {
	is!("x=ø; if x {x+1} else {2}", 2);
	is!("x=ø; x=3; x+1", 4);
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
