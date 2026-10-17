// card code-hole-arm-state: quoted code of several shapes, a call `data (g(y, z))` beside `data (n * 1)`, passes a
// Node to the matching function (a call before gave it a list parameter, and `data($a * 1)` stopped matching)
use crate::is;

const SIMPLIFY: &str =
	"simplify(code) := match code {\n\tdata($a * 1) => a\n\tdata($f($x, $x)) => data twice\n\t_ => code\n}\n";

#[test]
fn a_key_pattern_matches_after_a_call_was_passed() {
	is!(&format!("{SIMPLIFY}simplify(data (g(y, z))); simplify(data (n * 1)) == data n"), true);
	is!(&format!("{SIMPLIFY}simplify(data (g(y, y))) == data twice"), true);
}

#[test]
fn every_arm_in_one_list() {
	is!(
		&format!("{SIMPLIFY}[simplify(data (n * 1)), simplify(data (g(y, y))), simplify(data (g(y, z)))] == [data n, data twice, data (g(y, z))]"),
		true
	);
}
