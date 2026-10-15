//! Card compile-shaders: shader math in warp is vector math on lists of numbers with the element-wise operators (D3,
//! decision S2: the same meaning on the CPU and the GPU). A number on either side broadcasts, also a variable or a
//! parameter holding one, and two lists pair by index, wherever the compiler cannot tell which side is the list
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_number_on_the_left_broadcasts() {
	is!("2 .* [1, 2]", parse("[2 4]"));
	is!("xs = [1, 2]; 2 .- xs", parse("[1 0]"));
	is!("xs = [1, 2]; 1 ./ xs", parse("[1 0.5]"));
}

#[test]
fn a_number_variable_broadcasts_on_either_side() {
	is!("k = 3; xs = [1, 2]; k .* xs", parse("[3 6]"));
	is!("k = 3; xs = [1, 2]; xs .* k", parse("[3 6]"));
	is!("k = 3; k .* 2", 6);
}

#[test]
fn a_shader_function_mixes_numbers_and_vectors() {
	is!("hue(h) := 0.5 .+ 0.5 .* [h, h]; hue(1)", parse("[1 1]"));
	is!("spread = 2; turned(h) := spread .* (h .+ [0, 1]); turned(1)", parse("[2 4]"));
	is!("summed(a, b) := a .+ b; summed([1, 2], [3, 4])", parse("[4 6]"));
}
