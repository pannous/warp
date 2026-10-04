// An error points at a source position even where a lowering pass rebuilt the node: the position of its first part
use crate::common::fails_with;

#[test]
fn a_rebuilt_node_reports_the_position_of_its_parts() {
	fails_with("class dot{x:int}\nsmallest(xs: Comparable list) := (sort xs)#1\nsmallest([dot(3) dot(1)])", "at 3:");
	fails_with("x = 1\ny = x + \"a\" * [1]", "at 2:");
}

#[test]
fn an_error_found_while_emitting_names_its_statement() {
	fails_with("s=0\nfor x in [\"a\" \"b\"] {\n  s+=x\n}\ns", "at 3:3");
}
