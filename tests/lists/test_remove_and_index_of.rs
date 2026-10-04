// P35 defaults (night 2026-10-04): `m.remove(k)` removes the key from the map variable and gives its value (Python
// dict.pop); `xs.index_of(x)` is the 1-based position like `x in xs`, 0 when absent
use warp::is;

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
