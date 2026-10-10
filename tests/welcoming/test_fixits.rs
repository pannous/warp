//! Fixes (user, 2026-10-05: "suggest fixes and a mechanism to apply the fixes in the web demo for all the warnings"):
//! every warning and ambiguity error offers each reading the user might have meant as an edit of the source
//! (notes/fixits.md). Each test applies a fix to the source and checks that the result compiles, without a warning,
//! to the reading the fix names.
use warp::diagnostic::{take_error_diagnostics, take_warnings, with_warning_mode, Diagnostic, WarningMode};
use warp::fixits::fixed;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

const SQUARE: &str = "square:=it*it; ";
const OVERLOADS: &str = "class pdf{body}; class docx{body}; render(t:text) := pdf(\"%PDF \" + t); render(t:text) := docx(\"<w:t>\" + t); ";

/// The warnings of `code`, and its errors with fixes
fn diagnostics(code: &str) -> Vec<Diagnostic> {
	take_warnings();
	take_error_diagnostics();
	eval(code);
	let mut all = take_warnings();
	all.extend(take_error_diagnostics());
	all
}

/// `code` with the fix applied whose meaning mentions `meaning`
fn fixed_by(code: &str, meaning: &str) -> String {
	let all = diagnostics(code);
	let found = all.iter().find_map(|diagnostic| diagnostic.fixes.iter().find(|fix| fix.meaning.contains(meaning)).map(|fix| (diagnostic, fix)));
	let (diagnostic, fix) = found.unwrap_or_else(|| panic!("no fix meaning {meaning:?} for {code}: {all:?}"));
	fixed(code, diagnostic.line, diagnostic.column, fix).unwrap_or_else(|| panic!("{fix:?} does not find its text in {code}"))
}

/// The fix whose meaning mentions `meaning` turns `code` into a program whose value is `expected`, warning-free
fn assert_fix(code: &str, meaning: &str, expected: &str) {
	let source = fixed_by(code, meaning);
	let value = with_warning_mode(WarningMode::Error, || eval(&source));
	assert_eq!(value.serialize().trim(), expected, "{code} fixed to {meaning}: {source}");
}

#[test]
fn range_end_fixes() {
	assert_fix("x=0; for i in 1 upto 4 {x+=i}; x", "exclusive", "6");
	assert_fix("x=0; for i in 1 upto 4 {x+=i}; x", "inclusive", "10");
	assert_fix("n=4; x=0; for i in 0..n-1 {x+=i}; x", "exclusive", "3");
	assert_fix("n=4; x=0; for i in 0..n-1 {x+=i}; x", "inclusive", "6");
}

#[test]
fn signed_operand_fixes() {
	assert_fix("1 -1", "the list", "1 -1"); // `1 (-1)`: a list without brackets prints so
	assert_fix("[1 -1 2]", "the list", "[1 -1 2]");
	assert_fix("1 -1", "arithmetic", "0");
}

#[test]
fn local_or_global_fixes() {
	assert_fix("n=0; def f(x){n=5;x}; f(3); n", "a new local", "0");
	assert_fix("n=0; def f(x){n=5;x}; f(3); n", "the main-level", "5");
}

#[test]
fn list_operator_fixes() {
	assert_fix("[2]*3", "repeat", "[2 2 2]");
	assert_fix("[2]*3", "multiply", "[6]");
	assert_fix("[1 2 3]+4", "append", "[1 2 3 4]");
	assert_fix("[1 2 3]+4", "add to each", "[5 6 7]");
	assert_fix("xs=[1 2]; xs.insert(0, 4); xs", "position first", "[4 1 2]");
	assert_fix("xs=[1 2]; xs.insert(0, 4); xs", "value first", "[1 2 0]");
}

#[test]
fn bare_list_fixes() {
	assert_fix("a=1 2 3; a", "a list", "[1 2 3]");
	assert_fix("a=1 2 3; a", "separate statements", "1");
}

#[test]
fn suffix_word_fixes() {
	assert_fix(&format!("{SQUARE}1+2 squared"), "binds to 2", "5");
	assert_fix(&format!("{SQUARE}1+2 squared"), "applies to", "9");
}

#[test]
fn braceless_call_fixes() {
	assert_fix(&format!("{SQUARE}square 3 + square 4"), "of 3 only", "25");
	assert_fix(&format!("{SQUARE}square 3 + square 4"), "of the whole", "361");
	assert_fix(&format!("{SQUARE}square 3 + square(4)"), "only the first operand", "25"); // P68's got-it warning
	assert_fix(&format!("{SQUARE}square 3 + square(4)"), "the whole expression", "361");
}

