use crate::is;

// a variable bound to a list element takes the element's type: nested lists, texts, characters
#[test]
fn variables_bound_to_elements_keep_their_type() {
	is!("g = [[1, 4], [3, 2]]; i = 1; e = g[i]; e[1]", 2);
	is!("names = [\"Aa\", \"Bb\"]; p = names[1]; p", "Bb");
	is!("names = [\"A\", \"B\"]; p = names[1]; p", 'B');
	is!("names = ['A', 'B']; p = names[1]; p", 'B');
	is!("names = \"ABCDEF\"; p = names[1]; p", 'B');
	is!("xs = [1.5, 2.5]; x = xs[1]; x", 2.5);
}

// the loop variable over a list of lists is a list
#[test]
fn loop_variable_over_nested_lists() {
	is!("edges = [[1, 4], [3, 2]]; s = 0; for e in edges { s = s + e[1] }; s", 6);
	is!("s = 0; for e in [[1, 4], [3, 2]] { s = s + e[1] }; s", 6);
	is!("x = 0; for p in [[0, 1], [2, 3]] { x += p[1] }; x", 4);
}

// `[xs#2]` is the one-element list of that element, also when appended
#[test]
fn bracketed_elements_stay_list_items() {
	is!("g = [[0, 1], [2, 3]]; x = [g[1]]; x.count", 1);
	is!("g = [[0, 1], [2, 3]]; n = []; n.push(g[1]); n[0][1]", 3);
	is!("g = [[0, 1], [2, 3]]; x = [g[1][0], g[0][1]]; x[0] * 10 + x[1]", 21);
}

// `return [1, 2]` returns the list, it does not index `return`
#[test]
fn returned_list_literal() {
	is!("fun f() { return [1, 2] }; r = f(); r[1]", 2);
	is!("fun f() { return [1, 2] }; f()[1]", 2);
	is!("fun f(x) { return [x, 2] }; f(1)[0]", 1);
	is!("fun d() { return [[1, 2], [3]] }; result = d(); r = result[0]; r[1]", 2);
	is!("f() := [1, 2]; count(f())", 2);
	is!("ins(items) := { for i in 1..3 { x = 1 }; items }; count(ins([3, 1, 2]))", 3);
}

// "A" lexes as the character 'A', yet it equals and looks up like the text "A"
#[test]
fn one_character_texts_as_keys_and_arguments() {
	is!("lookup(m, k) := m[k]; lookup({\"A\": 7}, \"A\")", 7);
	is!("lookup(m, k) := m[k]; lookup({A: 7}, \"A\")", 7);
	is!("fun lookup(m, k) { return m[k] }; lookup({\"A\": 7}, \"A\")", 7);
	is!("fun lookup(m, k) { return m[k] }; lookup({\"AB\": 7}, \"AB\")", 7);
	is!("f(m, k) := m#1 == k; f([\"A\"], \"A\")", 1);
	is!("x = \"AB\"; x#1 == \"A\"", 1);
}
