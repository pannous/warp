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

/// Card note-dispatch-element-wise: Note (use sound) defines plus and minus, so `a + b` may dispatch to them; the
/// element-wise operators still pair items, and transpose each note of a list
#[test]
fn element_wise_operators_pair_items_beside_a_class_defining_them() {
	let notes = "use sound\nn = note(C4)\n";
	is!(&format!("{notes}added(p) := p .+ [1, 2]; added([3, 4])"), parse("[4 6]"));
	is!(&format!("{notes}taken(p) := p .- [1, 2]; taken([3, 4])"), parse("[2 2]"));
	is!(&format!("{notes}c() := [1, 2]; taken(p) := p .- c(); taken([3, 4])"), parse("[2 2]"));
	is!(&format!("{notes}0.5 .+ [1, 2]"), parse("[1.5 2.5]"));
	is!(&format!("{notes}up(ns) := ns .+ 2; up([n, note(E4)]).map(x => x.key)"), parse("[62 66]"));
	// melody_samples passes lists to closures of two parameters, which the analyzer reads as passed to every such closure
	let song = "use sound\nsong = melody_samples([C4], 0.25)\n";
	is!(&format!("{song}c() := [1, 2]; taken(p) := p .- c(); 1"), 1);
}

/// The prelude's element_wise_any, which `a .op b` calls where a side may be a list: each item of the list side, two
/// lists paired, two numbers plainly
#[test]
fn element_wise_any_pairs_lists_and_broadcasts_numbers() {
	is!("use prelude\nelement_wise_any(2, [1, 2], (a, b) => a * b)", parse("[2 4]"));
	is!("use prelude\nelement_wise_any([1, 2], [3, 4], (a, b) => a + b)", parse("[4 6]"));
	is!("use prelude\nelement_wise_any([1, 2], 10, (a, b) => a - b)", parse("[-9 -8]"));
	is!("use prelude\nelement_wise_any(3, 4, (a, b) => a * b)", 12);
}

/// A painted function's 3-vector of 0…1 is its pixel's color on the CPU as on the GPU: pixel_color packs it as
/// 0xAARRGGBB, each pixel_channel clamped and rounded; a number stays itself
#[test]
fn a_color_vector_packs_into_a_pixel() {
	is!("use prelude\npixel_color([1, 0, 0])", 0xFFFF0000_i64);
	is!("use prelude\npixel_color(7)", 7);
	is!("use prelude\n[pixel_channel(0.5), pixel_channel(2), pixel_channel(-1)]", parse("[128 255 0]"));
}
