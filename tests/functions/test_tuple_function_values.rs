// A function returning a tuple passed as a function value (closures.rs adapters) still destructures
use warp::is;

#[test]
fn tuple_functions_as_values() {
	is!("pair(x) := (x, x+1); f = function pair; a, b = f(3); b", 4);
	is!("pair(x) := (x, x+1); apply(g, v) := g(v); a, b = apply(pair, 3); b", 4);
	is!("pair(x) := (x, x+1); fs=[pair]; a, b = fs#1(3); b", 4);
}
