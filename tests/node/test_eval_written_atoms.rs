// card eval-code: code eval parses at run time may hold atoms no program value is (π, 2√2, nan); they reach the
// block as written and mean there what they mean in the program
use crate::is;

#[test]
fn eval_of_run_time_code_keeps_its_exact_atoms() {
	is!("f(c: text) := eval(c); 3.14 < f(\"π\") < 3.15", true);
	is!("f(c: text) := eval(c); 6.28 < f(\"2*π\") < 6.29", true);
	is!("f(c: text) := eval(c); f(\"√2 * √2\")", 2);
}
