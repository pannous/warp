//! "I don't care. Both are fine. There should not be a hint." (user, 2026-10-03): every style axis can be set to `Any`,
//! which accepts all forms of that axis without a hint; function definitions are `Any` by default
use crate::is;
use warp::normalize::*;
use warp::wasm_emitter::eval;
use warp::wasp_parser::WaspParser;

fn hints_with(style: Style, code: &str) -> Vec<String> {
	set_hint_mode(HintMode::Always);
	set_style(style);
	let (_, hints) = capture_hints(|| WaspParser::parse(code));
	set_style(Style::default());
	hints.into_iter().map(|hint| hint.original).collect()
}

#[test]
fn every_function_definition_form_compiles_without_a_hint_by_default() {
	for code in ["f(x) := x*2", "def f(x): x*2", "fn f(x) = x*2", "fun f(x) = x*2", "function f(x) { x*2 }", "define f(x): x*2"] {
		assert_eq!(hints_with(Style::default(), code), Vec::<String>::new(), "{code}");
		is!(&format!("{code}; f(3)"), 6);
	}
}

#[test]
fn an_explicit_function_style_still_hints() {
	let style = Style { function_def: FunctionStyle::ColonEquals, ..Style::default() };
	assert_eq!(hints_with(style, "def f(x): x*2"), vec!["def f(x): ...".to_string()]);
}

#[test]
fn any_axis_can_stop_caring() {
	let relaxed = Style {
		logical: LogicalStyle::Any,
		power: PowerStyle::Any,
		quotes: QuoteStyle::Any,
		index: IndexStyle::Any,
		var_def: VarStyle::Any,
		cast: CastStyle::Any,
		conditional: ConditionalStyle::Any,
		list_type: ListTypeStyle::Any,
		..Style::default()
	};
	for code in ["1 && 0", "!1", "2 ** 3", "'abc'", "s = \"abc\"; s[1]", "s = \"abc\"; s.length()", "var x = 5", "float(3)", "1 ? 2 : 3", "x: list<int> = [1]"] {
		assert_eq!(hints_with(relaxed.clone(), code), Vec::<String>::new(), "{code}");
	}
	assert!(!hints_with(Style::default(), "1 && 0").is_empty(), "the default still cares about logical operators");
	let _ = eval("1");
}
