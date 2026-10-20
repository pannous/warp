//! An enum declared in a used file reaches the files using it (card rust-lane): a file's own `use "x.warp"` is found
//! beside that file, and its cases are the cases there and in the program
use crate::is;
use warp::symbol;

const MIDDLE: &str = "use \"tests/modules/used_enum/middle.warp\"";

#[test]
fn an_enum_of_a_file_a_used_file_uses() {
	is!(&format!("{MIDDLE}; is_function(Meaning.Function)"), true);
	is!(&format!("{MIDDLE}; is_function(Meaning.Variable)"), false);
	is!(&format!("{MIDDLE}; variable_meaning"), symbol("Variable"));
}
