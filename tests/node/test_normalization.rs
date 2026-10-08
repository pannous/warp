//! Every non-canonical syntax form emits exactly one hint with the canonical text and the position of the form;
//! canonical forms emit none. The audit matrix lives in notes/normalization_audit.md.
use std::sync::Mutex;
use warp::normalize::*;
use warp::wasm_emitter::eval;
use warp::warp_parser::WarpParser;

/// Style and hint mode are global: tests that read hints or swap the style run one at a time
static GLOBAL_STYLE: Mutex<()> = Mutex::new(());

/// `(original, canonical, position)` of every hint the parser emits for the code
/// The hints under the canonical style (one spelling per form); the default leaves open what the user called legitimate
fn hints_of(code: &str) -> Vec<(String, String, String)> {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_hint_mode(HintMode::Always);
	set_style(Style::canonical());
	let (_, hints) = capture_hints(|| WarpParser::parse(code));
	set_style(Style::default());
	hints.into_iter().map(|hint| (hint.original, hint.canonical, hint.position)).collect()
}

fn hints_with_style(style: Style, code: &str) -> Vec<(String, String, String)> {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_hint_mode(HintMode::Always);
	set_style(style);
	let (_, hints) = capture_hints(|| WarpParser::parse(code));
	set_style(Style::default());
	hints.into_iter().map(|hint| (hint.original, hint.canonical, hint.position)).collect()
}

fn expect_hint(code: &str, original: &str, canonical: &str, position: &str) {
	assert_eq!(hints_of(code), vec![(original.to_string(), canonical.to_string(), position.to_string())], "hints for {code}");
}

fn expect_no_hint(code: &str) {
	assert_eq!(hints_of(code), vec![], "hints for {code}");
}

fn expect_styled_hint(style: Style, code: &str, original: &str, canonical: &str, position: &str) {
	let hints = hints_with_style(style, code);
	assert_eq!(hints, vec![(original.to_string(), canonical.to_string(), position.to_string())], "hints for {code}");
}

fn expect_styled_no_hint(style: Style, code: &str) {
	assert_eq!(hints_with_style(style, code), vec![], "hints for {code}");
}

// ---- generic list types: `ints` is canonical for every element type word

const ELEMENT_TYPE_WORDS: [&str; 8] = ["int", "text", "float", "number", "string", "char", "exact", "rational"];

#[test]
fn test_generic_list_type_hints_plural() {
	for element in ELEMENT_TYPE_WORDS {
		expect_hint(&format!("x:list<{element}>=[]"), &format!("list<{element}>"), &format!("{element}s"), "1:3");
	}
}

#[test]
fn test_of_list_type_hints_plural() {
	for element in ELEMENT_TYPE_WORDS {
		expect_hint(&format!("x:list of {element}=[]"), &format!("list of {element}"), &format!("{element}s"), "1:3");
	}
}

#[test]
fn test_plural_list_type_is_canonical() {
	for element in ELEMENT_TYPE_WORDS {
		expect_no_hint(&format!("x:{element}s=[]"));
	}
}

#[test]
fn test_list_type_hint_with_values_and_position() {
	expect_hint("x:list<int>=[1 2];x", "list<int>", "ints", "1:3");
	expect_hint("x:list of int=[1 2];x", "list of int", "ints", "1:3");
	expect_no_hint("x:ints=[1 2];x");
	expect_hint("y = 1\nxs:list<float>=[1.5 2.5]", "list<float>", "floats", "2:4");
}

#[test]
fn test_list_of_lists_has_no_plural_spelling() {
	expect_no_hint("x:list<list<int>>=[[1],[2]]");
}

#[test]
fn test_list_type_style_generic() {
	let style = Style { list_type: ListTypeStyle::Generic, ..Style::default() };
	expect_styled_hint(style.clone(), "x:ints=[1 2]", "ints", "list<int>", "1:3");
	expect_styled_hint(style.clone(), "x:list of text=[]", "list of text", "list<text>", "1:3");
	expect_styled_hint(style.clone(), "x:list of list of int=[]", "list of list of int", "list<list<int>>", "1:3");
	expect_styled_no_hint(style, "x:list<int>=[1 2]");
}

#[test]
fn test_list_type_style_of() {
	let style = Style { list_type: ListTypeStyle::Of, ..Style::default() };
	expect_styled_hint(style.clone(), "x:floats=[]", "floats", "list of float", "1:3");
	expect_styled_hint(style.clone(), "x:list<int>=[]", "list<int>", "list of int", "1:3");
	expect_styled_no_hint(style, "x:list of int=[1 2]");
}

