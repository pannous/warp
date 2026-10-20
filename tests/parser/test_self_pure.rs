//! The pure lane (card pure-runs): WARP_SELF_HOSTED=pure compiles a program with parser.warp, the warp passes,
//! analyzer.warp and emitter.warp alone; each program it compiled writes "pure: ran" to WARP_EMIT_REFUSALS
use std::path::PathBuf;

const RAN_PURE: &str = "pure: ran";

/// The program's output and what the lane recorded for it
fn pure_run(name: &str, code: &str) -> (String, String) {
	lane_run("pure", name, code)
}

/// The program's output in the lane WARP_SELF_HOSTED names, and what the lane recorded for it
fn lane_run(lane: &str, name: &str, code: &str) -> (String, String) {
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	// the target dir and its tmp are shared by every checkout: a run of another at the same time must not write here
	let record = scratch.join(format!("pure_{name}_{}.txt", std::process::id()));
	let _ = std::fs::remove_file(&record);
	let run = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", lane)
		.env("WARP_EMIT_REFUSALS", &record)
		.env("WARP_MODULE_CACHE", scratch.join("self_hosted_stage_cache"))
		.arg(code)
		.output()
		.expect("warp runs");
	let recorded = std::fs::read_to_string(&record).unwrap_or_default();
	let _ = std::fs::remove_file(&record);
	(String::from_utf8_lossy(&run.stdout).into_owned(), recorded)
}

fn runs_pure(name: &str, code: &str, value: &str) {
	let (output, record) = pure_run(name, code);
	assert!(output.contains(value), "{code}: {output}");
	assert_eq!(record.trim(), RAN_PURE, "{code}");
}

/// WARP_SELF_HOSTED=pure-only: no usual compile behind the lane, a program outside the core fails with its refusal
#[test]
fn pure_only_refuses_without_the_rust_compile() {
	assert!(lane_run("pure-only", "only_core", "1 + 2").0.contains('3'));
	let (output, _) = lane_run("pure-only", "only_refused", "nosuchword 3");
	assert!(output.contains("outside the core: pure: the unknown name nosuchword"), "{output}");
}

#[test]
fn the_core_runs_pure() {
	runs_pure("arithmetic", "1 + 2", "3");
	runs_pure("function", "f(x) := x * 2; f(4)", "8");
}

/// `for` lowers to `while` in lib/compiler/for_loops.warp: counted ranges, `to` including its end, walked lists
#[test]
fn for_loops_run_pure() {
	runs_pure("for_range", "s = 0; for i in 0..4 { s += i }; s", "6");
	runs_pure("for_to", "n = 3; t = 0; for i in 1 to n { t += i }; t", "6");
	runs_pure("for_list", "xs = [4, 5, 6]; t = 0; for x in xs { t += x }; t", "15");
	runs_pure("for_nested", "t = 0; for i in 1 to 2 { for j in 1 to 3 { t += i * j } }; t", "18");
}

/// node_matches.warp: an arm of a number, text or character tests the subject equals it
#[test]
fn match_values_run_pure() {
	runs_pure("match_number", "f(x) := match x { 1: 10; 2: 20; _: 0 }; f(2) + f(5)", "20");
	runs_pure("match_text", "g(w) := match w { \"hi\": 1; \"hello\": 2; _: 0 }; g(\"hello\")", "2");
}

/// plain_enums.warp: an enum declares its cases, `Color.Red` and `Color::Red` are the case (card pure-enums)
#[test]
fn enum_cases_run_pure() {
	let color = "enum Color { Red, Green, Blue }";
	let warmth = "warmth(c) := match c { Color.Red: \"warm\"; Color::Blue: \"cold\"; _: \"mild\" }";
	runs_pure("enum_arms", &format!("{color}\n{warmth}\nwarmth(Color.Red) + warmth(Color.Green)"), "warmmild");
	let cents = "cents(coin) := match coin { Penny: 1; Dime: 10; _: 0 }";
	runs_pure(
		"bare_case_arms",
		&format!("enum Coin {{ Penny, Dime }}\n{cents}\ncents(Dime) + cents(Coin.Penny)"),
		"11",
	);
}

