//! Card parse-type (user: `parse("1+1").type` said key): a node written with an operator is an operation, a key
//! written with `:` stays a key; expression is another word for operation. Decision body-code: parsed code has the type
//! code, and tests as its form (operation, key) too
use crate::is;

#[test]
fn an_operator_node_is_an_operation() {
	is!("parse(\"1+1\").type", "code");
	is!("type(parse(\"a < b\"))", "code");
	is!("x = data (1+1); type(x)", "operation");
	is!("x = parse(\"a<b\"); x.type", "code");
	is!("parse(\"1+1\").op", "+");
}

#[test]
fn a_pair_stays_a_key() {
	is!("parse(\"a:1\").type", "code");
	is!("parse(\"a=1\").type", "code");
	is!("type(parse(\"a:1\"))", "code");
}

#[test]
fn an_operation_is_a_key_and_an_expression() {
	is!("parse(\"1+1\") is operation", true);
	is!("parse(\"1+1\") is expression", true);
	is!("parse(\"1+1\") is key", true);
	is!("parse(\"a:1\") is operation", false);
}
