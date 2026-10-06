// Python's `list(x)`: the list itself, the characters of a text (as `x as list`)
use warp::wasm_emitter::eval;

#[test]
fn list_of_a_value() {
	assert_eq!(eval("def f(x): return x * 2; list(map(f, [1, 2, 3]))").serialize(), "[2 4 6]");
	assert_eq!(eval("xs = [1, 2]; list(xs)").serialize(), "[1 2]");
	assert_eq!(eval("list(\"ab\")").serialize(), "['a' 'b']");
}

#[test]
fn an_empty_list() {
	assert_eq!(eval("xs = list(); xs.add(3); xs").serialize(), "[3]");
}
