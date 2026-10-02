//! Slices `a[start:end]`, `a[start..end]`, `a[start...last]`, `s.slice(start)` of lists and texts, and `a.copy()`
//! (Sorting field test, probes/slices/). A negative index never wraps around (Footguns.md): it is an error naming last(x).

mod common;
use common::fails_with;
use warp::is;

#[test]
fn python_slices_of_a_list() {
	is!("a=[1,2,3,4]; b=a[0:2]; count(b)*10 + b#2", 22);
	is!("a=[1,2,3,4]; b=a[1:]; b#1*10 + count(b)", 23);
	is!("a=[1,2,3,4]; b=a[:2]; b#2*10 + count(b)", 22);
	is!("a=[1,2,3]; b=a[:]; count(b)", 3);
	is!("a=[1,2,3,4]; i=1; j=3; b=a[i:j]; b#1*10 + count(b)", 22);
	is!("a=[1,2,3]; b=a[2:1]; count(b)", 0);
	is!("a=[1,2,3,4]; b=a[:1] + a[2:]; count(b)*10 + b#2", 33);
	is!("def tail(xs){ xs[1:] }; t = tail([7,8,9]); t#1", 8);
}

#[test]
fn range_slices_of_a_list() {
	is!("a=[1,2,3,4]; b=a[0..2]; count(b)*10 + b#2", 22);
	is!("a=[1,2,3,4]; b=a[0...1]; count(b)*10 + b#2", 22);
}

#[test]
fn slices_of_a_text_are_texts_of_characters() {
	is!("s=\"hello\"; s[1:3]", "el");
	is!("s=\"hello\"; s[2:]", "llo");
	is!("s=\"hello\"; s[:1] + s[3:]", "hlo");
	is!("s=\"αβγδ\"; s[1:3]", "βγ");
}

#[test]
fn the_slice_method_takes_an_optional_end() {
	is!("s=\"hello\"; s.slice(1)", "ello");
	is!("a=[1,2,3,4]; b=a.slice(1, 3); b#1*10 + count(b)", 22);
}

#[test]
fn copy_and_clone_are_the_value_itself() {
	is!("a=[1,2]; b=a.copy(); b#2", 2);
	is!("a=[1,2]; b=a.clone(); count(b)", 2);
	is!("a=[1,2]; b=a.copy(); b.push(3); count(a)*10 + count(b)", 23);
}

#[test]
fn a_negative_index_or_bound_is_an_error_that_names_last() {
	fails_with("x=[1,2,3]; x[-1]", "the last element is last(x)");
	fails_with("a=[1,2,3,4]; a[-2:]", "negative index -2");
	fails_with("s=\"hello\"; s[-3:-1]", "the last element is last(s)");
	fails_with("a=[1,2,3]; k=-2; a[k:]", "index out of range");
	fails_with("a=[1,2,3]; a.slice(0, -1)", "index out of range");
}

/// `/` never truncates (Footguns.md, Integer division): a fractional bound fails like a fractional index, `n//2` is hinted
#[test]
fn a_fractional_bound_must_be_an_integer_like_an_index() {
	fails_with("a=[1,2,3]; a[0:3/2]", "index must be an integer");
	fails_with("a=[1,2,3]; m=3/2; a[0:m]", "index must be an integer");
	fails_with("s=\"hello\"; m=5/2; s[m:]", "index must be an integer");
	is!("a=[1,2,3,4]; m=4/2; b=a[m:]; b#1", 3);
	is!("a=[1,2,3]; b=a[0:3//2]; count(b)", 1);
}
