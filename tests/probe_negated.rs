use warp::analyzer::lower_negated_calls;
use warp::wasp_parser::WaspParser;
use warp::Node;

fn shape(node: &Node) -> String {
	match node {
		Node::Key(left, op, right) => format!("Key({}, {op:?}, {})", shape(left), shape(right)),
		Node::List(items, _, _) => format!("List[{}]", items.iter().map(shape).collect::<Vec<_>>().join(", ")),
		Node::Meta { node, .. } => format!("Meta({})", shape(node)),
		Node::Symbol(name) => format!("Sym({name})"),
		Node::Number(number) => format!("Num({number:?})"),
		other => format!("{other:?}"),
	}
}

#[test]
fn probe_negated() {
	let mut report = String::new();
	for code in ["double -3", "double(x):=x*2;double -3"] {
		let parsed = WaspParser::parse(code);
		report += &format!("PROBE {code:?}\n  {}\n  {}\n", shape(&parsed), shape(&lower_negated_calls(parsed.clone())));
	}
	panic!("{report}");
}
