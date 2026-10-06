// `float[n]` declares floats: any number written into it is stored as a float, in place, so filling a large one in a
// loop is linear (it was a list of Nodes rebuilt per write, and 100000 writes exhausted the call stack; card sized-array)
use crate::is;

#[test]
fn numbers_written_into_a_float_array_are_floats() {
	is!("xs = float[3]; xs#2 = 2.5; xs#3 = 4; xs", warp::floats(vec![0.0, 2.5, 4.0]));
	is!("xs = float[100000]; for i in 1 to 100000 { xs#i = i }; xs#90000", 90000.0);
	is!("xs = float[100000]; for i in 1 to 100000 { xs#i = i * 1.5 }; xs#2", 3.0);
}
