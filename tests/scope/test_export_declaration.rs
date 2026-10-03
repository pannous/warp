use warp::*;

#[test]
fn test_export_without_initializer_is_zero() {
	is!("export int k", 0);
	is!("export k", 0);
}

#[test]
fn test_export_with_type_word_keeps_the_value() {
	is!("export int k=7;k+1", 8);
	is!("export const int k=7", 7);
}

#[test]
fn test_exported_variable_can_be_assigned() {
	is!("export k=7;k=8;k", 8);
}

#[test]
fn test_export_of_a_function_stays_a_function() {
	is!("export square:=it*2;square 3", 6);
}

#[test]
fn test_type_words_come_from_the_analyzer() {
	is!("global text k", 0);
	is!("global bool k", 0);
}
