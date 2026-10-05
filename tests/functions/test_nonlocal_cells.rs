//! Card nonlocal-inner: `nonlocal y` lets a nested function change y (a cell outer and inner share), and a nested
//! function outlives its call: an escaping closure keeps its own values and its own cell
use warp::is;

#[test]
fn test_inner_changes_the_outer_variable() {
	is!("def outer(){ y=1; def inner(){ nonlocal y; y = y+1 }; inner(); y }; outer()", 2);
	is!("def outer(){ y=1; def inner(){ nonlocal y; y += 10 }; inner(); inner(); y }; outer()", 21);
	is!("def outer(n){ def inner(){ nonlocal n; n *= 2 }; inner(); n + 1 }; outer(5)", 11);
	is!("def outer(){ t=0.5; def inner(){ nonlocal t; t += 0.25 }; inner(); t }; outer()", 0.75);
}

#[test]
fn test_an_escaping_closure_keeps_its_values() {
	is!("def make(){ inc = () => 5; inc }; a = make(); a()", 5);
	is!("def make(k){ def add(x){ k + x }; add }; a = make(1); b = make(10); a(5) + b(5)", 21);
}

#[test]
fn test_an_escaping_closure_keeps_its_cell() {
	is!("def make(){ n=0; def inc(){ nonlocal n; n += 1; n }; inc }; c = make(); c(); c()", 2);
	is!("def make(){ n=0; def inc(){ nonlocal n; n += 1; n }; inc }; a = make(); b = make(); a(); a(); b()", 1);
}
