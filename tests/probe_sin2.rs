use warp::wasm_emitter::eval;
use warp::wasp_parser::WaspParser;
use warp::Node;

fn shape(node: &Node) -> String {
	match node {
		Node::Key(left, op, right) => format!("Key({}, {op:?}, {})", shape(left), shape(right)),
		Node::List(items, _, _) => format!("List[{}]", items.iter().map(shape).collect::<Vec<_>>().join(", ")),
		Node::Meta { node, .. } => shape(node),
		Node::Symbol(name) => format!("Sym({name})"),
		Node::Number(number) => format!("Num({number:?})"),
		other => format!("{other:?}"),
	}
}

#[test]
fn probe_sin2() {
	let mut report = String::new();
	for code in ["sin 2π", "sin -π/2", "sin 3*π/2"] {
		report += &format!("PROBE {code:?} {}\n", shape(&WaspParser::parse(code)));
	}
	for code in ["use sin;sin 2π", "use sin;sin(2π)", "use sin;sin 2*π", "use sin;sin -π/2", "use sin;sin(-π/2)", "use sin;sin 3*π/2"] {
		report += &format!("PROBE {code:?} => {:?}\n", eval(code));
	}
	panic!("{report}");
}
