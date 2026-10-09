// ++, --, += and = change the whole place they follow: an element `xs#1++`, a field of an element `bags#1.n++`, a nested
// field `p.b.n += 2` (card increment-element: `xs#1++` was an error, `bags#1.n++` parsed as `bags#1.(n++)`, a no-op)
use crate::is;
use warp::parse;

#[test]
fn an_element_increments() {
	is!("xs = [1 2]; xs#1++; xs", parse("[2 2]"));
	is!("xs = [1 2]; xs#2--; xs", parse("[1 1]"));
	is!("xs = [1 2]; i = 2; xs#i++; xs", parse("[1 3]"));
	is!("xs = [[1 2]]; xs#1#2++; xs", parse("[[1 3]]"));
}

#[test]
fn a_field_of_an_element_changes() {
	is!("bags = [{n: 1}]; bags#1.n = 5; bags#1.n", 5);
	is!("bags = [{n: 1}]; bags#1.n++; bags#1.n", 2);
	is!("bags = [{n: 1}]; bags#1.n += 4; bags#1.n", 5);
	is!("class bag{n: int}; bags = [bag(1)]; bags#1.n++; bags#1.n", 2);
}

#[test]
fn a_nested_field_changes() {
	is!("p = {b: {n: 1}}; p.b.n++; p.b.n", 2);
	is!("p = {b: {n: 1}}; p.b.n += 2; p.b.n", 3);
	is!("b = {n: 1}; b.n++; b.n", 2);
}

#[test]
fn an_element_of_a_field_and_a_nested_element_change() {
	is!("p = {items: [{n: 1}]}; p.items#1.n = 3; p.items#1.n", 3);
	is!("p = {items: [{n: 1}]}; p.items#1.n++; p.items#1.n", 2);
	is!("p = {items: [1 2]}; p.items#1 += 3; p.items", parse("[4 2]"));
	is!("xs = [[1 2]]; xs#1#2 += 5; xs", parse("[[1 7]]"));
	is!("b = {n: 1}; b.n *= 3; b.n", 3);
}