#[test]
fn test_list_type_hint_is_emitted_once_when_evaluated() {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| eval("x:list<int>=[1 2];x"));
	assert_eq!(hints.len(), 1, "{hints:?}");
	assert_eq!(hints[0].canonical, "ints");
}

// ---- operators

#[test]
fn test_logical_operators() {
	expect_hint("1 && 0", "&&", "and", "1:3");
	expect_hint("1 || 0", "||", "or", "1:3");
	expect_hint("1 & 1", "&", "and", "1:3");
	expect_hint("1 | 0", "|", "or", "1:3");
	expect_hint("!1", "!", "not", "1:1");
	for canonical in ["1 and 1", "1 or 0", "not 1"] {
		expect_no_hint(canonical);
	}
}

#[test]
fn test_logical_operators_symbol_style() {
	let style = Style { logical: LogicalStyle::Symbols, ..Style::default() };
	expect_styled_hint(style.clone(), "1 and 1", "and", "&&", "1:3");
	expect_styled_hint(style.clone(), "1 or 0", "or", "||", "1:3");
	expect_styled_hint(style.clone(), "not 1", "not", "!", "1:1");
	expect_styled_no_hint(style, "1 && 0");
}

#[test]
fn test_power_operator() {
	expect_hint("2 ** 3", "**", "^", "1:3");
	expect_no_hint("2 ^ 3");
	let style = Style { power: PowerStyle::DoubleStar, ..Style::default() };
	expect_styled_hint(style.clone(), "2 ^ 3", "^", "**", "1:3");
	expect_styled_no_hint(style, "2 ** 3");
}

#[test]
fn test_conditional() {
	expect_hint("true ? 1 : 2", "x ? y : z", "if x then y else z", "1:6");
	expect_no_hint("if true then 1 else 2");
	let style = Style { conditional: ConditionalStyle::Ternary, ..Style::default() };
	expect_styled_hint(style.clone(), "if true then 1 else 2", "if x then y else z", "x ? y : z", "1:1");
	expect_styled_no_hint(style, "true ? 1 : 2");
}

// ---- strings

#[test]
fn test_quotes_are_symmetric() {
	expect_hint("'hello'", "'hello'", "\"hello\"", "1:1");
	expect_no_hint("\"hello\"");
	let style = Style { quotes: QuoteStyle::Single, ..Style::canonical() };
	expect_styled_hint(style.clone(), "\"hello\"", "\"hello\"", "'hello'", "1:1");
	expect_styled_no_hint(style, "'hello'");
}

#[test]
fn test_one_character_strings_have_no_canonical_quote() {
	expect_no_hint("'a'");
	expect_no_hint("\"a\"");
}

#[test]
fn test_quote_hint_position_is_the_opening_quote() {
	expect_hint("x = 1; y = 'abc'", "'abc'", "\"abc\"", "1:12");
}

// ---- casts and type words

#[test]
fn test_cast_constructor() {
	expect_hint("float(3)", "float(3)", "3 as float", "1:1");
	expect_hint("char(65)", "char(65)", "65 as char", "1:1");
	expect_hint("int(\"123\")", "int(\"123\")", "\"123\" as int", "1:1");
	expect_no_hint("3 as float");
	expect_no_hint("\"123\" as int");
}

#[test]
fn test_string_type_word_is_hinted_once_with_the_cast() {
	expect_hint("str(42)", "str(42)", "42 as string", "1:1");
	expect_hint("String(4)", "String(4)", "4 as string", "1:1");
	expect_hint("5 as str", "str", "string", "1:1");
	expect_hint("x:str=\"ab\"", "str", "string", "1:3");
	expect_no_hint("42 as string");
	expect_no_hint("x:string=\"ab\"");
}

#[test]
fn test_cast_style_constructor() {
	let style = Style { cast: CastStyle::Constructor, ..Style::canonical() };
	expect_styled_hint(style.clone(), "3 as float", "3 as float", "float(3)", "1:1");
	expect_styled_hint(style.clone(), "5 as str", "5 as str", "string(5)", "1:1");
	expect_styled_no_hint(style, "float(3)");
}

// ---- definitions

#[test]
fn test_variable_definition() {
	expect_hint("let x = 5", "let x = 5", "x := 5", "1:1");
	expect_hint("var x = 5", "var x = 5", "x := 5", "1:1");
	expect_no_hint("x := 5");
	expect_no_hint("x = 5");
}

#[test]
fn test_variable_definition_let_style() {
	let style = Style { var_def: VarStyle::Let, ..Style::default() };
	expect_styled_hint(style.clone(), "x := 5", "x := 5", "let x = 5", "1:1");
	expect_styled_hint(style.clone(), "var x = 5", "var x = 5", "let x = 5", "1:1");
	expect_styled_no_hint(style, "let x = 5");
}

