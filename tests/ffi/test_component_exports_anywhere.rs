//! Card reflect-page-meta (notes/reflection.md step 4): `lib.exports` and `dir(lib)` of a component used with
//! `use wasm "x.wasm" as lib`, read off its binary at compile time, so the page's compiler answers them too
use crate::is;
use warp::texts;

const COMPONENT: &str = "tests/fixtures/components/rust_demo.wasm";
const EXPORTS: [&str; 9] =
	["checked_sqrt", "counter", "fib", "letters", "next_char", "sides", "stats_of", "total", "words"];

#[test]
fn a_components_exports_in_every_host() {
	is!(&format!("use wasm \"{COMPONENT}\" as r; r.exports"), texts(EXPORTS.to_vec()));
	is!(&format!("use wasm \"{COMPONENT}\" as r; dir(r)"), texts(EXPORTS.to_vec()));
}
