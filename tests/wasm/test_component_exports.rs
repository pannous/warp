// Card reflection-components (notes/reflection.md step 4): `lib.exports` and `dir(lib)` of a component used with
// `use wasm "x.wasm" as lib` are its exported function names, read off the component at compile time
use crate::is;

#[cfg(feature = "native")]
fn calculator() -> String {
	let directory = crate::common::scratch_directory("component_exports");
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join("calc.wasm");
	let source = "interface calculator { add: (i32, i32) -> i32; scale: (f64) -> f64 }\ncomponent calc { export api: calculator }\nexport def add(a: i32, b: i32) -> i32 { a + b }\nexport def scale(x: f64) -> f64 { x * 2.5 }\n0";
	std::fs::write(&path, warp::component_builder::build(source).expect("a component")).unwrap();
	format!("use wasm \"{}\" as calc\n", path.display())
}

#[cfg(feature = "native")]
#[test]
fn a_components_exports_and_dir() {
	let using = calculator();
	is!(&format!("{using}calc.exports"), warp::texts(vec!["add", "scale"]));
	is!(&format!("{using}dir(calc)"), warp::texts(vec!["add", "scale"]));
	is!(&format!("{using}calc.add(2, 3)"), 5);
}
