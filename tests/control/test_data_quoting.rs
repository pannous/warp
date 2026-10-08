// card data-quoting: `data` takes the whole expression after it, as text it reads as written
use crate::is;

#[test]
fn data_takes_the_whole_expression_after_it() {
	is!("x = data a and b; string x", "a and b");
	is!("string(data a and b)", "a and b");
}

#[test]
fn parentheses_around_the_data_only_group_it() {
	is!("string(data (a and b))", "a and b");
	is!("x = data (a and b); string(x)", "a and b");
}

#[test]
fn data_of_a_tag_reads_as_written() {
	is!("string(data point{x:1})", "point{x:1}");
}

// card data-tag: a tag held in a variable reads as written too, a map entry keeps its braces
#[test] fn a_tag_in_a_variable_reads_as_written() {
	is!("p = data point{x:1}; string p", "point{x:1}");
	is!("m = {a: data point{x:1}}; string m", "{a:point{x:1}}");
	is!("m = {a:{b:1}}; string m", "{a:{b:1}}");
	is!("e = data a:1; string e", "{a:1}");
}
