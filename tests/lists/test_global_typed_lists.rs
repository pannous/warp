// A list main builds and functions read is a global; a global list of numbers is a typed array like a local one
// (card compiler-picks): indexing it in a function is O(1), building it by appends is amortised O(1)
use crate::is;
use warp::{int, ints, list};

#[test]
fn a_global_number_list_read_by_a_function_is_an_array() {
	// as cons cells: 10^5 appends and 10^5 indexed reads ran out of fuel
	is!("n = 100000; xs = []; for i in 1..n { xs.add(i) }; at(i) := xs#i; s = 0; for i in 1..n { s += at(i) }; s", 4999950000i64);
	is!("xs = [1, 2, 3]; size() := count xs; total() := sum xs; [size(), total()]", ints(vec![3, 6]));
	is!("xs = int[5]; for i in 1 to 5 { xs#i = i * i }; f(i) := xs#i; f(4)", 16);
	is!("xs = []; for i in 1 to 4 { xs.add(i) }; doubled() := { s = 0; for x in xs { s += 2 * x }; s }; doubled()", 20);
}

#[test]
fn a_global_typed_list_reads_as_its_list() {
	is!("xs = [1, 2, 3]; f() := xs; f()", ints(vec![1, 2, 3]));
	is!("xs = [1, 2]; xs.add(3); g() := count xs; [g(), xs]", list(vec![int(3), ints(vec![1, 2, 3])]));
	is!("xs = [1, 2, 3]; ys = xs; ys#1 = 9; f() := xs#1 + ys#1; f()", 10);
}

/// a function that changes the global list keeps it a Node list, as before
#[test]
fn a_global_list_a_function_changes_stays_as_before() {
	is!("global xs = [1, 2, 3]; bump() := { xs#1 = 7 }; bump(); xs#1", 7);
	is!("global xs = []; push(v) := { xs.add(v) }; push(4); push(5); xs", ints(vec![4, 5]));
	is!("global xs = [1, 2, 3]; f(i) := xs#i; f(2)", 2);
}

/// card global-comprehension: a global list of floats built by a comprehension, by appends or by index, changed by a
/// function element by element, keeps its floats (was "text + int", "not an int", "cast failure")
#[test]
fn a_global_float_list_a_function_changes_keeps_its_floats() {
	let changed_by_g = "def g() { global hb; hb[1] = hb[1] + 0.5 }; g(); hb[1] >= 0.5 and hb[1] < 1.5";
	is!(&format!("hb = [random() for j in 0..4]; {changed_by_g}"), true);
	is!(&format!("hb = []; for j in 0..4 {{ hb.push(random()) }}; {changed_by_g}"), true);
	is!(&format!("hb = [0.0, 0.0, 0.0]; for j in 0..3 {{ hb[j] = random() }}; {changed_by_g}"), true);
	is!("hb = [random() for j in 0..4]; def f() { t = hb[1]; t + 1 }; def g() { global hb; hb[1] = 0.25 }; g(); f()", 1.25);
	is!("hb = [random() for j in 0..4]; def g() { global hb; hb[1] = 0.75 }; g()", 0.75);
}

/// an exact list given a float or a character by index holds it as it is
#[test]
fn an_element_of_another_type_widens_the_list() {
	is!("xs = [0, 0]; xs[0] = random(); xs[0] < 1", true);
	is!("xs = [0, 0]; xs[0] = random(); type(xs)", "list of float");
	is!("xs = [1, 2]; xs[0] = 2.5; print(xs[0] = 2.5); string(xs)", "[2.5 2]");
	is!("xs = [1, 2]; xs[0] = 'a'; string(xs)", r#"["a" 2]"#);
}

/// card global-arrays: a global array a function writes by index stays an array (each write rebuilt a Node list, O(n),
/// so 20000 writes ran out of fuel); writes of another element type or whole assignments keep the Node list
#[test]
fn a_global_array_a_function_writes_stays_an_array() {
	is!("global s = int[20000]; def f() { for i in 0..20000 { s[i] = i } }; f(); s[19999]", 19999);
	is!("global s = int[5]; def f(v) { s[2] = v; s[3] += 2 }; f(7); f(1); s", ints(vec![0, 0, 1, 4, 0]));
	is!("global s = float[3]; def f() { s[1] = 1.5 }; f(); s[1]", 1.5);
	is!("global s = int[3]; def f() { s[1] = 'a' }; f(); [s[0], s[1] == 'a']", ints(vec![0, 1]));
	is!("global s = int[3]; def f() { s = [4, 5] }; f(); s", ints(vec![4, 5]));
}
