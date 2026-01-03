use wasp::eq;
use wasp::node::Node;
use wasp::wasp_parser::parse;

#[test]
pub fn test_parser_serialize() {
	let code = "{ key: [ value, { key2: value2, num:123, text:'yeah' } ] }";
	let ast: Node = parse(code);
	let serial = ast.serialize();
	let right = "{key=[value, {key2=value2, num=123, text='yeah'}]}";
	eq!(serial, right);
	println!("serialize: {:#?}", ast);
	eq!(ast.size(), 1);
}

//#[test]
//pub fn test_tests() {
//    eq!(1, 1);
//}
