// A component's world (card wasm-interop-rest, samples/wasm_interop.warp): `component name { import …: {…} export …: … }`
// declares what a component imports and exports; its WIT is what `warp build --wit` writes. The declaration does
// nothing when the program runs.
use crate::is;
use warp::warp_parser::parse;

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

/// The `use` line of the component built from `source` (a file in the test's scratch directory, named `name.wasm`)
#[cfg(feature = "native")]
fn built_component(name: &str, source: &str) -> String {
	let directory = crate::common::scratch_directory("built_components");
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join(format!("{name}.wasm"));
	std::fs::write(&path, warp::component_builder::build(source).expect("a component")).unwrap();
	format!("use wasm \"{}\" as {name}\n", path.display())
}

/// step 3: texts cross the boundary as the canonical ABI lays them out, in the component's linear memory
#[cfg(feature = "native")]
#[test]
fn texts_cross_the_component_boundary() {
	let using = built_component("greeting", "interface greeter { greet: (string) -> string; size: (string) -> i32 }\ncomponent greeting { export api: greeter }\nexport def greet(name: string) -> string { \"hi \" + name }\nexport def size(t: string) -> i32 { count(t) }\n0");
	is!(&format!("{using}greeting.greet(\"Al\")"), "hi Al");
	is!(&format!("{using}greeting.size(\"abc\")"), 3);
}

/// card component-imports: the functions a world imports are the program's calls `host.time()`, core imports of the
/// module the import names; warp's host serves a component's `host` import its words print, time and read
#[cfg(feature = "native")]
#[test]
fn a_component_calls_what_its_world_imports() {
	let directory = crate::common::scratch_directory("component_imports");
	std::fs::create_dir_all(&directory).unwrap();
	let file = directory.join("greeting.txt");
	std::fs::write(&file, "hello").unwrap();
	let using = built_component("clocked", "interface clock { later: (i64) -> i64; shout: (string) -> string; say: (string) -> i32 }\ncomponent clocked {\n  import host: { print: (string) -> (); time: () -> i64; read: (string) -> string }\n  export api: clock\n}\nexport def later(x: i64) -> i64 { host.time() + x }\nexport def shout(path: string) -> string { host.read(path) + \"!\" }\nexport def say(t: string) -> i32 { host.print(t); count(t) }\n0");
	is!(&format!("{using}clocked.later(0) > 1700000000000"), true);
	is!(&format!("{using}clocked.say(\"abc\")"), 3);
	is!(&format!("{using}clocked.shout(\"{}\")", file.display()), "hello!");
}
