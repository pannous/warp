// card regression-module: a symbol handed on through untyped parameters stays a symbol in every function it reaches
use crate::is;

#[test]
fn a_symbol_passes_through_two_functions() {
	is!("look(k) := string(k); f(m) := look(m); f(:post)", "post");
	is!("look(k) := string(k); f(m) := look(m); f(post)", "post");
	is!("use math; look(k) := string(k); f(m) := look(m); f(math)", "math");
}
