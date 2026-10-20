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

/// lib/compiler/parser.warp reads `for each x in xs` and `each x in xs` as `for x in xs`; a loop variable named `each`
/// stays one (card pure-error-undefined)
#[test]
fn for_each_loops_run_pure() {
	let basket = "basket = [{price: 2} {price: 3}]; t = 0;";
	runs_pure("for_each", &format!("{basket} for each item in basket {{ t = t + item.price }}; t"), "5");
	runs_pure("each_in", &format!("{basket} each item in basket {{ t = t + item.price }}; t"), "5");
	runs_pure("variable_each", "t = 0; for each in [10, 20] { t = t + each }; t", "30");
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

/// What the lib bench found parser.warp failing on in lib/ (scripts/lib_bench.sh): a keyword naming a key or a member
/// (`{if: 4}`, `op.if`, emitter.warp), a membership word assigned (`alias contains = …`, prelude.warp), an operator
/// alias line, `&f` and `∛x` (math.warp), the degree sign (`20°C`, units.warp)
#[test]
fn lib_forms_read_self_hosted() {
	for lane in ["front", "pure"] {
		for (name, code, value) in [
			("keyword_key", "m = {if: 4, x: 1}; m[\"if\"]", "4"),
			("keyword_member", "op = {if: 4, else: 5}; op.if + op.else", "9"),
			("assigned_membership_word", "xs = [1, 2]; includes = 3; includes + #xs", "5"),
			("operator_alias_line", "operator × := *\n2 × 3", "6"),
			("function_reference_sign", "f(x) := x + 1; g = &f; g(2)", "3"),
			("cube_root_glyph", "∛27", "3"),
			("degree_temperature", "20°C is temperature", "yes"),
			("degree_variable", "x = 180; x° > 3.14", "yes"),
		] {
			let (output, _) = lane_run(lane, name, code);
			assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == value), "{lane} {code}: {output}");
		}
	}
}

/// parser.warp consumes every Unicode character by its class (card unicode-glyphs): arrow and declared glyph operators,
/// operator aliases (lib/math.warp), free symbols as values, a quote carrying a combining mark
#[test]
fn unicode_glyphs_read_self_hosted() {
	for lane in ["front", "pure"] {
		for (name, code, value) in [
			("arrow_down", "8↓2", "3"),
			("declared_glyph", "infix operator ⊕(a: int, b: int) := a * 10 + b; 4 ⊕ 2", "42"),
			("alias_dot", "2 · 3", "6"),
			("alias_and", "yes ∧ no", "no"),
			("alias_element", "2 ∈ [1, 2]", "yes"),
			("free_symbol_value", "x = ↯\nx", "↯"),
			("free_symbols_listed", "[↯ ↯]", "[↯ ↯]"),
			("technical_symbol_value", "x = ⌘\nx", "⌘"),
			("quote_with_mark", "x = \"\u{305}\"\nx.length", "1"),
		] {
			let (output, _) = lane_run(lane, name, code);
			assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == value), "{lane} {code}: {output}");
		}
	}
}

/// lib/compiler/kind_changes.warp: a variable given a value of a kind that does not mix fails with the message of
/// A type word after `global` types the declared name, every type word Rust knows (`long`, `double`), not only `int`
#[test]
fn global_type_words_read_self_hosted() {
	for lane in ["front", "pure"] {
		for (name, code, value) in
			[("global_long", "global long k=7; k", "7"), ("global_double", "global double d = 1.5; d + 1", "2.5")]
		{
			let (output, _) = lane_run(lane, name, code);
			assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == value), "{lane} {code}: {output}");
		}
	}
}

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

/// lib/compiler/emitter.warp: an instance leaving out fields gives them their declared defaults, an optional field ø
/// (card pure-field-defaults)
#[test]
fn field_defaults_run_pure() {
	runs_pure("call_defaults", "class point{x:int=0; y:int=3}; point(5)", "point{x:5 y:3}");
	runs_pure("brace_defaults", "class counter{n=0; step:int=1}; c = counter{}; c.n + c.step", "1");
	runs_pure("optional_field", "class opt { a: int; b? }; opt(4).a", "4");
}

/// lib/compiler/emitter.warp: a named argument `name = value` takes its parameter's or field's place (P37), a left-out
/// one its default (card pure-named-arguments)
#[test]
fn named_arguments_run_pure() {
	runs_pure("named_call", "f(a, b) := a * 10 + b; f(b=2, a=7)", "72");
	runs_pure("named_instance", "class P { x: int; y: int }; p = P(y=2, x=7); p.x * 10 + p.y", "72");
	runs_pure("named_and_default", "class P { x: int; y: int = 5 }; P(x=1)", "P{x:1 y:5}");
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
	runs_pure("arity_overloads", &probe("arity_overloads"), "6\n6\n14\n6\n10\n1\n10\n12");
	runs_pure("type_of_is", &probe("type_of_is"), "yes\nno\nyes");
	runs_pure("match_arm_forms", &probe("match_arms"), "x\nfirst\nb\nother");
	runs_pure("coalesce", &probe("coalesce"), "2\n1\n-2");
	runs_pure("recursive_join", &probe("recursive_join"), "[3 2 1]\n[2 1 0]");
	runs_pure("type_module_words", &probe("type_module_words"), "[1 2 3]\n[1 2]\n7");
	runs_pure("nested_case_patterns", &probe("nested_patterns"), "label\n2\nother");
	runs_pure("error_values", &probe("error_value"), "1\ntoo many\nyes");
	let chain = |name: &str| format!("{}/probes/pure_chains/{name}.warp", env!("CARGO_MANIFEST_DIR"));
	runs_pure("library_word_mapped", &chain("slice_named"), "['b']\n[2 1]");
	runs_pure("chained_iterations", &chain("sorted_then_mapped"), "[\"use a\" \"use b\"]\n[4 6]");
	runs_pure("module_directory", &chain("home_folder/own_folder"), "home_folder\nhelpers");
	runs_pure("read_files", &chain("files/read_file"), "zweite Zeile ü\n26\nnone");
	runs_pure("loop_variable_scope", &chain("slice_loops"), "['b' 'c']");
	runs_pure("loop_of_functions", &chain("loop_of_functions"), "6\n4");
	runs_pure("library_word_method_in_lambda", &chain("slice_method"), "[[\"operator_+\" 'y'] ['c' 'z']]");
	runs_pure("pending_results", &chain("pending_results"), "[\"int\" 4]\n[5 5 0 5 5 0 0]");
	runs_pure("map_get_default", &chain("map_get_default"), "ø\n1\n7");
	runs_pure("own_special_forms", &chain("own_special_forms"), "3\n12\n2\n0");
	runs_pure("bit_operators", &chain("bit_operators"), "6\n-5\n48\n-1\n6");
	runs_pure("ord", &chain("ord"), "97\n252\n122\n98");
	runs_pure("std_module_of_a_used_file", &chain("std_in_used/main"), "6");
	runs_pure("text_bytes", &chain("text_bytes"), "[97 195 188]\n[121]\n2\n2\n16\n97\n98");
	runs_pure("float_bits", &chain("float_bits"), "4609434218613702656\n-4625196817309499392");
	runs_pure("char_as_text", &chain("char_as_text"), "?\none\nab");
	runs_pure("colon_named_arguments", &chain("colon_named_arguments"), "3\ny\n72");
	runs_pure("arrow_match_arms", &chain("arrow_arms"), "two\nbig\nfew");
}
