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
