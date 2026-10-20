use crate::eq;
use warp::warp_parser::parse_file;
use warp::Node::*;

#[test]
// DONE: separator precedence creates nested groups, needs flattening for WIT
pub fn test_wit_parse() {
	// loop {} // Should be killed at .1
	let ast = parse_file("samples/wit/warp-ast.wit");
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