#[test]
fn word_fixes() {
	assert_fix("total=5; totl", "the name total", "5");
	assert_fix("total=5; totl", "the symbol", "totl");
	assert_fix(&format!("{OVERLOADS}x = render \"hi\"; x.body"), "docx", "\"<w:t>hi\"");
	assert_fix("a=1;b=2; x : a+b; x", "its value", "3");
}

#[test]
fn operator_ambiguity_fixes() {
	assert_eq!(eval("true + true"), 2); // P195 (user): no fix needed, a bool counts as 1/0
	assert_fix("2==2==1", "both equalities", "no");
	assert_fix("3 & 4 == 4", "comparison first", "1");
	assert_fix("\"a\" as int", "code point", "97");
	assert_fix("c=1; c>0 and 0 or 5", "whenever", "0");
	assert_fix("2 * 1.5 as int", "convert the whole", "3");
	assert_fix("-7 % 3", "truncated", "-1");
	assert_fix("(1, 2) == 1, 2", "all the values", "yes");
	assert_fix("xs=[2,5,7]; xs#1..3", "the slice", "[2 5]");
}

#[test]
fn loop_it_fix() {
	assert_fix("f:={s=it; for 1..3 {s+=it}; s}; f(10)", "function's `it`", "30");
	assert_fix("f:={s=it; for i in [5 6] {s+=it}; s}; f(10)", "function's `it`", "30"); // #23: `for i in` binds it too
}

/// The hint about `original` in `code` offers its preferred form, which gives `expected`
fn assert_hint_fix(code: &str, original: &str, expected: &str) {
	let (_, hints) = capture_hints(|| eval(code));
	let hint = hints.iter().find(|hint| hint.original == original).unwrap_or_else(|| panic!("{hints:?}"));
	let (line, column) = hint.line_and_column();
	let source = fixed(code, line, column, &hint.fix().expect("a rewrite")).expect("the hint's text is in the source");
	assert_eq!(eval(&source).serialize(), expected, "{source}");
}

#[test]
fn a_hint_offers_its_preferred_form() {
	assert_hint_fix("x = 2; y = x ** 3; y", "**", "8");
	assert_hint_fix("'a' as float", "'a' as float", "97"); // P74: a character is no number, codepoint('a') is
}

#[test]
fn the_page_gets_each_fix_as_an_edit() {
	let report = warp::web::evaluate(&format!("{SQUARE}square 3 + square 4"), Default::default());
	let fixes = report["errors"][0]["fixes"].as_array().unwrap_or_else(|| panic!("{report}"));
	assert_eq!(fixes.len(), 2, "{report}");
	assert_eq!(fixes[0]["label"], "I meant: square(3) + square(4)");
	assert_eq!((fixes[0]["start"].as_u64(), fixes[0]["end"].as_u64()), (Some(SQUARE.len() as u64), Some(SQUARE.len() as u64 + 19)));
	let warned = warp::web::evaluate("x=0; for i in 1 upto 4 {x+=i}; x", Default::default());
	assert_eq!(warned["warnings"][0]["topic"], "upto");
	assert_eq!(warned["warnings"][0]["fixes"][1]["replacement"], "...");
}

#[test]
fn codepoint_is_the_preferred_name() {
	// P74 (user, 2026-10-05): codepoint(c) in hints, fixes and docs; ord and ordinal stay synonyms
	crate::is!("codepoint('x') as float", 120.0);
	crate::is!("(codepoint('x') as float) / 8", 15.0);
	crate::is!("ord('x') + ordinal('x')", 240);
	let (_, hints) = capture_hints(|| eval("codepoint('x')"));
	assert!(hints.is_empty(), "codepoint(c) is no type constructor: {hints:?}");
}

#[test]
fn the_page_says_got_it_for_one_expression() {
	let code = "x=0; for i in 1 upto 4 {x+=i}; for i in 1 upto 3 {x+=i}; x";
	let report = warp::web::evaluate(code, Default::default());
	let first = report["warnings"][0]["expression_key"].as_str().unwrap_or_else(|| panic!("{report}")).to_string();
	assert!(first.starts_with("upto@"), "{first}");
	assert_eq!(report["got_it"][0]["topic"], "upto");
	let silenced = warp::web::evaluate(code, [first].into_iter().collect());
	let warnings = silenced["warnings"].as_array().unwrap();
	assert_eq!(warnings.len(), 1, "the other upto still warns: {silenced}");
	assert!(warnings[0]["message"].as_str().unwrap().contains("upto 3"), "{silenced}");
	assert_eq!(silenced["value"], report["value"]);
}

