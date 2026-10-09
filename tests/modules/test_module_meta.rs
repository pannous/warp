// Card reflection-foreign-meta (notes/reflection.md "warp.meta layout"): a compiled module names its functions and
// classes in its warp.meta section, so a program importing it reflects on them: `m.exports` are the module's own words
// (not warp's runtime exports), `m.f.params` and `m.f.signature` its functions' parameters and signatures
#[cfg(feature = "native")]
use crate::is;

#[cfg(feature = "native")]
fn imported(name: &str, source: &str) -> String {
	let directory = crate::common::scratch_directory("module_meta");
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join(format!("{name}.wasm"));
	std::fs::write(&path, warp::pipeline::compile(source).expect("a module").bytes).unwrap();
	format!("import \"{}\"\n", path.display())
}

#[cfg(feature = "native")]
#[test]
fn an_imported_warp_module_reflects_its_functions() {
	let adder = imported("adder", "export def add(left: i32, right: i32) -> i32 { left + right }\nexport def twice(value: i32) -> i32 { value * 2 }\n0");
	is!(&format!("{adder}add(2, 3)"), 5);
	is!(&format!("{adder}adder.exports"), warp::texts(vec!["add", "twice"]));
	is!(&format!("{adder}adder.add.params"), warp::texts(vec!["left", "right"]));
	is!(&format!("{adder}adder.twice.signature"), "(value:i32) -> int");
}

#[test]
fn the_meta_section_names_functions_and_classes() {
	let module = warp::pipeline::compile("class P{width:int; height:int; area() := width * height}\nf(a:int) := a + 1\nf(P(1, 2).area())").expect("a module");
	let functions = warp::meta_section::entry(&module.bytes, "functions").expect("a functions entry");
	assert_eq!(functions["f"]["signature"].serialize(), "\"(a:int) -> int\"");
	let classes = warp::meta_section::entry(&module.bytes, "classes").expect("a classes entry");
	assert_eq!(classes["P"]["fields"].serialize(), "[\"width\" \"height\"]");
	assert_eq!(classes["P"]["methods"].serialize(), "[\"area\"]");
}
