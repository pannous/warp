//! Tree-patterns step 7: code patterns match code parsed at run time, and quoted code of several statements stays as
//! written (notes/matching.md, notes/parse_word.md)
use crate::is;

const SIMPLIFY: &str = "simplify(code) := match code {\n\tdata($a + 0) => a\n\tdata($a * 1) => a\n\t_ => code\n}\n";

#[test]
fn code_pattern_matches_code_parsed_at_run_time() {
	is!(&format!("{SIMPLIFY}t = \"n * \" + \"1\"\nsimplify(parse(t)) == data n"), true);
	is!(&format!("{SIMPLIFY}simplify(parse(\"y + 0\")) == data y"), true);
}

#[test]
fn quoted_statements_bind_no_blocks() {
	is!("x = data (j: 1\nk: n * 1)\n(x#2).name", "k");
	is!("x = parse(\"j: 1\\nk: n * 1\")\n(x#2).name", "k");
	is!("x = data (j: 1\nk: n * 1)\nx#2 == parse(\"k: n * 1\")", true);
}
