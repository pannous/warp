use crate::common::fails_with;

#[test]
fn test_undefined_variable_use_is_an_error_value() {
	fails_with("undefinedname+1", "undefined variable: undefinedname");
}

#[test]
fn test_undefined_variable_increment_is_an_error_value() {
	fails_with("i++", "undefined variable: i");
}

#[test]
fn test_undefined_variable_compound_assignment_is_an_error_value() {
	fails_with("y+=1", "undefined variable: y");
}

#[test]
#[ignore = "card gap-element-array"]
fn an_element_array_phrase_declares_the_list() {
	crate::is!("x is a 100 element array; x.length", 100);
}
