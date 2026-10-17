//! Card render-as (Features.md, D10): the English phrase `to render text as pdf: …` names its result type after `as`,
//! and `(render "hello") as pdf` calls it, with one definition as with overloads (notes/dispatch.md).

use crate::is;

const PHRASES: &str = "class pdf{body:text}; class docx{body:text}; to render text as pdf: pdf(\"%PDF \" + text); to render text as docx: docx(\"<w>\" + text); ";

#[test]
fn a_phrase_names_its_result_after_as() {
	is!("class pdf{body:text}; to render text as pdf: pdf(text); x = (render \"hello\") as pdf; x.body", "hello");
	is!("class pdf{body:text}; to render text as pdf: pdf(text); type of ((render \"hello\") as pdf)", "pdf");
}

#[test]
fn as_names_the_result_of_a_single_definition() {
	is!("class pdf{body:text}; render(t):pdf := pdf(t); ((render \"hello\") as pdf).body", "hello");
	is!("class pdf{body:text}; render(t) := pdf(t); ((render \"hello\") as pdf).body", "hello");
}

#[test]
fn phrases_with_different_results_are_overloads() {
	is!(&format!("{PHRASES}((render \"hi\") as docx).body"), "<w>hi");
	is!(&format!("{PHRASES}docx d = render \"hi\"; d.body"), "<w>hi");
	is!(&format!("{PHRASES}((render \"hi\") as pdf).body"), "%PDF hi");
}
