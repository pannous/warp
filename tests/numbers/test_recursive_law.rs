// card recursive-laws (notes/bend.md): a law about a recursive function is proved by Lean through functional induction
// (`fun_induction`, the recursive call is the induction hypothesis); the function is exported with the termination
// measure "the sum of its parameters' sizes". `law a => b` holds where a does not; an instance recursing too deep to
// run is no counterexample
use warp::law::{lean, property_test, separate_laws, verify, Assurance, Verdict, PROPERTY_TRIALS};
use warp::warp_parser::parse;

const SUM_TO: &str = "sum_to(n) := if n <= 0 then 0 else n + sum_to(n - 1)";

fn law_of(function: &str, law: &str) -> String {
	format!("{function}\nlaw {law}")
}

#[test]
fn a_recursive_function_is_exported_with_its_termination_measure() {
	let code = law_of(SUM_TO, "sum_to(n) >= 0");
	let lawful = separate_laws(parse(&code));
	let source = lean::export(&lawful.functions, &lawful.laws[0], lean::TACTICS).unwrap();
	assert!(source.contains("termination_by n.toNat\ndecreasing_by all_goals omega"), "{source}");
	assert!(source.contains("fun_induction sum_to n"), "{source}");
}

#[test]
fn an_instance_too_deep_to_run_is_no_counterexample() {
	let code = law_of(SUM_TO, "sum_to(n) >= 0");
	let lawful = separate_laws(parse(&code));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, &code), Verdict::Holds);
}

#[test]
fn a_precondition_skips_the_instances_it_excludes() {
	let code = law_of(SUM_TO, "n >= 0 => sum_to(n) * 2 == n * (n + 1)");
	let lawful = separate_laws(parse(&code));
	assert_eq!(property_test(&lawful, &lawful.laws[0], PROPERTY_TRIALS, &code), Verdict::Holds);
}

#[test]
fn a_false_recursive_law_has_its_counterexample() {
	let reports = verify(&law_of(SUM_TO, "sum_to(n) == n"));
	assert!(reports[0].failed(), "{}", reports[0]);
	assert!(reports[0].to_string().contains("counterexample n=-1"), "{}", reports[0]);
}

#[cfg_attr(not(feature = "native"), ignore = "browser: needs the lean prover")]
#[test]
fn recursive_laws_are_proved_by_induction() {
	crate::requires!(crate::common::LEAN);
	for (function, law) in [
		(SUM_TO, "sum_to(n) >= 0"),
		(SUM_TO, "n >= 0 => sum_to(n) * 2 == n * (n + 1)"),
		("power(b, e) := if e <= 0 then 1 else b * power(b, e - 1)", "e >= 0 => power(b, e + 1) == b * power(b, e)"),
	] {
		let reports = verify(&law_of(function, law));
		assert_eq!(reports[0].assurance, Assurance::Proved, "{}", reports[0]);
	}
}

// a law proved before is a lemma of the laws after it, as Bend proves with helper laws: `fact(n) > 0` needs
// x*y > 0 from x, y > 0, which Lean's automation does not find in the induction alone
#[cfg_attr(not(feature = "native"), ignore = "browser: needs the lean prover")]
#[test]
fn a_proved_law_is_a_lemma_of_the_next() {
	crate::requires!(crate::common::LEAN);
	let fact = "fact(n) := if n <= 0 then 1 else n * fact(n - 1)";
	assert_eq!(verify(&law_of(fact, "fact(n) > 0"))[0].assurance, Assurance::Tested);
	let reports = verify(&format!("{fact}\nlaw x > 0 and y > 0 => x * y > 0\nlaw fact(n) > 0"));
	assert_eq!(reports[0].assurance, Assurance::Proved, "{}", reports[0]);
	assert_eq!(reports[1].assurance, Assurance::Proved, "{}", reports[1]);
}
