// card g-_alg (wiki/mark.md): inside a tag's block `label(for:pwd)` is the tag `label{for:pwd}` and
// `label(for:pwd):"Password"` the tag `label{for:pwd "Password"}`; samples/html.wasp, the playground's HTML demo, runs
use crate::common::fails_with;
use crate::eq;
use warp::wasm_emitter::eval;

#[test]
fn a_tag_takes_its_attributes_in_parentheses() {
	eq!(eval("html{ div{ label(for:pwd):\"Password\" } }"), eval("html{ div{ label{for:pwd, \"Password\"} } }"));
	eq!(eval("div{ label(for:pwd a:b) }"), eval("div{ label{for:pwd a:b} }"));
	runs("samples/html.wasp");
}

// card g-_bGo: the playground's HTML DSL demo runs; its list items are tags li{…}, children as repeated XML elements
#[test]
fn the_html_dsl_sample_runs() {
	runs("samples/html_dsl.wasp");
}

fn runs(sample: &str) {
	let value = eval(sample);
	assert!(value.first_error().is_none(), "{sample}: {}", value.serialize());
}

#[test]
fn a_call_outside_tags_or_of_a_function_stays_a_call() {
	fails_with("label(for:pwd)", "undefined function: label");
	eq!(eval("label(x) := x + 1; div{ label(x:1) }"), eval("div{ 2 }"));
}
