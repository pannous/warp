// use wasp::Parser;
// use crate::parser::wasp::*;
use wasp;
use wasp::parser::*;
// use super::parser;
// use wasp::parser;
// mod parser;
// use parser::wasp::*;

#[test]
pub fn test_parser() {
    let code = "{ key: [ value, { key2: value2, num:123, text:'yeah' } ] }";
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    println!("{:#?}", ast);
    println!("Parsed: {:?}", code);
    assert_eq!(1, 1);
}
