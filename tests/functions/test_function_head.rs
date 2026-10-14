// One view of a function definition (card cleanup-definition-view): each form read back as name, parameters,
// form, result type and body
use warp::lowering::{function_head, DefinitionForm};
use warp::operators::Op;
use warp::warp_parser::parse;

/// name, parameters, form, result, body: each written as warp prints it
fn viewed(code: &str) -> (String, Vec<String>, DefinitionForm, Option<String>, String) {
	let statement = parse(code);
	let view = function_head(&statement).unwrap_or_else(|| panic!("no function definition: {code}"));
	let shown = |node: &warp::Node| node.serialize().trim().to_string();
	(
		view.name.to_string(),
		view.parameters().into_iter().map(shown).collect(),
		view.form,
		view.result.map(shown),
		shown(view.body),
	)
}

fn texts(words: &[&str]) -> Vec<String> {
	words.iter().map(|word| word.to_string()).collect()
}

#[test]
fn the_operator_forms() {
	let (name, parameters, form, result, _) = viewed("f(x, y) := x+y");
	assert_eq!(
		(name.as_str(), parameters, form, result),
		("f", texts(&["x", "y"]), DefinitionForm::Operator(Op::Define), None)
	);
	assert_eq!(viewed("f() := 1").1, Vec::<String>::new());
	assert_eq!(viewed("f(x) = x").2, DefinitionForm::Operator(Op::Assign));
	assert_eq!(viewed("f(x): int := x").3.as_deref(), Some("int"));
}

#[test]
fn the_keyword_forms() {
	let (name, parameters, form, result, _) = viewed("def f(x): x*2");
	assert_eq!(
		(name.as_str(), parameters, form, result),
		("f", texts(&["x"]), DefinitionForm::Keyword(Op::Colon), None)
	);
	assert_eq!(viewed("def f(x) := x*2").2, DefinitionForm::Keyword(Op::Define));
	let (_, parameters, form, _, _) = viewed("fun f(x, y) { x }");
	assert_eq!((parameters, form), (texts(&["x", "y"]), DefinitionForm::KeywordBlock));
	let (_, _, form, result, _) = viewed("def f(x) -> int { x }");
	assert_eq!((form, result.as_deref()), (DefinitionForm::Keyword(Op::Arrow), Some("int")));
	assert_eq!(viewed("def f(x): int { x }").3.as_deref(), Some("int"));
	let (_, parameters, form, _, _) = viewed("def (f x) { x }");
	assert_eq!((parameters, form), (texts(&["x"]), DefinitionForm::KeywordSpaced));
}

#[test]
fn no_function_definition() {
	for code in ["x := 3", "x = f(y)", "def x", "3 + 4"] {
		assert!(function_head(&parse(code)).is_none(), "{code}");
	}
}
