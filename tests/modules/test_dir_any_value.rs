// `dir` of any value answers (card error-undefined-dir): a value it knows nothing about lists no names, a map its keys,
// instead of "undefined: dir"
use crate::is;

#[test]
fn dir_of_a_plain_value_lists_nothing() {
	is!("names = dir pi; #names", 0);
	is!("#dir(pi)", 0);
	is!("x = 3.14; names = dir x; #names", 0);
	is!("#dir(\"hi\")", 0);
	is!("#dir([1 2])", 0);
	is!("class P{a:int}; x = 3; #dir(x)", 0);
}

#[test]
fn dir_of_a_map_lists_its_keys() {
	is!("(dir {a:1 b:2})#2", "b");
	is!("class P{a:int}; m = {b:1}; (dir m)#1", "b");
}