/// plain_enums.warp converts a case to its index and back, node_matches.warp takes `other:` as the default arm and a
/// case named like a type word (`Int` of node.warp's Kind) as the case (card enum-leftovers)
#[test]
fn enum_conversions_and_default_arms_run_pure() {
	let coin = "enum Coin { Penny, Nickel, Dime }";
	runs_pure("case_as_int", &format!("{coin}\nCoin.Dime as int"), "2");
	runs_pure("case_as_position", &format!("{coin}\nCoin.Dime as position"), "3");
	runs_pure("int_as_case", &format!("{coin}\n2 as Coin"), "Dime");
	runs_pure("case_call", &format!("{coin}\nCoin(1)"), "Nickel");
	let cost = "cost(c) := match c { Penny: 1; other: 3 }";
	runs_pure("other_arm", &format!("{coin}\n{cost}\ncost(Coin.Dime) * 10 + cost(Coin.Penny)"), "31");
	let name = "name(k) := match k { Int: \"int\"; Float: \"float\"; _: \"node\" }";
	runs_pure(
		"type_named_cases",
		&format!("enum Kind {{ Empty, Int, Float }}\n{name}\nname(Kind.Int) + name(Kind.Empty)"),
		"intnode",
	);
}

/// A call passing the result of a recursive call not known yet, and functions calling each other while one is still
/// read, as parser.warp's expression, atom and continued do
#[test]
fn recursive_results_run_pure() {
	let passed_on =
		"def expression(p, depth) { if depth > 2 { return p }; continued(p, expression(p, depth + 1), depth) }";
	runs_pure(
		"passed_on",
		&format!("{passed_on}; def continued(p, left, depth) := left + p * depth; expression(3, 0)"),
		"12",
	);
	let mutual =
		"def expression(p, depth) { if depth > 2 { return p }; left = atom(p, depth); continued(p, left, depth) }";
	let atom = "def atom(p, depth) := if depth > 1 then p else expression(p, depth + 1)";
	runs_pure(
		"mutual",
		&format!("{mutual}; {atom}; def continued(p, left, depth) := left + p * depth; expression(3, 0)"),
		"12",
	);
}

/// parser.warp reads a name glued to `!` as one word when the source defines it, and a tab between items of a line
/// as a column separator (card parser-reads-2)
#[test]
fn bang_words_and_tab_columns_run_pure() {
	runs_pure(
		"bang_words",
		"def is!(a, b) := a == b\ndef check!(x) := x > 2\ny = 3\n[is!(3, 3), is!(2, 3), check!(5), y != 4]",
		"[yes no yes yes]",
	);
	runs_pure("tab_columns", "row = ['0-1'\t-1]; count(row)", "2");
}

// card parser-reads-string: parser.warp reads `\x41` and `\u{1b}` as their characters
#[test]
fn text_escapes_read_pure() {
	runs_pure("hex_escape", "\"\\x41\\x42\"", "AB");
	runs_pure("unicode_escape", "\"\\e[H\" == \"\\u{1b}[H\"", "yes");
}

/// parser.warp reads the optional field `left?` as Rust does (samples/objects/binary_tree.warp), and emitter.warp
/// builds an instance whose float field gets an int (probes/samples/r.warp's Vec3): both through the self-hosted
/// emitter, nothing refused (card emitter-twin-fails)
#[test]
fn optional_and_float_fields_run_self_hosted() {
	let runs_emitted = |name: &str, code: &str, value: &str| {
		let (output, refusals) = lane_run("1", name, code);
		assert!(output.contains(value), "{code}: {output}");
		assert_eq!(refusals, "", "{code}");
	};
	runs_emitted(
		"optional_field",
		"type T { value: int; left? }\nt = T { value: 2 }\nt.left = T { value: 1 }\nt.left.value",
		"1",
	);
	runs_emitted("float_field", "type V { x: float; y: float }\nv = V { x: 1, y: 2.5 }\nv.x + v.y", "3.5");
}

/// The names a program defines itself (card pure-program-names): `square := it * it` is a function of `it`, a spaced
/// `f 3 4` calls a function of as many parameters, `x := 5` defines a value as Rust's defined_function reads them
#[test]
fn program_names_run_pure() {
	runs_pure("implicit_it", "square := it * it; square 3", "9");
	runs_pure("spaced_call", "f(a, b) := a + b; f 3 4", "7");
	runs_pure("statement_block", "add := { it + 1 }; add 2", "3");
	runs_pure("value_definition", "limit := 3; f(a) := a > limit; f 4", "yes");
}

/// card pure-follows-use: a used file's declarations, and those of the files it uses, reach the pure lane
#[test]
fn used_files_run_pure() {
	let program = concat!(env!("CARGO_MANIFEST_DIR"), "/probes/self_hosting/pure/31_use_file.warp");
	runs_pure("use_file", program, "[3 3]\n8\nyes");
}

