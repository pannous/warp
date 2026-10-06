//! D10 (user, un-parked): "Dispatch on return type". The expected result type of a call picks the overload; without one
//! the first-declared overload is taken with a got-it warning naming the explicit form (notes/dispatch.md).

use warp::diagnostic::take_warnings;
use crate::is;

const RENDER: &str = "class pdf{body}; class docx{body}; render(t:text):pdf := pdf(\"%PDF \" + t); render(t:text):docx := docx(\"<w:t>\" + t); ";
const INFERRED: &str = "class pdf{body}; class docx{body}; render(t:text) := pdf(\"%PDF \" + t); render(t:text) := docx(\"<w:t>\" + t); ";

fn with(prelude: &str, code: &str) -> String {
	format!("{prelude}{code}")
}

fn return_type_warnings() -> Vec<String> {
	take_warnings().into_iter().filter(|warning| warning.message.contains("variants returning")).map(|warning| format!("{} fix: {:?}", warning.message, warning.fix)).collect()
}

#[test]
fn test_as_picks_the_overload() {
	is!(&with(RENDER, "x = render \"hi\" as pdf; x.body"), "%PDF hi");
	is!(&with(RENDER, "x = render \"hi\" as docx; x.body"), "<w:t>hi");
	is!(&with(RENDER, "x = (render \"hi\") as docx; x.body"), "<w:t>hi");
}

#[test]
fn test_a_typed_declaration_picks_the_overload() {
	is!(&with(RENDER, "docx example = render \"hi\"; example.body"), "<w:t>hi");
	is!(&with(RENDER, "example:docx = render \"hi\"; example.body"), "<w:t>hi");
	is!(&with(RENDER, "pdf example = render \"hi\"; example.body"), "%PDF hi");
}

#[test]
fn test_a_typed_parameter_picks_the_overload() {
	is!(&with(RENDER, "words(d:docx) := d.body; words(render(\"hi\"))"), "<w:t>hi");
}

#[test]
fn test_the_result_type_can_be_inferred_from_the_body() {
	is!(&with(INFERRED, "x = render \"hi\" as docx; x.body"), "<w:t>hi");
	is!(&with(INFERRED, "pdf p = render \"hi\"; p.body"), "%PDF hi");
}

#[test]
fn test_no_expected_type_takes_the_first_overload_with_a_warning() {
	take_warnings();
	is!(&with(RENDER, "x = render \"hi\"; x.body"), "%PDF hi");
	let warnings = return_type_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("pdf, docx") && warning.contains("as pdf")), "{warnings:?}");
}

#[test]
fn test_a_picked_overload_warns_nothing() {
	take_warnings();
	is!(&with(RENDER, "x = render \"hi\" as docx; x.body"), "<w:t>hi");
	assert!(return_type_warnings().is_empty());
}

#[test]
fn test_a_single_definition_with_a_result_type() {
	is!("class pdf{body}; make(t):pdf := pdf(t); m = make(\"a\"); m.body", "a");
	is!("class pdf{body}; make(t) := pdf(t); m = make(\"a\"); m.body", "a");
}
