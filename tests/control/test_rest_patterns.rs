//! Rest patterns in `switch`/`match` (card rest-patterns): `[first, ...rest]` (also `*rest`, `rest...`) matches a list of
//! at least the other items, binding the rest as a list; a bare `..` or `...` takes the rest unnamed. A class pattern's
//! `..` (Rust's `Point{x, ..}`, `Point(x, ..)`) leaves the other fields out.
use crate::is;

#[test]
fn a_rest_binds_the_remaining_items() {
	is!("xs=[1,2,3]; match xs { [first, ...rest] => first * 10 + count(rest); _ => 0 }", 12);
	is!("xs=[1,2,3,4]; match xs { [a, *rest] => count(rest); _ => 0 }", 3);
	is!("xs=[1]; match xs { [a, ...rest] => count(rest); _ => 9 }", 0);
	is!("xs=[]; match xs { [a, ...rest] => 1; _ => 9 }", 9);
}

#[test]
fn items_after_a_rest_count_from_the_end() {
	is!("xs=[1,2,3,4]; match xs { [first, ...middle, last] => first + last * 10 + count(middle) * 100; _ => 0 }", 241);
	is!("xs=[5,6]; match xs { [.., last] => last; _ => 0 }", 6);
	is!("xs=[5,6,7]; match xs { [a, ...] => a; _ => 0 }", 5);
	is!("xs=[1,2,3]; match xs { [1, ..] => 1; _ => 0 }", 1);
}

#[test]
fn a_class_pattern_leaves_other_fields_out() {
	let point = "class P{x:int; y:int; z:int}; p = P(1, 2, 3); ";
	is!(&format!("{point}match p {{ P{{x, ..}} => x; _ => 0 }}"), 1);
	is!(&format!("{point}match p {{ P(x, ..) => x; _ => 0 }}"), 1);
	is!(&format!("{point}match p {{ P(x, y, ...) => x + y; _ => 0 }}"), 3);
}
