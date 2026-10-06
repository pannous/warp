use crate::is;
use warp::wasp_parser::parse;
use warp::{Node, Op};

// foreign keyword forms (Python, JS, Ruby, PHP) compile to their intent; snippets in probes/parse/

#[test]
fn else_if_chains_another_if() {
	is!("x=0\nr=1\nif r == 0 { x += 1 } else if r == 2 { x += 10 }\nx", 0);
	is!("x=0\nr=2\nif r == 0 { x += 1 } else if r == 2 { x += 10 }\nx", 10);
	is!("x = 0\n{\nr = 2\nif r == 0 {\n x += 1\n} else if r == 2 {\n x += 10\n} else {\n x += 100\n}\n}\nx", 10);
	is!("fun f(n) {\n  if n == 0 { return 0 } else if n == 1 { return 10 } else { return 20 }\n}\nf(0) + f(1) + f(2)", 30);
}

#[test]
fn elif_elsif_and_elseif_are_else_if() {
	is!("x = 0\nn = 3\nif n == 2 { x = 1 } elif n == 3 { x = 2 } else { x = 3 }\nx", 2);
	is!("x = 0\nn = 1\nif n == 2 { x = 1 } elsif n == 3 { x = 2 } else { x = 3 }\nx", 3);
	is!("x = 0\nn = 3\nif n == 2 { x = 1 } elseif n == 3 { x = 2 }\nx", 2);
	is!("x = 0\nn = 3\nif n == 2: x = 1 elif n == 3: x = 2 else: x = 3\nx", 2);
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
	let membership = |node: &Node| matches!(node.drop_meta(), Node::List(items, _, _) if items.len() == 3 && items[1].name() == "in");
	let Node::Key(if_then, Op::Else, _) = parse("if \"Z\" in v { 1 } else { 0 }").drop_meta().clone() else { panic!("if … else expected") };
	let Node::Key(if_condition, Op::Then, body) = if_then.drop_meta() else { panic!("if … then expected, got {if_then:?}") };
	let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else { panic!("if condition expected, got {if_condition:?}") };
	assert!(membership(condition), "membership condition, got {condition:?}");
	assert!(matches!(body.drop_meta(), Node::List(_, warp::Bracket::Curly, _)), "block body, got {body:?}");
	let Node::Key(_, Op::Assign, value) = parse("x = \"Z\" in v").drop_meta().clone() else { panic!("assignment expected") };
	assert!(membership(&value), "the whole membership is assigned, got {value:?}");
	is!("number of chars in \"héllo\"", 5); // a unit word keeps the counting phrase
}

#[test]
fn for_destructures_a_tuple_or_comma_names() {
	let parsed = parse("for (r, c) in [(0, 1), (2, 3)] { x += r*10+c }");
	let Node::List(items, _, _) = parsed.drop_meta() else { panic!("for loop expected, got {parsed:?}") };
	assert!(matches!(items[1].drop_meta(), Node::List(names, _, _) if names.len() == 2), "loop names, got {:?}", items[1]);
	assert!(matches!(items[3].drop_meta(), Node::List(elements, _, _) if elements.len() == 2), "iterable, got {:?}", items[3]);
	let parsed = parse("for k, v in m { x += v }");
	let Node::List(items, _, _) = parsed.drop_meta() else { panic!("for loop expected, got {parsed:?}") };
	assert!(matches!(items[1].drop_meta(), Node::List(names, _, _) if names.len() == 2), "loop names, got {:?}", items[1]);
}
