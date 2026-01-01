use wasp::eq;
use wasp::node::Node::*;
use wasp::wasp_parser::read;

#[test]
pub fn test_parser() {
    let ast = read("test.wasp");
    println!("AST: {:#?}", ast);
    println!("serialize: {:#?}", ast.serialize());

    // The parsed structure should be a Tag node with name "html"
    if let Tag { title, params, body } = ast {
        eq!(title, "html");
        println!("Tag title: {}", title);
        println!("Params: {:#?}", params);
        println!("Body: {:#?}", body);
    } else {
        panic!("Expected Tag node, got: {:?}", ast);
    }
}