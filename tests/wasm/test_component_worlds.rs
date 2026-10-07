// A component's world (card wasm-interop-rest, samples/wasm_interop.wasp): `component name { import …: {…} export …: … }`
// declares what a component imports and exports; its WIT is what `warp build --wit` writes. The declaration does
// nothing when the program runs.
use crate::is;
use warp::wasp_parser::parse;

const PROGRAM: &str = "interface calculator {\n    add: (i32, i32) -> i32\n    ticks: () -> i64\n}\ncomponent my_component {\n    import host: {\n        print: (string) -> ()\n        time: () -> i64\n    }\n    export api: calculator\n}\n";

#[test]
fn a_component_declaration_does_nothing_at_run_time() {
	is!(&format!("{PROGRAM}6 * 7"), 42);
}

#[test]
fn the_world_as_wit() {
	let wit = warp::component_worlds::world_wit(&parse(PROGRAM)).expect("a world");
	assert_eq!(wit, "package warp:my-component;\n\ninterface calculator {\n  add: func(p1: s32, p2: s32) -> s32;\n  ticks: func() -> s64;\n}\n\nworld my-component {\n  import host: interface {\n    print: func(p1: string);\n    time: func() -> s64;\n  }\n  export api: interface {\n    add: func(p1: s32, p2: s32) -> s32;\n    ticks: func() -> s64;\n  }\n}\n");
}

#[test]
fn a_program_without_component_has_no_world() {
	assert!(warp::component_worlds::world_wit(&parse("interface shape { area: (int) -> float }")).is_err());
}

#[test]
fn an_unknown_interface_is_named() {
	let failure = warp::component_worlds::world_wit(&parse("component c { export api: missing }")).unwrap_err();
	assert!(failure.contains("missing"), "{failure}");
}

/// step 2: `warp build --component` makes a component of the world's exports, which another program uses like any
#[cfg(feature = "native")]
#[test]
fn a_built_component_is_used_by_another_program() {
	let directory = crate::common::scratch_directory("built_component");
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join("calc.wasm");
	let source = "interface calculator { add: (i32, i32) -> i32; scale: (f64) -> f64 }\ncomponent calc { export api: calculator }\nexport def add(a: i32, b: i32) -> i32 { a + b }\nexport def scale(x: f64) -> f64 { x * 2.5 }\n0";
	std::fs::write(&path, warp::component_builder::build(source).expect("a component")).unwrap();
	let using = format!("use wasm \"{}\" as calc\n", path.display());
	is!(&format!("{using}calc.add(2, 3)"), 5);
	is!(&format!("{using}calc.scale(2.0)"), 5.0);
}