/// card pure-lane-use: parser.warp reads the member after a dot as a name, `math.sqrt(16)` no √ (the lane refuses
/// `use math` and the program compiles the usual way, from parser.warp's tree)
#[test]
fn a_member_named_like_a_prefix_word_reads_pure() {
	let (output, _) = pure_run("member_sqrt", "use math\nprint math.sqrt(16)\nprint math.abs(-7)");
	assert!(output.contains("4\n7"), "{output}");
}

/// `name := {…}` holds what the block computes, not the block (card analyzer-reads): `hi()` is 7, an int
#[test]
fn block_definitions_hold_their_value_pure() {
	runs_pure("block_value", "hi := { 3 + 4 }; hi()", "7");
	runs_pure("block_value_added", "hi := { 3 + 4 }; hi() + 1", "8");
}

/// parser.warp reads an integer and a time unit word as the duration Rust's parser makes (card parser-reads-day):
/// `1day` is no `1*day`, and `1 day in h` converts
#[test]
fn durations_read_self_hosted() {
	for lane in ["front", "pure"] {
		for (name, code, value) in [
			("duration_in_unit", "d = 1 day; d in h", "24h"),
			("glued_duration", "1day + 1 day", "2 days"),
			("compound_duration", "1 month 2 days", "1 month 2 days"),
		] {
			let (output, _) = lane_run(lane, name, code);
			assert!(output.contains(value), "{lane} {code}: {output}");
		}
	}
}

/// A call leaving out trailing parameters takes their defaults, in the program's functions and runtime words alike
#[test]
fn defaults_run_pure() {
	runs_pure("default_given", "add(x, y = 2) := x + y; add(1)", "3");
	runs_pure("default_typed", "add(x: int, y: int = 2) := x + y; add(1, 5)", "6");
	runs_pure("default_runtime_word", "xs = [1, 2, 3]; xs.slice(1)", "[2 3]");
}

/// A map that starts as `{}` or `{1: 5}` is keyed by texts, so a number key names an entry (P34, card integer-key)
#[test]
fn number_keys_run_pure() {
	runs_pure("number_key_empty_map", "m = {}; m[1] = 2; m", "{1:2}");
	runs_pure("number_key_runtime", "d = {}; k = 4; d[k] = 7; d[k]", "7");
	runs_pure("number_key_literal", "m = {1: 5}; m[1]", "5");
}

/// List comprehensions lower to the loop that builds the list (card pure-comprehensions), a second `for` nesting in the
/// first clause as Rust's parser reads it
#[test]
fn comprehensions_run_pure() {
	runs_pure("comprehension", "xs = [1, 2, 3, 4]; [x * 2 for x in xs]", "[2 4 6 8]");
	runs_pure("filtered_comprehension", "xs = [1, 2, 3, 4]; [x for x in xs if x > 2]", "[3 4]");
	runs_pure("nested_for_comprehension", "[a + b for a in [1, 2] for b in [10, 20]]", "[11 21 12 22]");
}

/// Only `is` tests a type (card is-operator): parser.warp reads `is` as its own operator, so `3 == int` compares 3
/// with the type int, as Rust's parser reads it; `x is not 4` and `x isn't 4` are `not (x is 4)`
#[test]
fn is_tests_types_and_equality_compares_self_hosted() {
	for lane in ["front", "pure"] {
		for (name, code, value) in [
			("is_type_word", "3 is int", "yes"),
			("equals_type_word", "3 == int", "no"),
			("text_equals_type_word", "\"a\" == text", "no"),
			("unequal_type_word", "3 != int", "yes"),
			("is_value", "x = 3; x is 3", "yes"),
			("is_not_value", "x = 3; x is not 4", "yes"),
			("isnt_value", "x = 4; x isn't 4", "no"),
		] {
			let (output, _) = lane_run(lane, name, code);
			assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == value), "{lane} {code}: {output}");
		}
	}
}

/// lib/compiler/lambdas.warp: iteration words over a lambda or a function loop, other lambdas are closures capturing
/// their variables, a function's name is a value and a variable holding one is called (card pure-lambdas)
#[test]
fn lambdas_run_pure() {
	runs_pure("map_lambda", "xs = [1, 2, 3]; xs.map(x => x * 10)", "[10 20 30]");
	runs_pure("filter_function", "big(x) := x > 1; filter([1, 2, 3], big)", "[2 3]");
	runs_pure("any_lambda", "[1, 2, 3].any(x => x > 2)", "yes");
	runs_pure("fold_lambda", "fold([1, 2, 3], 10, (a, b) => a + b)", "16");
	runs_pure("nested_maps", "xs = [1, 2]; xs.map(x => xs.map(y => x * y))", "[[1 2] [2 4]]");
	runs_pure("captured", "n = 5; apply(f, x) := f(x); apply(x => x + n, 1)", "6");
	runs_pure("function_value", "square(x) := x * x; twice(f, x) := f(f(x)); twice(square, 3)", "81");
	runs_pure("returned_closure", "adder(n) := x => x + n; add2 = adder(2); add2(40)", "42");
	runs_pure(
		"mapped_parameter",
		"scaled(xs, k) := each_item(xs, x => x * k); each_item(xs, f) := xs.map(f); scaled([1, 2], 4)",
		"[4 8]",
	);
}

