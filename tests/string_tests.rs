// extern crate warp;
// use warp::extensions::*;
#[allow(dead_code)]
mod extensions;
use extensions::*;
// just soft-linked for now ;)


#[test]
fn test_substring() {
    let s = "hello 🌍";
    let sub = s.substring(3, 5);
    put!("substring ", sub);
    assert_eq!(sub, "lo");
}


#[test]
fn test_interpolation() {
    let world = "🌍";
    let s = format!("hello {world}");
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