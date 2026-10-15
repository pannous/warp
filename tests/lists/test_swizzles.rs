//! card compile-shaders step 4, decision S1 (notes/compile_shaders.md): `.x .y .z .w` and swizzles `.xy .zyx .xyy` read
//! the components of a list of 2 to 4 numbers, known when compiled or only at run time; a map's entry of that name, a
//! square-bracket map's too, keeps winning
use crate::common::fails_with;
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_letter_is_a_component_of_a_list_of_numbers() {
	is!("p = [1,2,3]; p.x", 1);
	is!("p = [1,2,3]; p.y + p.z", 5);
	is!("v = [1,2,3,4]; v.w", 4);
	is!("p = [1,2,3]; q = p.x; q + 1", 2);
	is!("f(p) := p.y; f([4,5])", 5);
}

#[test]
fn letters_pick_a_list_of_components() {
	is!("[1,2,3].zy", parse("[3 2]"));
	is!("p = [1,2,3]; q = p.xy; q", parse("[1 2]"));
	is!("v = [1,2,3,4]; v.wzyx", parse("[4 3 2 1]"));
	is!("p = [1,2,3]; sum(p.xyz)", 6);
	is!("p = [1,2]; p.yx .* 2", parse("[4 2]"));
	is!("f(p) := p.zyx; f([1,2,3])", parse("[3 2 1]"));
	is!("f(p) := p.xyy; f([1.5,2])", parse("[1.5 2 2]"));
}

#[test]
fn a_map_entry_of_the_name_wins() {
	is!("p = {x:7}; p.x", 7);
	is!("p = [x:1, y:2]; p.x", 1);
	is!("p = [x:1, xy:[7,8]]; q = p.xy; q", parse("[7 8]"));
	is!("p = [x:\"a\", xy:\"hi\"]; q = p.xy; q", "hi");
	is!("f(p) := p.xy; f([x:1, xy:\"hi\"])", "hi");
	is!("f(p) := p.y; f([x:1, y:2])", 2);
	is!("class Point{x:int y:int}; p = Point{x:1 y:2}; p.x + p.y", 3);
}

#[test]
fn a_component_beyond_the_vector_is_no_field() {
	fails_with("p = [1,2,3]; p.w", "no field w");
	fails_with("f(p) := p.z; f([1,2])", "no field z");
	fails_with("f(p) := p.x; f([1,2,3,4,5])", "no field x");
	fails_with("f(p) := p.x; f([yes,no])", "no field x");
	fails_with("p = [x:1, y:2]; p.yx", "no field yx");
}