/// lib/compiler/type_tests.warp: a field named like a type word (`{symbol: …}`) leaves `x is symbol` a type test, only
/// a variable of that name is compared (card pure-lane-symbol)
#[test]
fn type_words_named_by_fields_run_pure() {
	runs_pure("field_named_symbol", "kinds = {symbol: 1}; is_word(n) := n is symbol; is_word(:hello)", "yes");
	runs_pure("variable_named_text", "text = \"a\"; \"a\" is text", "yes");
}

/// lib/compiler/kind_changes.warp: a variable given a value of a kind that does not mix fails with the message of
/// lib/compiler/errors.data, said by the pure lane itself (card pure-diagnostics, P45)
#[test]
fn kind_changes_fail_pure() {
	let (output, _) = lane_run("pure-only", "kind_change", "x = 5; x = [1]; x");
	assert!(output.contains("x was an Int, is given a List: use another name"), "{output}");
	let (output, _) = lane_run("pure-only", "body_kind_change", "f() := (x = 1; x = \"a\"; x); f()");
	assert!(output.contains("x was an Int, is given a Text"), "{output}");
	runs_pure("kinds_that_mix", "x = 1; x = 2.5; x", "2.5");
}

/// card uses-std-modules: a standard module a program uses by name (`use case_tables`, lib/case_tables.warp) brings
/// the definitions the program reaches, its values included, in front of the program
#[test]
fn std_modules_run_pure() {
	runs_pure("use_case_tables", "use case_tables\n#upper_case_ranges", "1818");
}

/// card scope-binding: a used file's plain enum reaches the program (scope.warp's Meaning in parser.warp); a parameter
/// of kind Op, Bracket or Separator holds the text a node carries, and a type variable `T` any value
#[test]
fn enums_of_used_files_and_type_variables_run_pure() {
	let probe = |name: &str| format!("{}/probes/compiler_enums/{name}.warp", env!("CARGO_MANIFEST_DIR"));
	runs_pure("used_enum", &probe("f_used_enum2"), "Function\nyes\nyes");
	runs_pure("used_op", &probe("f_used_op"), "yes\nno");
	runs_pure("enum_parameter", &probe("f_meaning"), "yes\nno\nyes\nyes");
	runs_pure("type_variable", "unchanged(n: T): T := n; print unchanged(3); print unchanged(\"x\")", "3\nx");
}

/// parser.warp's last refusals in the pure lane (card pure-parser-rest): a type test beside a local of that name in
/// another function, `else` on the next line, a prelude alias beside a function of other arity, min and max, a member
/// naming a library word, `/!` of a value known only at run time, the keys of `{}`
#[test]
fn parser_words_run_pure() {
	let probe = |name: &str| format!("{}/probes/parser_rest/{name}.warp", env!("CARGO_MANIFEST_DIR"));
	runs_pure("type_test_beside_local", &probe("is_shadowed"), "no\nyes\n3\nyes");
	runs_pure("else_on_next_line", &probe("else_lines"), "[2]");
	runs_pure("then_on_next_line", &probe("then_line"), "4");
	runs_pure("method_on_next_line", &probe("method_line"), "[2 4]");
	runs_pure("alias_beside_arity", &probe("overload"), "16\n42");
	runs_pure("min_max", &probe("min_max"), "2\n5\n9\n1\n2\n1");
	runs_pure("member_words", &probe("plurals"), "['h' 'é' 'l' 'l' 'o']\n['a' 'b']\n['a' 'b']\n['a' 'b']\n[1 2]\n5");
	runs_pure("floor_quotient_of_nodes", &probe("floor_node"), "3\n3\n2\n2");
	runs_pure("keys_of_nothing", &probe("empty_keys"), "ø\nø");
	runs_pure("empty_lists_join", &probe("empty_joins"), "0\n1\n0");
	runs_pure("arity_overloads", &probe("arity_overloads"), "6\n6\n14\n6\n10\n1");
}
