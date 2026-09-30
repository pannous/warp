use warp::wasm_emitter::eval;
use warp::*;

#[test]
fn test_angle_and_of_forms_are_the_same_type() {
	for declaration in ["x:list<int>=[1 2]", "x:list of int=[1 2]"] {
		is!(&format!("{declaration}; type(x)"), "list of int");
		is!(&format!("{declaration}; x size"), 2);
	}
}

#[test]
fn test_nested_type_applications() {
	is!("x:list<list<int>>=[[1], [2]]; type(x)", "list of list of int");
	is!("x:list of list of int=[[1], [2]]; type(x)", "list of list of int");
}

#[test]
fn test_type_of_a_literal_list_stays() {
	is!("type([1 2 3])", "list of int");
}

#[test]
fn test_angle_brackets_after_a_name_are_not_a_variable_lookup() {
	let result = format!("{:?}", eval("x=[1 2 3]; x<int>"));
	assert!(!result.contains("undefined variable: int"), "{result}");
}

#[test]
fn test_comparisons_and_shifts_are_not_type_applications() {
	is!("a=1;b=2;a < b", true);
	is!("a=1;b=2;a > b", false);
	is!("a=3;b=2;a >= b", true);
	is!("1 << 3", 8);
	is!("1<<3", 8);
}
