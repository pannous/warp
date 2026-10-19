use crate::is;

// foreign keyword forms (Python, JS, Ruby, PHP) compile to their intent

#[test]
fn else_if_chains_another_if() {
	is!("x=0\nr=1\nif r == 0 { x += 1 } else if r == 2 { x += 10 }\nx", 0);
	is!("x=0\nr=2\nif r == 0 { x += 1 } else if r == 2 { x += 10 }\nx", 10);
	is!("x = 0\n{\nr = 2\nif r == 0 {\n x += 1\n} else if r == 2 {\n x += 10\n} else {\n x += 100\n}\n}\nx", 10);
	is!(
		"fun f(n) {\n  if n == 0 { return 0 } else if n == 1 { return 10 } else { return 20 }\n}\nf(0) + f(1) + f(2)",
		30
	);
}

#[test]
fn elif_elsif_and_elseif_are_else_if() {
	is!("x = 0\nn = 3\nif n == 2 { x = 1 } elif n == 3 { x = 2 } else { x = 3 }\nx", 2);
	is!("x = 0\nn = 1\nif n == 2 { x = 1 } elsif n == 3 { x = 2 } else { x = 3 }\nx", 3);
	is!("x = 0\nn = 3\nif n == 2 { x = 1 } elseif n == 3 { x = 2 }\nx", 2);
	is!("x = 0\nn = 3\nif n == 2: x = 1 elif n == 3: x = 2 else: x = 3\nx", 2);
	is!("x = 0\nn = 3\nif n == 2: x = 1 elif n == 3: x = 2\nx", 2);
}

#[test]
fn in_is_a_value_and_a_condition() {
	is!("v = [\"A\"]\nx = \"Z\" in v\nx", false);
	is!("v = [\"A\"]\nif \"Z\" in v { 1 } else { 0 }", 0);
}

#[test]
fn else_takes_a_python_colon() {
	is!("x=0\nn=3\nif n == 2: x = 1 else: x = 3\nx", 3);
}

#[test]
fn return_takes_the_whole_expression() {
	is!("fun f() {\n  return -1\n}\nf()", -1);
	is!("fun f(n) {\n  return n * 2 + 1\n}\nf(3)", 7);
	is!("fun f() {\n  return [1, 2, 3]\n}\nf()#3", 3);
	is!("fun f(x) {\n  if x > 1 { return }\n  x\n}\nf(1)", 1);
}

#[test]
fn a_counting_word_after_return_is_the_variable() {
	is!("fun f(i) { count = 7; return count }\nf(10)", 7);
	is!("fun f(i) {\n  count = 0\n  count = count + i\n  return count\n}\nf(5)", 5);
	is!("xs = [1, 2, 3]\nxs count", 3); // still the property of a list
}

#[test]
fn a_space_before_a_bracket_makes_a_list() {
	is!("f(xs) := xs#2\nf [5, 6, 7]", 6);
	is!("xs = [5, 6, 7]\nxs[1]", 6);
}

#[test]
fn membership_in_binds_like_a_comparison() {
	is!("v = \"XYZ\"; if \"Z\" in v { 1 } else { 0 }", 1);
	is!("v = \"XY\"; if \"Z\" in v { 1 } else { 0 }", 0);
	is!("v = \"XYZ\"; x = \"Z\" in v; x", true); // the whole membership is assigned
	is!("number of chars in \"héllo\"", 5); // a unit word keeps the counting phrase
}

#[test]
fn for_destructures_a_tuple_or_comma_names() {
	is!("x = 0; for (r, c) in [(0, 1), (2, 3)] { x += r*10+c }; x", 24);
	is!("m = {a: 1, b: 2}; x = 0; for k, v in m { x += v }; x", 3);
}
