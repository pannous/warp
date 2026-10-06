// A prefix or suffix operator's missing operand is ø in the tree; the text shows the operator as written (`#x`, not
// `ø#x`), and an ø written as a value stays (`x = ø`, `a: ø`, `ø == x`)
use warp::parse;

fn serialized(code: &str) -> String {
	parse(code).serialize().trim().to_string()
}

#[test]
fn a_missing_operand_is_not_written() {
	assert_eq!(serialized("#x"), "#x");
	assert_eq!(serialized("-x"), "-x");
	assert_eq!(serialized("not x"), "not x");
	assert_eq!(serialized("√x"), "√x");
	assert_eq!(serialized("x++"), "x++");
	assert_eq!(serialized("y = (#name as string)"), "y=(#name as string)");
}

#[test]
fn a_written_empty_value_stays() {
	assert_eq!(serialized("x = ø"), "x=ø");
	assert_eq!(serialized("[1 ø 2]"), "[1 ø 2]");
	assert_eq!(serialized("x == ø"), "x==ø");
}

#[test]
fn a_prefix_form_reads_back_as_the_same_tree() {
	for code in ["#x", "-x", "not x", "x++", "1 - -x"] {
		assert_eq!(parse(&parse(code).serialize()), parse(code), "{code}");
	}
}

#[test]
fn a_condition_keyword_is_written_without_its_empty_left() {
	assert_eq!(serialized("if x then 1 else 2"), "if x then 1 else 2");
	assert_eq!(serialized("while(i<9){i++}"), "while (i<9) do {i++}");
	for code in ["if x then 1 else 2", "while(i<9){i++}", "if x {1}"] {
		assert_eq!(parse(&parse(code).serialize()), parse(code), "{code}");
	}
}
