// Generic constraints: a parameter `xs: Comparable list` takes a list whose elements conform to the trait, checked
// where the function is called (wiki/trait.md)
use crate::common::fails_with;
use warp::is;

const AGES: &str = "class dot{x:int}; class age{n:int}; compare(a:age, b:age) := a.n - b.n; smallest(xs: Comparable list) := (sort xs)#1; ";
const SHAPES: &str = "trait shape{area}; class sq{s:int}; class dot{x:int}; area(q:sq) := q.s*q.s; total(xs: shape list) := { t=0; for x in xs { t += area(x) }; t }; ";

#[test]
fn a_constrained_list_parameter_takes_conforming_elements() {
	is!(&format!("{AGES}smallest([3 1 2])"), 1);
	is!(&format!("{AGES}smallest([age(3) age(1)]).n"), 1);
	is!(&format!("{SHAPES}total([sq(2) sq(3)])"), 13);
	is!("f(xs: int list) := xs#2; f([3 4])", 4);
}

#[test]
fn elements_that_do_not_conform_are_a_compile_error() {
	fails_with(&format!("{AGES}smallest([dot(3) dot(1)])"), "dot is not Comparable");
	fails_with(&format!("{AGES}ds=[dot(3) dot(1)]; smallest(ds)"), "compare(a:dot, b:dot)");
	fails_with(&format!("{SHAPES}total([dot(2)])"), "dot is not shape");
	fails_with("f(xs: flaot list) := 1; f([1])", "unknown type list of flaot");
}

#[test]
fn a_list_annotation_reads_both_ways() {
	is!("f(xs: list of int, y) := xs#2 + y; f([3 4], 1)", 5);
	is!("f(xs: int list, y) := xs#2 + y; f([3 4], 1)", 5);
	is!(&format!("{AGES}least(xs: list of Comparable) := (sort xs)#1; least([age(3) age(1)]).n"), 1);
	fails_with(&format!("{AGES}least(xs: list of Comparable) := (sort xs)#1; least([dot(3) dot(1)])"), "dot is not Comparable");
}
