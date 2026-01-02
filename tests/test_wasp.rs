// use wasp::Parser;
// use crate::parser::wasp::*;
use wasp;
use wasp::node::Node;
use wasp::parser::*;
// use super::parser;
// use wasp::parser;
// mod parser;
// use parser::wasp::*;

#[test]
pub fn test_parser() {
    let code = "{ key: [ value, { key2: value2, num:123, text:'yeah' } ] }";
    let mut parser = Parser::new(code);
    let ast:Node = parser.parse();
    let serial = ast.serialize();
    // let right= "key: {[value, {[key2: value2, num: 123, text: 'yeah']}]}";
    let right="key={[value, {[key2=value2, num=123, text='yeah']}]}";
    eq!(serial, right);
    println!("serialize: {:#?}", ast);
    eq!(ast.size(), 1);
    // ast["key"]

}

//#[test]
//pub fn test_tests() {
//    eq!(1, 1);
//}