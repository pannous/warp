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

#[test]
fn an_element_wise_operator_of_a_call_decides_at_run_time() {
	is!("xs = [1.5, 2.25]; xs .- floor(xs)", parse("[0.5 0.25]"));
	is!("dotted(a, b) := dot(a, b); dotted([1, 2], [3, 4])", 11);
}

#[test]
fn norm_is_the_euclidean_length_of_a_list() {
	is!("norm([3, 4])", 5);
	is!("v = [3, -4]; norm v", 5);
	is!("magnitude([6, 8])", 10);
	is!("norm(-3)", 3);
}

#[test]
fn libm_words_of_one_number_map_over_a_list() {
	is!("sin([0, 0])", parse("[0 0]"));
	is!("xs = [0, 0]; exp(xs)", parse("[1.0 1.0]"));
	is!("sqrt([4, 9])", parse("[2 3]"));
	is!("use math; cosine([0, 0])", parse("[1.0 1.0]"));
}

#[test]
fn shader_math_words_take_numbers_and_vectors() {
	is!("use math; lerp([0, 0], [2, 4], 0.5)", parse("[1 2]"));
	is!("use math; mix(0, 10, 0.25)", 2.5);
	is!("use math; fract(2.25)", 0.25);
	is!("use math; fract([1.5, 2.25])", parse("[0.5 0.25]"));
	is!("use math; step(0.5, 0.7)", 1);
	is!("use math; smoothstep(0, 1, 0.5)", 0.5);
	is!("use math; distance([0, 0], [3, 4])", 5);
	is!("use math; normalize([3, 4])", parse("[0.6 0.8]"));
	is!("use math; cross([1, 0, 0], [0, 1, 0])", parse("[0 0 1]"));
	is!("use math; reflect([1, -1], [0, 1])", parse("[1 1]"));
}
