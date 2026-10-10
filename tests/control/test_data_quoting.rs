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

// card data-list: quoted data in a list reads as written, as the list held in a variable does
#[test] fn data_in_a_list_reads_as_written() {
	is!("string [data a or b, 3]", "[a or b 3]");
	is!("xs = [data a or b, 3]; string xs", "[a or b 3]");
}

#[test] // a variable named data is read: no prefix quotes what follows it (card counting-sort)
fn a_variable_named_data_is_no_prefix() {
	is!("data=[4 2 2]; data where it == 2", warp::ints(vec![2, 2]));
	is!("data=[4 2 2]; n=0; for x in data { n += x }; n", 8);
	is!("data = [4 2 2 8 3 3 1]\nresult = []\nfor v in 1 to 8 {\n\tmatches = [x for x in data where x == v]\n\tc = #matches\n\tj = 0\n\twhile j < c {\n\t\tresult += [v]\n\t\tj += 1\n\t}\n}\nresult", warp::ints(vec![1, 2, 2, 3, 3, 4, 8]));
}
