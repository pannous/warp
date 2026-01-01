use wasp::eq;
use wasp::node::Node::*;
use wasp::wasp_parser::read;

#[test]
pub fn test_parser() {
    let ast = read("test.wasp");
    println!("AST: {:#?}", ast);
    println!("serialize: {:#?}", ast.serialize());

    // The parsed structure should be a Tag node with name "html"
    match &ast {
        Tag { title, params, body } => {
            eq!(title, "html");
            println!("Tag title: {}", title);
            println!("Params: {:#?}", params);
            println!("Body: {:#?}", body);
        }
        WithMeta(node, meta) => {
            println!("AST is wrapped in WithMeta, meta: {:?}", meta);
            if let Tag { title, .. } = node.as_ref() {
                eq!(title, "html");
                println!("Tag title: {}", title);
            } else {
                panic!("Expected Tag node inside WithMeta, got: {:?}", node);
            }
        }
        other => panic!("Unexpected node variant: {:?}", other),
    }
}