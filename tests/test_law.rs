// Progressive verification: stated → asserted → tested → proved
use warp::law::{extract_laws, lean, property_test, separate_laws, verify, Assurance, Verdict, PROPERTY_TRIALS};
use warp::type_kinds::Kind;
use warp::wasm_emitter::eval;
use warp::wasp_parser::parse;
use warp::{is, Node};

const SQUARE: &str = "square(x) := x*x\nlaw square(-x) == square(x)";
const WRONG_DOUBLE: &str = "double(x) := x+x\nlaw double(x) == x";

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
	let result = eval(&format!("{WRONG_DOUBLE}\ndouble(3)"));
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
#[ignore = "next"] // law found it: `:=` functions compile x:float params as Int, half(1.0) == 0
fn test_law_float_parameters_are_tested() {
	let (code, lawful) = (HALF, separate_laws(parse(HALF)));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, code), Verdict::Holds);
}

#[test]
fn test_law_lean_export() {
	let lawful = separate_laws(parse(SQUARE));
	let source = lean::export(&lawful.functions, &lawful.laws[0], &["grind"]).unwrap();
	assert!(source.contains("def square (x : BitVec 64) : BitVec 64 := (x * x)"), "{source}");
	assert!(source.contains("theorem square_law (x : BitVec 64) : (square (-x)) = (square x)"), "{source}");
}

#[test]
fn test_law_proved_by_lean() {
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


// Warp Int wraps: square(3037000500) == -9223372036709301616, so x*x >= 0 is false
const OVERFLOWING: &str = "square(x) := x*x\nlaw square(x) >= 0";

#[test]
fn test_law_overflow_found_by_property_tests() {
	let lawful = separate_laws(parse(OVERFLOWING));
	assert_eq!(
		property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, OVERFLOWING),
		Verdict::Violated("counterexample x=3037000500".into())
	);
}

#[test]
fn test_law_overflow_found_by_lean() {
	let lawful = separate_laws(parse(OVERFLOWING));
	let verdict = lean::prove(&lawful.functions, &lawful.laws[0]);
	assert!(matches!(&verdict, Verdict::Violated(why) if why.contains("lean counterexample x=")), "{verdict:?}");
	assert!(verify(OVERFLOWING)[0].failed());
}
