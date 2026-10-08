// A library word passed to map or filter is the function it names, like an operator word (`xs.map(sqrt)`);
// a list a function updates is a list whether or not the function is called
use crate::is;

#[test]
fn a_library_word_is_a_function_value_of_map_and_filter() {
	is!("[\"hi\", \"yo\"].map(upper)#2", "YO");
	is!("map([\"hi\"], upper)#1", "HI");
	is!("count(['1', 'b', '2'].filter(is_digit))", 2);
	is!("upper(x) := x + \"!\"; [\"hi\"].map(upper)#1", "hi!");
}

#[cfg(feature = "native")]
#[test]
fn print_in_a_block_prints_the_one_expression() {
	assert!(crate::common::printed("for n in [\"hi\"] { print upper n }").starts_with("HI\n"));
}

#[test]
fn an_uncalled_function_updating_its_list_parameter_compiles() {
	is!("f(xs) := { xs.add(420); xs }; 5", 5);
	is!("f(xs) := xs + [1]; 5", 5);
	is!("f(xs) := { xs.add(420); xs }; count(f([1]))", 2);
}

#[test]
fn a_function_updating_a_main_list_asks_for_global() {
	is!("names = [\"a\"]; grow() := { global names; names.add(420) }; grow(); count(names)", 2);
	crate::common::fails_with("names = [\"a\"]; grow() := { names.add(420) }; 1", "declare it `global names`");
}
