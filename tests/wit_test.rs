use wasp::eq;
use wasp::wasp_parser::read;

#[test]
pub fn test_parser() {
    let ast = read("wasp-ast.wit");
    let serial = ast.serialize();
    println!("serialize: {:#?}", serial);
    eq!(ast.size(), 3);
}