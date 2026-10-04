// Iterable (wiki/trait.md): a type defining `iterate(b:bag)` is walked by `for x in b` and searched by `x in b`, through
// the list its iterate gives
use warp::is;

const BAG: &str = "class bag{a:int b:int}; iterate(g:bag) := [g.a, g.b]; ";

#[test]
fn for_walks_what_iterate_gives() {
	is!(&format!("{BAG}s = 0; for x in bag(3, 4) {{ s += x }}; s"), 7);
	is!(&format!("{BAG}g = bag(5, 6); s = 0; for x in g {{ s += x }}; s"), 11);
}

#[test]
fn in_searches_what_iterate_gives() {
	is!(&format!("{BAG}g = bag(5, 6); 6 in g"), 2);
	is!(&format!("{BAG}g = bag(5, 6); 7 in g"), 0);
}
