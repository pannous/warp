// cards typed-holes and code-hole-types (matching roadmap step 2): a hole of a code pattern with a condition, `$r: int`,
// `$n: > 0`, `$s: symbol`, `$b: block`, binds the code there only when it meets the condition; `$k: int + $x` is
// `($k: int) + $x`, though `:` binds looser than `+`
use crate::is;

const SUMMAND: &str = "summand(n) := match n { data($k: int + $x) => x; data($x + $k: int) => x; _ => \"none\" }\n";
const KINDS: &str =
	"kind(n) := match n { data(f($n: > 0)) => \"positive\"; data(f($s: symbol)) => s; data(f($b: block)) => \"block\"; _ => \"other\" }\n";

#[test]
fn a_typed_hole_matches_only_its_type() {
	is!(&format!("{SUMMAND}summand(data(3 + y))"), "y");
	is!(&format!("{SUMMAND}summand(data(y + 3))"), "y");
	is!(&format!("{SUMMAND}summand(data(y + z))"), "none");
	is!("match data(2 * 3 + 4) { data($a: int * $b: int + $c) => a * b; _ => 0 }", 6);
}

#[test]
fn a_hole_meets_its_condition() {
	is!(&format!("{KINDS}kind(data(f(5)))"), "positive");
	is!(&format!("{KINDS}kind(data(f(y)))"), "y");
	is!(&format!("{KINDS}kind(data(f({{y}})))"), "block");
	is!(&format!("{KINDS}kind(data(f(-5)))"), "other");
	is!("match data([1, 2, 3]) { data([$h: int, $t...]) => h; _ => 0 }", 1);
}
