use wasp::eq;
use wasp::node::Node::*;
use wasp::wasp_parser::read;

#[test]
pub fn test_parser() {
    let ast = read("wasp-ast.wit");
    println!("serialize: {:#?}", ast.serialize());
    if let List(ref items) = ast {
        for (i, item) in items.iter().enumerate() {
            println!("item {}: {:#?}", i, item);
        }
    }
    eq!(ast.size(), 3);
}