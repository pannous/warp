use warp::wasm_emitter::eval;
use warp::{parse, Bracket, Node, Separator};

fn b(x: &Bracket) -> &'static str {
	match x { Bracket::Curly => "{}", Bracket::Square => "[]", Bracket::Round => "()", Bracket::Less => "<>", _ => "-" }
}
fn sp(x: &Separator) -> &'static str {
	match x { Separator::Space => "sp", Separator::Colon => ",", Separator::Semicolon => ";", Separator::Newline => "nl", _ => "-" }
}

fn dump(n: &Node) -> String {
	match n {
		Node::Key(a, op, b) => format!("Key({}, {:?}, {})", dump(a), op, dump(b)),
		Node::List(items, br, sep) => format!("List{}{}[{}]", b(br), sp(sep), items.iter().map(dump).collect::<Vec<_>>().join(", ")),
		Node::Meta { node, data } => format!("Meta({}, {})", dump(node), dump(data)),
		Node::Symbol(s) => format!("Sym({s})"),
		other => format!("{other:?}"),
	}
}

#[test]
fn probe() {
	let mut out = String::new();
	for code in [
		"x:list<int>=[1 2]",
		"x:list of int=[1 2]",
		"x:list of int",
		"x:list<int>",
		"x:ints=[1 2]",
		"list<int>",
		"a<b",
		"a<b>",
		"x:list of int=[1 2]; x",
		"x:list of int=[1 2]; type(x)",
		"x:list<int>=[1 2]; type(x)",
		"x=[1 2 3]; x<int>",
		"x=[1 2 3]; x<2",
		"a=1;b=2;a < b",
		"a=1;b=2;a<b",
		"1<<3",
		"x:ints=[1 2]; type(x)",
		"type([1 2 3])",
	] {
		let tree = std::panic::catch_unwind(|| dump(&parse(code)));
		let value = std::panic::catch_unwind(|| format!("{:?}", eval(code)));
		out += &format!("CODE {code}\n  PARSE {tree:?}\n  EVAL {value:?}\n");
	}
	panic!("PROBEOUT\n{out}");
}
