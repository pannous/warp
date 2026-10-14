//! Card data-quote-lowered: quoted code stays as written, the same Node the run-time `parse(t)` gives; no lowering pass
//! rewrites inside `data (…)` (a member access stayed `#`, a literal range became its list)

use crate::is;

/// `data (code) == parse(t)` where t holds the code only at run time
fn quote_equals_runtime_parse(code: &str) -> String {
	format!("q = data ({code})\nt = \"{code}\" + \"\"\nq == parse(t)")
}

#[test]
fn quoted_code_is_the_code_parse_gives() {
	is!("q = data (a.b); q.op", ".");
	is!("q = data (1..3); q.op", "..");
	for code in ["a.b", "a.warp", "1..3", "x += 1", "f(x) := x * 2", "a > 0 ? b : c", "5 km"] {
		is!(&quote_equals_runtime_parse(code), true);
	}
}
