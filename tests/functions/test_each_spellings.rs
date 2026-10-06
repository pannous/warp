// `each` as other languages spell it: JS and Kotlin forEach, Rust for_each, PHP foreach ('undefined function: forEach')
use crate::is;

#[test]
fn for_each_spellings_are_each() {
	is!("total = 0; [1,2,3].forEach { total += it }; total", 6);
	is!("total = 0; [1,2,3].forEach(x => total += x); total", 6);
	is!("total = 0; xs = [4,5]; xs.for_each { total += it }; total", 9);
	is!("total = 0; [1,2].foreach { total += it }; total", 3);
}