// "No fix yet" (notes/fixits.md, card g-qU1o): a kebab data key whose parts are variables (user, P81) and the filter loops

#[test]
fn kebab_key_fixes() {
	assert_fix("a=5; b=1; a-b:2", "the data key a-b", "a-b:2");
	assert_fix("a=5; b=1; x={a-b:2}; x.a-b", "the data key a-b", "2"); // the member read already means the key
	assert_fix("a=5; b=1; a-b:2; a-b", "the subtraction", "4");
	assert_fix("a=5; b=1; a-b:2; a-b", "renamed a_b", "2");
	assert_fix("a=5; b=1; a-b:2; a-b + a-b", "renamed a_b", "4");
}

#[test]
fn filter_loop_fixes() {
	assert_fix("xs=[1,\"a\",2]; s=0; for int in xs: s += it\ns", "filter the items", "3");
	assert_fix("xs=[1,5,3]; s=0; for (it>2) in xs: s += it\ns", "filter the items", "8");
	assert_fix("type Friend {name}; xs=[Friend{name:\"a\"}, 3]; n=0; for Friend in xs: n += 1\nn", "filter the items", "1");
}

#[test]
fn the_page_gets_every_edit_of_a_fix() {
	let code = "a=5; b=1; a-b:2; a-b";
	let report = warp::web::evaluate(code, Default::default());
	let renamed = report["warnings"][0]["fixes"].as_array().unwrap_or_else(|| panic!("{report}")).iter()
		.find(|fix| fix["meaning"].as_str().unwrap_or_default().contains("renamed")).unwrap_or_else(|| panic!("{report}"));
	let edits = renamed["edits"].as_array().unwrap();
	assert_eq!(edits.len(), 2, "{renamed}");
	// the last in the text first: the read at the end, then the key
	assert_eq!((edits[0]["start"].as_u64(), edits[1]["start"].as_u64()), (Some(code.len() as u64 - 3), Some(10)));
}

#[test]
fn ambiguous_call_comparison_fixes() {
	assert_fix("say(x) := x\nsay 3 == 4", "the call compared", "no");
	assert_fix("say(x) := x\nsay 3 == 4", "the comparison as the argument", "no");
	assert_fix("say(x) := x\nsay 3 == 3", "the call compared", "yes");
	assert_eq!(fixed_by("say(x) := x\nsay 3 == 3", "the comparison as the argument"), "say(x) := x\nsay(3 == 3)");
}

/// card more-fixes (user): an error with an evident fix offers it; a missing closer is added where it was left open
#[test]
fn missing_closer_fixes() {
	assert_eq!(fixed_by("x = \"abc", "the closing \""), "x = \"abc\"");
	assert_eq!(fixed_by("x = \"abc\ny = 2", "the closing \""), "x = \"abc\"\ny = 2");
	assert_fix("f(x) := x + 1\nf(1", "the closing )", "2");
	assert_fix("xs = [1, 2\n", "the closing ]", "[1 2]");
	assert_fix("‖-3", "the closing ‖", "3");
}

#[test]
fn misspelled_name_fixes() {
	assert_fix("count = 3; coutn + 1", "the name count", "4");
	assert_fix("square:=it*it; sqaure(3)", "the name square", "9");
}

#[test]
fn written_form_fixes() {
	assert_fix("(7 // 2)", "floor division", "3");
	assert_eq!(fixed_by("x is 5", "a definition"), "x be 5");
	assert_eq!(fixed_by("t = \\alpha", "the uniscript entity"), "t = \\:alpha");
}

#[test]
fn misspelled_parameter_and_type_fixes() {
	assert_fix("f(name, age) := age; f(nmae: \"a\", age: 3)", "the parameter name", "3");
	assert_fix("f(x: flaot) := x * 2; f(1.5)", "the type float", "3");
}

#[test]
fn declared_type_mismatch_fixes() {
	assert_fix("x:int = 2.5; x", "the int of the value", "2");
	assert_fix("x:int = 2.5; x", "x holds float", "2.5");
	assert_eq!(fixed_by("x:int = ø", "allow ø"), "x:int? = ø");
}

#[test]
fn unused_copy_fix() {
	assert_fix("x = [3, 1, 2]; sort x; x", "change x", "[1 2 3]");
}
