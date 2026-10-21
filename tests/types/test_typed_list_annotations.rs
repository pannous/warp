//! card typed-list-annotations: list types inside annotations and casts: `list of Name` of an alias, a plural result
//! of a function with typed parameters, a list cast `xs as texts` converting each item
use crate::is;

#[test]
fn an_alias_inside_a_list_type() {
	is!("type Name = text\nf(xs: list of Name) := #xs\nf([\"a\", \"b\"])", 2);
	is!("type Name = text\nclass Names { all: list of Name }\nNames{all: [\"x\"]}.all#1", "x");
}

#[test]
fn a_plural_result_of_typed_parameters() {
	is!("h(xs: ints): ints := xs\nh([1, 2])#2", 2);
	is!("h(xs: list): ints := xs\nh([1, 2])#1", 1);
	is!("h(x: int): ints := [x, x]\n#h(3)", 2);
	is!("k(xs: list): texts := xs\nk([\"a\"])#1", "a");
}

#[test]
fn a_list_cast_converts_each_item() {
	is!("ys = [\"a\"] as texts\nys#1", "a");
	is!("ys = [1, 2] as texts\nys#2", "2");
	is!("ys = [\"1\", \"2\"] as ints\nys#1 + ys#2", 3);
	is!("ys = [1, 2] as [text]\nys#1", "1");
}
