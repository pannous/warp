// Progressive verification: stated → asserted → tested → proved
use warp::law::{extract_laws, lean, property_test, separate_laws, verify, Assurance, Verdict, PROPERTY_TRIALS};
use warp::type_kinds::Kind;
use warp::wasm_emitter::eval;
use warp::wasp_parser::parse;
use warp::Node;
use crate::is;

const SQUARE: &str = "square(x) := x*x\nlaw square(-x) == square(x)";
const WRONG_DOUBLE: &str = "twice(x) := x+x\nlaw twice(x) == x";

#[test]
fn test_law_stated() {
	let laws = extract_laws(&parse(SQUARE));
	assert_eq!(laws.len(), 1);
	assert_eq!(laws[0].function, "square");
	assert_eq!(laws[0].variables, vec![("x".to_string(), Kind::Int)]);
}

#[test]
fn test_law_does_not_change_program_result() {
	is!(&format!("{SQUARE}\nsquare(3)"), 9);
	is!("add(a,b) := a+b; law add(a,b) == add(b,a); add(3,4)", 7);
}

#[test]
fn test_law_asserted_at_call_sites() {
	let result = eval(&format!("{WRONG_DOUBLE}\ntwice(3)"));
	assert!(matches!(result, Node::Error(_)), "expected law violation, got {result:?}");
	assert!(result.serialize().contains("x=3"), "{}", result.serialize());
}

#[test]
fn test_law_tested_with_generated_inputs() {
	let lawful = separate_laws(parse(SQUARE));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, SQUARE), Verdict::Holds);
	let lawful = separate_laws(parse(WRONG_DOUBLE));
	assert_eq!(
		property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, WRONG_DOUBLE),
		Verdict::Violated("counterexample x=1".into())
	);
}

const HALF: &str = "half(x:float) := x/2\nlaw half(x)*2 == x";

#[test]
fn test_law_float_parameter_kinds() {
	let lawful = separate_laws(parse(HALF));
	assert_eq!(lawful.laws[0].variables, vec![("x".to_string(), Kind::Float)]);
}

#[test]
fn test_law_float_parameters_are_tested() {
	let (code, lawful) = (HALF, separate_laws(parse(HALF)));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, code), Verdict::Holds);
}

#[test]
fn test_law_lean_export() {
	let lawful = separate_laws(parse(SQUARE));
	let source = lean::export(&lawful.functions, &lawful.laws[0], &["grind"]).unwrap();
	assert!(source.contains("def square (x : Int) : Int := (x * x)"), "{source}");
	assert!(source.contains("theorem square_law (x : Int) : (square (-x)) = (square x)"), "{source}");
}

// An unused simp argument is a Lean warning: unfold only the functions the law reaches
#[test]
fn test_law_lean_export_unfolds_only_the_functions_the_law_reaches() {
	let lawful = separate_laws(parse(&format!("{SQUARE}\nadd(a,b) := a+b\nlaw add(a,b) == add(b,a)")));
	let sources: Vec<String> = lawful.laws.iter().map(|law| lean::export(&lawful.functions, law, &["grind"]).unwrap()).collect();
	assert!(sources[0].contains("simp only [square]\n"), "{}", sources[0]);
	assert!(sources[1].contains("simp only [add]\n"), "{}", sources[1]);
	let lawful = separate_laws(parse("double(x) := x+x\nquad(x) := double(double(x))\nlaw quad(x) == 4*x"));
	let source = lean::export(&lawful.functions, &lawful.laws[0], &["grind"]).unwrap();
	assert!(source.contains("simp only [double, quad]\n"), "{source}");
}

#[test]
fn test_law_lean_export_modulo_is_euclidean() {
	let lawful = separate_laws(parse("m(x) := x % 3\nlaw m(x) >= 0"));
	let source = lean::export(&lawful.functions, &lawful.laws[0], &["grind"]).unwrap();
	assert!(source.contains("(Int.emod x (3 : Int))"), "{source}");
	let lawful = separate_laws(parse("t(x) := x rem 3\nlaw t(x) <= 2"));
	let source = lean::export(&lawful.functions, &lawful.laws[0], &["grind"]).unwrap();
	assert!(source.contains("(Int.tmod x (3 : Int))"), "{source}");
}

#[cfg_attr(not(feature = "native"), ignore = "browser: needs the lean prover")]
#[test]
fn test_law_proved_by_lean() {
	crate::requires!(crate::common::LEAN);
	let reports = verify(&format!("{SQUARE}\nadd(a,b) := a+b\nlaw add(a,b) == add(b,a)"));
	assert_eq!(reports.len(), 2);
	for report in reports {
		assert_eq!(report.assurance, Assurance::Proved, "{report}");
	}
}

#[test]
fn test_law_violation_reported_by_verify() {
	let reports = verify(WRONG_DOUBLE);
	assert!(reports[0].failed(), "{}", reports[0]);
}


// Warp Int promotes on overflow: square(3037000500) == 9223372037000250000.
const PROMOTING: &str = "square(x) := x*x\nlaw square(x) >= 0";

#[test]
fn test_law_overflow_promotion_holds_in_property_tests() {
	let lawful = separate_laws(parse(PROMOTING));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, PROMOTING), Verdict::Holds);
}

#[cfg_attr(not(feature = "native"), ignore = "browser: needs the lean prover")]
#[test]
fn test_law_overflow_promotion_proved_by_lean() {
	crate::requires!(crate::common::LEAN);
	let lawful = separate_laws(parse(PROMOTING));
	assert_eq!(lean::prove(&lawful.functions, &lawful.laws[0]), Verdict::Holds);
	let reports = verify(PROMOTING);
	assert_eq!(reports[0].assurance, Assurance::Proved, "{}", reports[0]);
	assert!(!reports[0].failed());
}
