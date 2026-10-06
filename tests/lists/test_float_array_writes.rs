// `float[n]` declares floats: any number written into it is stored as a float, in place, so filling a large one in a
// loop is linear (it was a list of Nodes rebuilt per write, and 100000 writes exhausted the call stack; card sized-array)
use crate::is;

#[test]
fn numbers_written_into_a_float_array_are_floats() {
	is!("xs = float[3]; xs#2 = 2.5; xs#3 = 4; xs", warp::floats(vec![0.0, 2.5, 4.0]));
	is!("xs = float[100000]; for i in 1 to 100000 { xs#i = i }; xs#90000", 90000.0);
	is!("xs = float[100000]; for i in 1 to 100000 { xs#i = i * 1.5 }; xs#2", 3.0);
}

/// A for loop over a typed array reads its items directly (wasm_emitter walking_counter, bounded_counters): nested
/// loops, an early break and a list grown in the body keep their values
#[test]
fn walking_a_typed_array_reads_every_item() {
	is!("xs = float[1000]; for i in 1 to 1000 { xs#i = i }; s = 0.0; for x in xs { s += x }; s", 500500.0);
	is!("xs = [1, 2, 3]; ys = [10, 20]; t = 0; for x in xs { for y in ys { t += x * y } }; t", 180);
	is!("xs = [1, 2, 3, 4]; t = 0; for x in xs { if x == 3 { break }; t += x }; t", 3);
	is!("xs = [1, 2, 3]; out = []; for x in xs { out.add(x * 2) }; out", warp::ints(vec![2, 4, 6]));
}
