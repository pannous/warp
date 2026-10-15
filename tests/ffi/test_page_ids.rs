//! Page ids as names (card page-ids, user 2026-10-15): an id in the program's own markup is a name, plain `who` the
//! element and `$who` its value (text, number, yes/no, text content); `element(x)` for any other id; a clash of an id
//! with a variable, function or word is a compile-time error naming both
use crate::common::fails_with;
use warp::normalize::capture_hints;

const INPUT: &str = "input{ id: \"who\" }\n";

fn compiles(code: &str) {
	warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}"));
}

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).unwrap_or_else(|error| panic!("{code} does not lower: {error:?}")).serialize()
}

/// The hints compiling `code` gives, as (written, preferred)
fn hints_of(code: &str) -> Vec<(String, String)> {
	let (_, hints) = capture_hints(|| warp::wasm_emitter::compile(code));
	hints.into_iter().map(|hint| (hint.original, hint.canonical)).collect()
}

#[test]
fn a_markup_id_is_the_element() {
	compiles(&format!("{INPUT}button{{ on click {{ who.focus() }} \"Go\" }}"));
	compiles(&format!("{INPUT}who.placeholder = \"name\""));
	fails_with(&format!("{INPUT}who.focuss()"), "did you mean focus");
}

#[test]
fn a_kebab_id_is_a_name_too() {
	compiles("input{ id: \"user-name\" }\nuser-name.focus()\ngreeting = \"Hi \" + $user-name");
}

#[test]
fn an_unquoted_id_is_one_too() {
	compiles("input{ type:email id:email }\nemail.focus()");
}

#[test]
fn a_dollar_id_is_the_elements_value() {
	assert!(lowered(&format!("{INPUT}greeting = \"Hello \" + $who")).contains("value"));
	assert!(lowered("input{ id: \"agree\" type: \"checkbox\" }\nok = $agree").contains("checked"));
	assert!(lowered("input{ id: \"age\" type: \"number\" }\nn = $age").contains("valueAsNumber"));
	assert!(lowered("p{ id: \"note\" \"hi\" }\nt = \"note: \" + $note").contains("textContent"));
}

#[test]
fn assigning_a_dollar_id_writes_the_value() {
	assert!(lowered(&format!("{INPUT}$who = \"Ann\"")).contains("value"));
	assert!(lowered("input{ id: \"agree\" type: \"checkbox\" }\n$agree = yes").contains("checked"));
}

#[test]
fn an_id_clashing_with_a_variable_is_an_error_naming_both() {
	let code = "input{ id: \"name\" }\nname = $name";
	fails_with(code, "the page element of id name");
	fails_with(code, "the variable name");
	fails_with(code, "write `input{ bind: name }` and drop the id");
}

#[test]
fn an_id_clashing_with_a_function_or_word_is_an_error() {
	fails_with("input{ id: \"greet\" }\ngreet(x) := x", "the function greet");
	fails_with("p{ id: \"print\" }\nprint", "the word print");
	compiles("section{ id: \"map\" }\nxs = [1 2].map(x => x * 2)"); // a word the program doesn't name plainly
}

#[test]
fn the_first_dollar_field_hints_at_bind() {
	let hints = hints_of(&format!("{INPUT}a = $who\nb = $who"));
	let bind_hints = hints.iter().filter(|(_, preferred)| preferred == "input{ bind: who }").count();
	assert_eq!(bind_hints, 1, "{hints:?}");
}

#[test]
fn element_finds_any_id() {
	compiles("element(\"result\").innerHTML = \"<b>x</b>\"");
	compiles("name = \"out\"\nelement(name).textContent = \"x\"");
	fails_with("element(\"result\").innerHtml = \"x\"", "did you mean innerHTML");
}

#[test]
fn document_and_window_are_plain_names() {
	compiles("t = document.title");
	compiles("w = window.innerWidth");
	fails_with("document.titel", "did you mean title");
}

#[test]
fn the_old_dollar_forms_work_with_a_hint() {
	let hints = hints_of(&format!("{INPUT}a = $who.value"));
	assert!(hints.contains(&("$who.value".into(), "$who".into())), "{hints:?}");
	let hints = hints_of(&format!("{INPUT}$who.focus()"));
	assert!(hints.contains(&("$who".into(), "who".into())), "{hints:?}");
	let hints = hints_of("t = $document.title");
	assert!(hints.contains(&("$document".into(), "document".into())), "{hints:?}");
	let hints = hints_of("$xyz.innerHTML = \"x\"");
	assert!(hints.contains(&("$xyz".into(), "element(\"xyz\")".into())), "{hints:?}");
}

#[test]
fn jquery_gives_a_hint_naming_the_warp_forms() {
	let hints = hints_of(&format!("{INPUT}a = $(\"#who\").value"));
	assert!(hints.contains(&("$(\"#who\").value".into(), "$who".into())), "{hints:?}");
	let hints = hints_of(&format!("{INPUT}$(\"who\").focus()"));
	assert!(hints.contains(&("$(\"who\")".into(), "who".into())), "{hints:?}");
	fails_with("$(\".note p\").focus()", "document.querySelector");
}

#[test]
fn an_attribute_naming_an_id_stays_its_name() {
	compiles("div{ label{ for:email \"Email\" } input{ type:email id:email } }");
}

#[test]
fn a_dollar_outside_warp_code_stays_text() {
	assert!(lowered(&format!("{INPUT}p{{ \"costs $5, $who\" }}")).contains("costs $5, $who"));
	assert!(lowered(&format!("{INPUT}p{{ 'hello $who' }}")).contains("hello $who"));
}
