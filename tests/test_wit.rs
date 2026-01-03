use wasp::eq;
use wasp::node::Node::*;
use wasp::wasp_parser::parse_file;

#[test]
fn test_timeout_protection() {
	let do_test_timeout_protection = false;
	if do_test_timeout_protection {
		loop {} // Should be killed at .1s
	}
}

#[test]
pub fn test_wit_parse() {
	// loop {} // Should be killed at .1
	let ast = parse_file("wasp-ast.wit");
	println!("serialize: {:#?}", ast.serialize());
	if let List(ref items, _, _) = ast {
		for (i, item) in items.iter().enumerate() {
			println!("item {}: {:#?}", i, item);
		}
	}
	eq!(ast.size(), 3);
	eq!(ast[0].name(), "package");
	eq!(ast[1].name(), "interface");
	eq!(ast[2].name(), "world");
}
