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

/// A type's name is a value, `k = int` prints `class int` (decision type-text); a parameter, a result or a variable
/// declared `type` holds one, a map entry named by it is found, and a match arm of a type word tests the subject's type
/// (card pure-lane-reads)
#[test]
fn type_values_run_pure() {
	runs_pure("type_value", "k = int; k", "class int");
	runs_pure("type_value_member", "k = text; [int, text] contains k", "yes");
	runs_pure("type_value_parameter", "wider(a: type, b: type): type := if [a, b] contains float then float else int; wider(int, float)", "class float");
	runs_pure("type_value_key", "held = {int: 1, float: 2}; held[float]", "2");
	runs_pure("type_arm", "name(t: any): text := match t { int: \"whole\"; _: \"other\" }; name(3)", "whole");
}
