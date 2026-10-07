// P35 defaults (night 2026-10-04): `m.remove(k)` removes the key from the map variable and gives its value (Python
// dict.pop); `xs.index_of(x)` is the 1-based position like `x in xs`, 0 when absent
use crate::is;

#[test]
fn remove_takes_a_key_out_of_a_map_variable() {
	is!("m = {a:1 b:2}; v = m.remove(\"a\"); v * 10 + #m.keys()", 11);
	is!("m = {a:1 b:2}; m.remove(\"b\"); m.has(\"b\")", 0);
	is!("m = {a:1 b:2}; m.remove(\"b\"); m.a", 1);
}

#[test]
fn index_of_is_the_one_based_position() {
	is!("xs = [5 6 7]; xs.index_of(6)", 2);
	is!("xs = [5 6 7]; xs.index_of(9)", 0);
	is!("\"abc\".index_of(\"c\")", 3);
}

#[test]
fn a_stack_built_from_empty_pops() {
	is!("rpn(tokens) := { st = []; for t in tokens { if t == \"+\" { b = st.pop(); a = st.pop(); st.add(a + b) } else { st.add(t as int) } }; st#1 }; rpn([\"3\" \"4\" \"+\"])", 7);
}

/// card list-remove: on a list variable `xs.remove(v)` takes out the first element equal to v and gives the list
/// (tests/lists/test_lists.rs `pixel.remove(3)`); it silently changed nothing
#[test]
fn remove_takes_a_value_out_of_a_list_variable() {
	is!("xs = [\"a\", \"b\", \"c\"]; xs.remove(\"a\"); count(xs)", 2);
	is!("xs = [\"a\", \"b\", \"c\"]; xs.remove(\"a\"); xs#1", "b");
	is!("xs = [1, 2, 3, 2]; xs.remove(2); xs", warp::ints(vec![1, 3, 2]));
	is!("pixel = [1 2 3]; pixel.remove(3)", warp::ints(vec![1, 2]));
}
