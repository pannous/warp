//! A bare name defined nowhere as a match arm binds the subject when its body uses it: `name => "hi " + name` catches
//! the rest (card match-binding-arm); a name its body doesn't use stays a symbol case (`red: "fire"`), a variable compares
use crate::is;

#[test]
fn bare_name_arm_binds_the_subject() {
	is!("match \"bob\" { \"alice\" => \"hey\"; name => \"hi \" + name }", "hi bob");
	is!("match 7 { 1 => 0; n => n * 2 }", 14);
}

#[test]
fn bare_name_unused_by_its_body_is_a_symbol_case() {
	is!("color = red; switch color { green: \"woods\"; red: \"fire\" }", "fire");
}

#[test]
fn variable_arm_compares() {
	is!("limit = 3; match 3 { limit => \"at limit\"; _ => \"other\" }", "at limit");
	is!("limit = 3; match 4 { limit => \"at limit\"; _ => \"other\" }", "other");
}
