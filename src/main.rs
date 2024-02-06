// pub mod string_tests {
// use crate::extensions::StringExtensions;

mod extensions;
use crate::extensions::*; // crate for F12
use extensions::numbers::*;
use extensions::strings::*;
use extensions::lists::*;
use extensions::utils::*;

// glob import doesn't reexport anything because no candidate is public enough
// reexporting is done by pub use
// pub mod extensions;
// pub use extensions::*;


// typedef Vec<String> StringVec in rust:
type Strings = Vec<String>;

#[allow(unused_variables)] // for testing
fn main() {
    let url="https://files.pannous.com/test";
    let test:String=download(url);
    let n=Number::Int(5);
    let f=Number::Float(5.0);
    let c=Number::Complex(5.0,7.0);
    let q=Number::Quotient(5,7);
    let f:f64 = (q/n).into();
    // put!((q/n) as f64);
    // put!("q.sign();
    put!("test ", test);

    let ranges=1..10;
    let range:Vec<i32> = ranges.collect();
    // let range1:Vec<i32> = ranges.into();
    put!("range ", range);

    // tests are in a separate module, usually not accessed from the main module
    // string_tests::test_all();

    let upper= "hello 🌍".map(|c| c.upper());
    put!("upper reverse ", upper.reverse());

    // list of 5 numbers
    let numbers = [1, 2, 3, 4, 5];
    let xxs : List<Number> = numbers.into();
    // map each number to a string
    let numbers_as_chars = numbers.map(|n| n.to_char());
    print_list(numbers_as_chars);
    let numbers_as_strings = numbers.map(|n| n.to_string());
    print_list(&numbers_as_strings);
    // print_list(numbers_as_strings);
    let xs:Strings = numbers_as_strings.to_vec();
    // let ys:[String;5] = xs.into();
    // let zs:[String;6] = xs.with(6);
    let zs:Vec<String> = xs.add("6");
    // print_list(ys);
    print_list(zs);
    // print_list(zs.clone());
    // let numbers_as_strings: [String; 5] = numbers.map(|n| n.to_string());
    // let numbers_as_strings: Vec<String> = numbers.iter().map(|n| n.to_string()).collect();
    tee();
}