#[test]
fn test_function_definition() {
	// user 2026-10-03: function definitions are "don't care" by default, every form without a hint
	expect_no_hint("def f(x): x*2");
	expect_no_hint("fn f(x) = x*2");
	expect_no_hint("fun f(x) = x*2");
	expect_no_hint("function f(x) { x*2 }");
	expect_no_hint("f(x) := x*2");
}

/// The short `f(x) := …` form as an explicit preference (the default is FunctionStyle::Any)
fn colon_equals_functions() -> Style {
	Style { function_def: FunctionStyle::ColonEquals, ..Style::default() }
}

#[test]
fn test_function_hint_quotes_the_body_form_actually_written() {
	expect_styled_hint(colon_equals_functions(), "def f(x){x*2}", "def f(x) { ... }", "f(x) := ...", "1:1");
	expect_styled_hint(colon_equals_functions(), "def f(x) { x*2 }", "def f(x) { ... }", "f(x) := ...", "1:1");
	expect_styled_hint(colon_equals_functions(), "fn f(x): x*2", "fn f(x): ...", "f(x) := ...", "1:1");
	expect_styled_hint(colon_equals_functions(), "function f(x) = x*2", "function f(x) = ...", "f(x) := ...", "1:1");
	let style = Style { function_def: FunctionStyle::Function, ..Style::default() };
	expect_styled_hint(style, "def f(x): x*2", "def f(x): ...", "function f(x) { ... }", "1:1");
}

#[test]
fn test_function_definition_def_style() {
	let style = Style { function_def: FunctionStyle::Def, ..Style::default() };
	expect_styled_hint(style.clone(), "f(x) := x*2", "f(x) := ...", "def f(x): ...", "1:1");
	expect_styled_hint(style.clone(), "fn f(x) = x*2", "fn f(x) = ...", "def f(x): ...", "1:1");
	expect_styled_no_hint(style, "def f(x): x*2");
}

// ---- indexing

#[test]
fn test_indexing() {
	expect_hint("s = \"abc\"; s[1]", "s[1]", "s#2", "1:12");
	expect_no_hint("s = \"abc\"; s#1");
	expect_hint("s = \"abc\"; s.length()", "s.length()", "#s", "1:12");
	let style = Style { index: IndexStyle::Bracket, ..Style::default() };
	expect_styled_hint(style.clone(), "s = \"abc\"; s#2", "s#2", "s[1]", "1:12");
	expect_styled_no_hint(style, "s = \"abc\"; s[1]");
}

#[test]
fn test_length_prefix_under_bracket_style() {
	let style = Style { index: IndexStyle::Bracket, ..Style::default() };
	expect_styled_hint(style.clone(), "s = \"abc\"; n = #s", "#s", "s.length()", "1:16");
	expect_styled_no_hint(style, "s = \"abc\"; s.length()");
}

#[test]
fn test_define_keyword() {
	expect_styled_hint(colon_equals_functions(), "define f(x): x*2", "define f(x): ...", "f(x) := ...", "1:1");
	expect_no_hint("define f(x): x*2");
	let style = Style { function_def: FunctionStyle::Define, ..Style::default() };
	expect_styled_hint(style.clone(), "def f(x): x*2", "def f(x): ...", "define f(x): ...", "1:1");
	expect_styled_no_hint(style, "define f(x): x*2");
}

#[test]
fn test_each_form_is_hinted_once_when_evaluated() {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_hint_mode(HintMode::Always);
	set_style(Style::canonical());
	for code in ["let x = 5; x", "float(3)", "2 ** 3", "5 as str", "s = \"abc\"; s[1]"] {
		let (_, hints) = capture_hints(|| eval(code));
		assert_eq!(hints.len(), 1, "{code}: {hints:?}");
	}
	set_style(colon_equals_functions());
	let (_, hints) = capture_hints(|| eval("def f(x): x*2; f(2)"));
	set_style(Style::default());
	assert_eq!(hints.len(), 1, "def under an explicit := style: {hints:?}");
}

// ---- forms without a canonical counterpart in the Style struct

#[test]
fn test_ranges_have_no_canonical_form() {
	expect_no_hint("1 to 3");
	expect_no_hint("1..3");
}

#[test]
fn test_hints_are_off_when_the_mode_is_off() {
	let _guard = GLOBAL_STYLE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	set_hint_mode(HintMode::Off);
	let (_, hints) = capture_hints(|| WarpParser::parse("1 && 0"));
	set_hint_mode(HintMode::Always);
	assert_eq!(hints, vec![]);
}
