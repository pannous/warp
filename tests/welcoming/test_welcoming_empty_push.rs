use crate::is;

// an empty list takes the type of what is first pushed to it, also inside a function returning one of its elements
#[test]
fn an_empty_list_is_typed_by_its_first_push() {
	is!("fun g() { xs = []; xs.push(5); return xs[0] }; g() + 1", 6);
	is!("fun g() { var xs = []; xs.push(5); return xs[0] }; var t = 0; t += g(); t", 5);
	is!("fun g(n) { var xs = []; xs.push(n); return xs[0] }; g(2) + g(3)", 5);
	is!("fun g() { xs = []; xs.push(\"ab\"); return xs[0] }; g()", "ab");
	is!("xs = []; xs.push([1, 2]); ys = xs[0]; ys[1]", 2);
}

// the natural two-row Levenshtein: `prev = []` grows by push, `prev[n]` is returned and summed
#[test]
fn levenshtein_with_a_pushed_row() {
	is!("fun lev(a, b) { var n = b.length; var prev = []; for j in 0...n { prev.push(j) }; return prev[n] }; var t = 0; t += lev(\"ab\", \"abc\"); t", 3);
}
