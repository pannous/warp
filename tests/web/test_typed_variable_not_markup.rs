//! card annotation-html: the typed variable `a:V = …` is no HTML a element, so no accessibility warning; markup
//! assigned to a variable is still checked
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings().into_iter().map(|warning| warning.message).collect()
}

#[test]
fn a_typed_variable_named_like_a_tag_is_no_element() {
	assert_eq!(warnings_of("class V{x:int}\na:V = V(4)\na.x"), Vec::<String>::new());
	assert_eq!(warnings_of("class V{x:int}\nimg:V = V(4)\nimg.x"), Vec::<String>::new());
}

#[test]
fn markup_assigned_to_a_variable_is_still_checked() {
	assert_eq!(warnings_of("page = div{ a{ \"home\" } }"), vec!["a without href: not reachable by keyboard".to_string()]);
}
