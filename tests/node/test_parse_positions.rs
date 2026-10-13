//! Card parse-word, its last open part: `parse(t)` of a text known only at run time keeps where each part was written
//! and the comment before it, as `parse("…")` of a constant text and `data (…)` do (notes/parse_word.md)
use crate::is;

const RUNTIME_TEXT: &str = "use comments\nt = \"x = 1\\n// note\\n  n * \" + \"2\"\nr = parse(t)\n";

#[test]
fn code_parsed_at_run_time_knows_its_line_column_and_comment() {
	is!(&format!("{RUNTIME_TEXT}r#2.line"), 3);
	is!(&format!("{RUNTIME_TEXT}r#2.column"), 3);
	is!(&format!("{RUNTIME_TEXT}r#2.comment"), "note");
	is!(&format!("{RUNTIME_TEXT}r#2 == data (n * 2)"), true);
}
