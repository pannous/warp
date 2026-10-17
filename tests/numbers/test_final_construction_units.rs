//! card printing-object: a program ending in a construction, `Display(96 dpi, 24 inch)`, shows its unit fields in
//! units, the same as a variable holding that instance does
use warp::wasm_emitter::eval;

fn shown(program: &str) -> String {
	eval(program).serialize().trim().to_string()
}

fn shown_like_a_variable(class: &str, construction: &str) {
	let direct = shown(&format!("{class}\n{construction}"));
	assert_eq!(direct, shown(&format!("{class}\nx = {construction}\nx")));
	assert!(!direct.contains("warp::"), "{direct}");
}

#[test]
fn a_final_construction_shows_its_units() {
	shown_like_a_variable("class Display{density: dpi; diagonal: inch}", "Display(96 dpi, 24 inch)");
	shown_like_a_variable("class Run{distance: km; note: text}", "Run(5 km, \"x\")");
	assert!(shown("class Run{distance: km}\nRun(5 km)").contains("5km"));
}
