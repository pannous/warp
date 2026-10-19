//! card holder-says: a named argument holding a lambda, `Holder(d: x => x + 1)`, gives d the lambda as `{d: x => x + 1}`
//! does; before, `:` binding tighter than `=>` made it the lambda `(d: x) => x + 1` and x undefined. A typed lambda
//! `(x: int => x * 2)` stays one
use crate::is;

#[test]
fn a_named_argument_holds_the_lambda() {
	is!("class Holder { d }; h = Holder(d: x => x + 1); h.d(4)", 5);
	is!("apply(f, v) := f(v)\napply(x: int => x * 2, 5)", 10);
}

/// reviewer: a `key: value` item is an entry in any bracket, `[d: x => x + 1]` holds the lambda too
#[test]
fn a_listed_entry_holds_the_lambda() {
	is!("m = [d: x => x + 1]; m.d(2)", 3);
}
