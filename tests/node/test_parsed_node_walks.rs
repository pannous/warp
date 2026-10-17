//! Cards parsed-kinds, built-key-print, bracketed-empty, parse-const-fold: what a compiler pass written in warp needs of
//! parsed and built nodes (notes/self_hosting_readiness.md): kind tests and dispatch, printing, rebuilding
use crate::is;

/// Code in braces is a list of its statements, as code in any other brackets; it is no map
#[test]
fn parsed_braces_are_lists() {
	is!("g(n) := n is list; g(parse(\"{a; b}\"))", true);
	is!("g(n) := n is list; g(parse(\"{a: b}\"))", true);
	is!("g(n) := n is map; g(parse(\"{a: b}\"))", false);
	is!("n = parse(\"{a; b}\"); n is list", true);
	is!("n = parse(\"{a; b; c}\"); n.map(x => x.name) == [\"a\", \"b\", \"c\"]", true);
	is!("n = parse(\"{a; b}\"); out = []; for x in n { out += [x.name] }; out == [\"a\", \"b\"]", true);
}

/// A walk dispatches on the kind of every node it is given, the same function for symbols and keys
#[test]
fn kinds_dispatch_parsed_nodes() {
	let kind_of = "kind_of(n) := match n { symbol => 'symbol'; key => 'key'; list => 'list'; _ => 'other' }; ";
	is!(
		&format!("{kind_of}[kind_of(parse(\"x\")), kind_of(parse(\"x + 1\")), kind_of(parse(\"[a]\"))]"),
		warp::parse("['symbol' 'key' 'list']")
	);
	is!("f(n) := n; [f(data x), f(data x+1)]", warp::parse("[x x+1]"));
}

/// A node that is no group has no bracket and no separator: ø
#[test]
fn only_groups_have_brackets() {
	is!("parse(\"x\").bracket", warp::Node::Empty);
	is!("parse(\"1 + 2\").separator", warp::Node::Empty);
	is!("parse(\"f(x)\").bracket", "(");
}

/// Parsed and built nodes print as their code
#[test]
fn nodes_print_as_code() {
	is!("n = parse(\"x\"); \"<\\(n)>\"", "<x>");
	is!("k = key(symbol(\"y\"), \"+\", parse(\"z\")); \"\\(k.value)\"", "z");
	is!("\"\\(key(symbol(\"y\"), \"+\", symbol(\"z\")))\"", "y+z");
	is!("op = \"*\"; \"\\(key(symbol(\"y\"), op, symbol(\"z\")))\"", "y*z");
}

/// No items is the empty list ø whatever the bracket, as `()` parses
#[test]
fn an_empty_group_is_empty() {
	is!("bracketed([], \"(\")", warp::Node::Empty);
	is!("separated([], \",\")", warp::Node::Empty);
	is!("bracketed([], \"(\") == parse(\"()\")", true);
}

/// Quoted code defines nothing: its calls name no function of the program
#[test]
fn quoted_definitions_define_nothing() {
	is!("r(n) := n; r(parse(\"f(x) := g(x - 1)\")).op", ":=");
	is!("q = data f(x) := g(x - 1); q.op", ":=");
}

/// Card parse-typedef: a source declaring a type parses, its declaration a node that prints as Node::serialize writes it
#[test]
fn parsed_type_declarations_are_nodes() {
	is!("n = parse(\"class P { x: int }\"); \"\\(n)\"", "type P {x:type int ø}");
	is!("t = \"class P { x: int }\"; n = parse(t); \"\\(n)\"", "type P {x:type int ø}");
	is!("t = \"class P { x: int }\\nf(a) := a\"; #parse(t)", 2);
	is!("n = parse(read(\"lib/compiler/parser.warp\")); #n > 50", true);
}
