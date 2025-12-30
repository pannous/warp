use wasp::*;
use wasp::node::Node;
// NO, test are their OWN crate!

// extern crate wasp;
#[allow(dead_code)]
// use crate::extensions::strings::*; // NO, test are their OWN crate!
// use super::extensions::strings::*;  // NO, test are their OWN crate!

// #[path = "../src/extensions.rs"] mod extensions;
// #[path = "../src/extensions/strings.rs"] mod strings;
// use strings::*;  // NO, test are their OWN crate!
// use extensions::*;
// extern crate wasp;
// use wasp::extensions::*;
// use wasp::extensions::strings::*;  // NO, test are their OWN crate!
// just soft-linked for now ;)


#[test]
fn test_string_substring() {
    init_lib();
    let s = "hello 🌍";
    let sub = s.substring(3, 5);
    put!("substring ", sub);
    assert_eq!(sub, "lo");
}

#[test]
fn test_string_substring_from() {
    init_lib();
    let s = "hello 🌍";
    // let sub = s.from(3); // reserved for String.from("…") constructor
    // let sub = s.start(3); // ugly! just learn:
    let sub = &s[3..]; 
    put!("substring from 3", sub);
    assert_eq!(sub, "lo 🌍");
}


#[test]
fn test_string_at() {
    init_lib();
    let s = "hello 🌍";
    let sub = s.at(3);
    put!("substring from 3", sub);
    assert_eq!(sub, 'l');
    // assert_eq!(sub, "l");
}


#[test]
fn test_string_from(){
    init_lib();
    let s = "hello 🌍";
    let sub = s.after("ell");
    assert_eq!(sub, "o 🌍");
}

#[test]
fn test_string_set_at() {
    init_lib();
    let s = "hello 🌍";
    let sub = s.set(1, 'a');
    assert_eq!(sub, "hallo 🌍");
}


#[test]
fn test_first_char() {
    let s = "hello 🌍";
    let c = s.first_char();
    put!("first_char: ", c);
    assert_eq!(c, 'h');
    assert_eq!(s.at(1),'e');
    assert_eq!(s.char(1),'e');
    assert_eq!(s.last_char(),'🌍');
    // assert_eq!(-1%3,2);
    assert_eq!(s.at(-1),'🌍');
}

#[test]
fn test_reverse() {
    let s = "hello 🌍";
    let rev = s.reverse();
    put!("reverse ", &rev);
    assert_eq!(rev, "🌍 olleh");
}

#[test]
fn test_interpolation() {
    let _world = "🌍";
    let s = format!("hello {_world}");
    assert_eq!(s, "hello 🌍");
}

#[test]
fn test_map() {
    // custom .to_uppercase()
    let upper = "hello 🌍".map(|c| c.upper());
    put!("upper ", &upper);
    assert_eq!(upper, "HELLO 🌍");
}
// }

fn eval(s:String) -> Node {
    wasp::wasp_parser::WaspParser::parse(&s).unwrap_or_else(|_| Node::symbol(&s))
}

#[test]
fn test_check(){
    eq!(4,4);
    is!("3.0",3.0);
    is!("3",3);
    is!('🍏','🍏');
    is!("🍏",'🍏');// !
    is!("hello","hello");// goes through eval! may serialize and deserialize wasm ;)
    let a=3;
    assert!(a==3);
}

// #[test]
// pub(crate) fn test_all(){
// JUST TEST ALL IN FILE
//     test_reverse();
//     test_map();
// }

