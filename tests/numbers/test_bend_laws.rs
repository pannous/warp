//! Bend-style named quantifiers and the project-level LAWS.warp verification gate (card bend-style).
#![cfg(feature = "native")]

use warp::law::{extract_laws, Assurance};
use warp::type_kinds::Kind;
use warp::warp_parser::parse;

const QUANTIFIED_LAW: &str = "square(x) := x*x\nlaw square_symmetry:\n  for x: int\n  square(-x) == square(x)";

#[test]
fn a_named_for_quantifier_is_the_existing_law_form() {
	let laws = extract_laws(&parse(QUANTIFIED_LAW));
	let canonical = extract_laws(&parse("square(x) := x*x\nlaw square(-x) == square(x)"));
	assert_eq!(laws.len(), 1);
	assert_eq!(laws[0].variables, vec![("x".to_string(), Kind::Int)]);
	assert_eq!(laws[0].statement, canonical[0].statement);
	assert_eq!(laws[0].to_string(), format!("law square_symmetry: {}", canonical[0].statement.serialize().trim()));
}

fn verify(arguments: &[&str]) -> std::process::Output {
	crate::common::warp_command().args(arguments).output().expect("warp verify runs")
}

#[test]
fn the_project_laws_file_is_a_green_verification_gate() {
	let run = verify(&["verify", "LAWS.warp"]);
	let output = String::from_utf8_lossy(&run.stdout);
	assert!(run.status.success(), "{output} {}", String::from_utf8_lossy(&run.stderr));
	assert!(output.contains("Proved  law square_symmetry"), "{output}");
	assert!(output.contains("ALL PROOFS CHECK"), "{output}");
}

#[test]
fn a_violated_law_makes_the_gate_loud_and_red() {
	let run = verify(&["verify", "twice(x) := x+x\nlaw double_is_identity:\n  for x: int\n  twice(x) == x"]);
	let output = String::from_utf8_lossy(&run.stdout);
	assert!(!run.status.success(), "{output}");
	assert!(output.contains("FAILED") && output.contains("SOME PROOFS FAIL"), "{output}");
}

#[test]
fn quantified_laws_still_reach_the_lean_prover() {
	crate::requires!(crate::common::LEAN);
	let report = &warp::law::verify(QUANTIFIED_LAW)[0];
	assert_eq!(report.assurance, Assurance::Proved, "{report}");
}
