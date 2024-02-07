use wasp::*;

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
fn test_substring() {
    init_lib();
    let s = "hello 🌍";
    let sub = s.substring(3, 5);
    put!("substring ", sub);
    assert_eq!(sub, "lo");
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
fn test_interpolation() {
    let _world = "🌍";
    let s = format!("hello {_world}");
    assert_eq!(s, "hello 🌍");
}

#[test]
fn test_reverse() {
    let s = "hello 🌍";
    let rev = s.reverse();
    put!("reverse ", &rev);
    assert_eq!(rev, "🌍 olleh");
}

#[test]
fn test_map() {
    // custom .to_uppercase()
    let upper = "hello 🌍".map(|c| c.upper());
    put!("upper ", &upper);
    assert_eq!(upper, "HELLO 🌍");
}
// }


// #[test]
// pub(crate) fn test_all(){
// JUST TEST ALL IN FILE
//     test_reverse();
//     test_map();
// }