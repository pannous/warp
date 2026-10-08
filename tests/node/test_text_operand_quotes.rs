// card text-operand-quotes: a text or character left of an operator keeps its quotes when serialized, so the code an
// error message shows (`cannot assign text "5"*3`) re-parses to the same tree
use warp::parse;

fn serialized(code: &str) -> String {
	parse(code).serialize().trim().to_string()
}

#[test]
fn a_text_operand_keeps_its_quotes() {
	assert_eq!(serialized("\"ab\"+\"cd\""), "\"ab\"+\"cd\"");
	assert_eq!(serialized("\"5\"*3"), "'5'*3");
	assert_eq!(serialized("\"ab\"==x"), "\"ab\"==x");
	assert_eq!(serialized("\"ab\" as text"), "\"ab\" as text");
	assert_eq!(serialized("'a'..'z'"), "'a'..'z'");
}
