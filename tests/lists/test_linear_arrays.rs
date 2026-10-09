// `linear xs = int[n]` / `float[n]`: an explicit array in linear memory, one block [count][cells] (user 2026-10-06);
// discouraged with a hint, since the compiler picks where number lists live by itself (notes/linear_arrays.md)
use crate::common::fails_with;
use crate::is;
use warp::{float, int, ints, list};

#[test]
fn a_linear_array_reads_and_writes_its_cells() {
	is!("linear xs = int[5]; xs#2 = 7; xs#3 += 4; [xs#2, xs#3, #xs, xs#1, xs.count]", ints(vec![7, 4, 5, 0, 5]));
	is!("linear ys = float[3]; ys#1 = 1.5; ys#1 += 0.25; ys#1", 1.75);
	is!("linear xs = int[1000000]; for i in 1 to 1000000 { xs#i = i }; s = 0; for i in 1 to 1000000 { s += xs#i }; s", 500000500000i64);
}

#[test]
fn a_linear_array_is_a_list_as_a_whole() {
	is!("linear xs = int[3]; xs#1 = 4; xs", ints(vec![4, 0, 0]));
	is!("linear ys = float[2]; ys#2 = 2.5; s = 0.0; for y in ys { s += y; s += 1.0 }; [s, ys]", list(vec![float(4.5), list(vec![float(0.0), float(2.5)])]));
	is!("linear xs = int[4]; f(a) := a#2 + 1; xs#2 = 5; f(xs)", 6);
	is!("linear xs = int[2]; xs#2 = 3; t = 0; for x in xs { t += x }; t", int(3));
}

#[test]
fn a_linear_array_checks_its_bounds_and_stays_in_its_task() {
	fails_with("linear xs = int[3]; xs#4", "index out of range");
	fails_with("linear xs = int[3]; xs#0 = 1", "index out of range");
	fails_with("linear xs = int[4]; g(a) := a#1; await go g(xs)", "a task cannot take the linear array xs");
}

/// a for loop over a linear array gives its last x, as over a list (card linear-for-value)
#[test]
fn a_for_loop_over_a_linear_array_gives_its_last_value() {
	is!("linear xs = int[3]; xs#3 = 7; for x in xs { x }", int(7));
	is!("linear xs = int[3]; xs#1 = 2; r = for x in xs { x * 10 }; r", int(0));
}

// a float map of a linear float array runs as a kernel two cells at a time in f64x2 lanes (card simd-map,
// notes/simd.md): its result is a new linear array; any other body stays the general map
#[test]
fn a_float_map_of_a_linear_array_is_a_simd_kernel() {
	let fill = "linear xs = float[5]; for i in 1 to 5 { xs#i = i * i * 1.0 }; ";
	is!(&format!("{fill}ys = xs.map(x => x * 0.5 + 1); ys"), list(vec![float(1.5), float(3.0), float(5.5), float(9.0), float(13.5)]));
	is!(&format!("{fill}ys = map(xs, v => -sqrt(v) / 2); [ys#5, #ys]"), list(vec![float(-2.5), int(5)]));
	let lowered = |code: &str| warp::pipeline::lower(code).expect("a program").serialize();
	assert!(lowered(&format!("{fill}ys = xs.map(x => x * 0.5 + 1); ys#1")).contains("linear_mapf·x·x*0.5+1"));
	let k = "k = 3.0; ";
	assert!(!lowered(&format!("{fill}{k}ys = xs.map(x => x * k); ys#1")).contains("linear_mapf"));
	is!(&format!("{fill}{k}ys = xs.map(x => x * k); ys#2"), 12.0);
}

// card linear-map: a numeric map the f64x2 kernel cannot compute (sin, min …) writes a new linear block in one loop,
// and a linear array read as a whole fills a typed list of its count, no list grown item by item (that was superlinear)
#[test]
fn a_numeric_map_of_a_linear_array_writes_a_new_block() {
	let fill = "linear xs = float[3]; for i in 1 to 3 { xs#i = i * 1.0 }; ";
	is!(&format!("{fill}ys = xs.map(x => max(x, 2) * 2); [ys#1, ys#3, #ys]"), list(vec![float(4.0), float(6.0), int(3)]));
	is!(&format!("{fill}ys = xs.map(x => floor(x / 2)); ys"), list(vec![float(0.0), float(1.0), float(1.0)]));
	is!(&format!("{fill}sum(xs)"), 6.0);
	is!("linear xs = int[3]; xs#2 = 5; xs", ints(vec![0, 5, 0]));
	let lowered = warp::pipeline::lower(&format!("{fill}ys = xs.map(x => sin(x)); ys#1")).expect("a program").serialize();
	assert!(lowered.contains("linear_new") && !lowered.contains(".map"), "{lowered}");
}

// a map of a numeric map's result is a linear array too (the results found until none is added)
#[test]
fn a_numeric_map_of_a_mapped_linear_array_writes_a_block_too() {
	let program = "linear xs = float[2]; xs#2 = 1.0; ys = xs.map(x => sin(x)); zs = ys.map(y => max(y, 0.5)); [zs#1, #zs]";
	is!(program, list(vec![float(0.5), int(2)]));
	assert!(!warp::pipeline::lower(program).expect("a program").serialize().contains(".map"));
}

// a chain of numeric maps over a linear array is one loop of g after f, not a list of f's results mapped again
#[test]
fn a_chain_of_numeric_maps_of_a_linear_array_is_one_loop() {
	let program = "linear xs = float[2]; xs#2 = 1.0; zs = xs.map(x => sin(x)).map(y => max(y, 0.5)); [zs#1, #zs]";
	is!(program, list(vec![float(0.5), int(2)]));
	let lowered = warp::pipeline::lower(program).expect("a program").serialize();
	assert!(lowered.contains("linear_setf") && !lowered.contains("+["), "{lowered}");
}

// card linear-interpolation: a text hole reads a linear array's item like any other expression
#[test]
fn an_item_of_a_linear_array_reads_in_a_text_hole() {
	is!("linear xs = float[3]; xs#1 = 2.5; \"a \\(xs#1) b\"", "a 2.5 b");
	is!("linear xs = int[2]; xs#2 = 7; \"\\(#xs): ${xs#2}\"", "2: 7");
}

// `xs .* ys` of two linear arrays pairs their items by index, as of two lists (it multiplied by ys's address)
#[test]
fn element_wise_operators_pair_two_linear_arrays() {
	let filled = "linear xs = float[3]; linear ys = float[3]; xs#1 = 2.0; ys#1 = 3.0; xs#2 = 4.0; ys#2 = 0.5\n";
	is!(&format!("{filled}zs = xs .* ys; [zs#1, zs#2, #zs]"), list(vec![float(6.0), float(2.0), int(3)]));
	is!(&format!("{filled}sum(xs .* ys)"), 8.0);
	is!(&format!("{filled}dot(xs, ys)"), 8.0);
	is!(&format!("{filled}zs = xs .+ ys .* 2; zs#2"), 5.0);
	fails_with("linear xs = float[3]; linear ys = float[2]; zs = xs .* ys; zs#1", "differ in length");
	fails_with("linear xs = float[3]; linear ys = float[2]; sum(xs .* ys)", "differ in length");
}
