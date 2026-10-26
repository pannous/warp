//! Card self-node-api: warp code reads and builds Nodes, as a parser written in warp must (notes/self_hosting.md):
//! a key's `.key`, `.value`, `.op`, a word's or key's `.name`, a list's `.bracket` (a field of that name wins), and
//! `symbol(text)`, `key(left, op, right)`, `bracketed(items, bracket)` to build them
use crate::is;

#[test]
fn a_key_shows_its_parts() {
	is!("p = data (x: 3); p.key", warp::parse("x"));
	is!("p = data (x: 3); p.value", 3);
	is!("p = data (x: 3); p.op", ":");
	is!("k = data a+b; k.op", "+");
	is!("k = data a+b; k.value", warp::parse("b"));
	is!("p = data (x: 3); p.name", "x");
	is!("s = data foo; s.name", "foo");
	// a field of that name wins
	is!("p = {name: 3}; p.name", 3);
	is!("p = {key: 3}; p.key", 3);
}

#[test]
fn a_list_shows_its_bracket() {
	is!("c = data f(1, 2); c.bracket", "(");
	is!("c = data [a b]; c.bracket", "[");
	is!("c = data {a b}; c.bracket", "{");
}

#[test]
fn nodes_are_built_from_parts() {
	is!("symbol(\"x\")", warp::parse("x"));
	is!("name = \"xy\"; s = symbol(name); [type(s), s.name]", warp::parse("[symbol \"xy\"]"));
	is!("op = \"*\"; key(2, op, 3)", warp::parse("2*3"));
	is!("k = key(symbol(\"ab\"), \":\", 1); [k.name, k.value, type(k)]", warp::parse("[\"ab\" 1 key]"));
	is!("g = bracketed([symbol(\"fg\"), 1, 2], \"(\"); [#g, g#1.name]", warp::parse("[3 \"fg\"]"));
	is!("g = bracketed([1, 2], \"(\"); g.bracket", "(");
	is!("key(1, \"@@\", 2)", warp::error("unknown operator: @@"));
}

/// A rebracketed or reseparated list is that list in every cell, so == sees it as the written one (card bracketed-equality)
#[test]
fn rebuilt_lists_equal_the_written_ones() {
	is!("bracketed([1, 2], \"(\") == (1, 2)", true);
	is!("bracketed([1, 2, 3], \"{\") == {1, 2, 3}", true);
	is!("separated([1, 2, 3], \" \") == [1 2 3]", true);
	is!("separated(bracketed([1, 2, 3], \"(\"), \" \") == (1 2 3)", true);
	is!("bracketed([1, 2], \"(\") == [1, 2]", false);
}

/// `symbol("word")` of a literal is one symbol per word and module (card speed-symbol-calls): made again and again it
/// stays equal to itself and to the symbol of the word made at run time
#[test]
fn literal_symbols_are_made_once() {
	is!("f(n: Node): bool := n == symbol(\"print\"); f(symbol(\"print\"))", true);
	is!("f(n: Node): bool := n == symbol(\"print\"); f(symbol(\"other\"))", false);
	is!("f(n: Node): bool := n == symbol(\"print\"); f(data print)", true);
	is!("w = \"pr\" + \"int\"; n = 0; for i in 0..3 { if symbol(w) == symbol(\"print\") { n += 1 } }; n", 3);
	is!("s = symbol(\"xy\"); t = symbol(\"xy\"); s == t", true);
	is!("s = symbol(\"xy\"); symbol(\"xy\"); s.name", "xy");
}
