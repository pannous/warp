// Card import-compiled (notes/import_compiled.md): a module warp compiled is imported like any WebAssembly module, and
// its functions taking or giving values (texts, lists, any value: a Node of the module) are called too, each value
// copied between the program's instance and the module's
use crate::is;

#[cfg(feature = "native")]
const SHAPES: &str = "class P{x:int; y:int; sum() := x + y}\ndef area(w:int, h:int) { w * h }\ndef greet(name:text) { \"hi \" + name }\ndef total(numbers) { sum(numbers) }\ndef width(p:P) { p.x }\ndef origin() { P(0, 7) }\n0";

#[cfg(feature = "native")]
fn imported(name: &str, source: &str) -> String {
	let directory = crate::common::scratch_directory("import_compiled");
	std::fs::create_dir_all(&directory).unwrap();
	let path = directory.join(format!("{name}.wasm"));
	std::fs::write(&path, warp::pipeline::compile(source).expect("a module").bytes).unwrap();
	format!("import \"{}\"\n", path.display())
}

#[cfg(feature = "native")]
#[test]
fn a_compiled_modules_functions_take_and_give_values() {
	let shapes = imported("shapes", SHAPES);
	is!(&format!("{shapes}area(2, 3)"), 6);
	is!(&format!("{shapes}greet(\"Bo\")"), "hi Bo");
	is!(&format!("{shapes}shapes.greet(\"Ann\")"), "hi Ann");
	is!(&format!("{shapes}total([1, 2, 3])"), 6);
}

#[cfg(feature = "native")]
#[test]
fn a_compiled_modules_classes_are_the_programs_too() {
	let shapes = imported("shapes", SHAPES);
	is!(&format!("{shapes}P(1, 2).y"), 2);
	is!(&format!("{shapes}shapes.P(1, 2).x"), 1);
	is!(&format!("{shapes}P(3, 4).sum()"), 7);
	is!(&format!("{shapes}width(P(5, 6))"), 5);
	is!(&format!("{shapes}origin().y"), 7);
}

// card import-compiled-browser: the same through a module compiled once (`warp compile --wasm
// tests/fixtures/compiled/shapes.warp`), which the browser reads too
const COMPILED_SHAPES: &str = "import tests/fixtures/compiled/shapes\n";

#[test]
fn a_compiled_fixtures_values_and_classes_cross() {
	is!(&format!("{COMPILED_SHAPES}area(2, 3)"), 6);
	is!(&format!("{COMPILED_SHAPES}greet(\"Bo\")"), "hi Bo");
	is!(&format!("{COMPILED_SHAPES}P(3, 4).sum()"), 7);
	is!(&format!("{COMPILED_SHAPES}width(P(5, 6))"), 5);
	is!(&format!("{COMPILED_SHAPES}origin().y"), 7);
}

// a module compiled for any host exports the reflection getters the browser's host reads its values with
#[cfg(feature = "native")]
#[test]
fn a_module_for_any_host_can_be_read_by_the_browser() {
	let module = warp::pipeline::for_any_host(|| warp::pipeline::compile(SHAPES)).expect("a module");
	assert!(module.bytes.windows("reflect_data".len()).any(|window| window == b"reflect_data"));
}

// card import-shared: an instance the program passes is the module's too, not a copy: a change the module makes is
// the program's
#[cfg(feature = "native")]
#[test]
fn an_instance_passed_to_a_compiled_module_is_shared() {
	let moving = imported("moving", "class P{x:int; y:int}\ndef moved(p:P) { p.x = 9; 0 }\ndef grown(numbers) { numbers.add(4); 0 }\n0");
	is!(&format!("{moving}p = P(1, 2)\nmoved(p)\np.x"), 9);
	is!(&format!("{moving}q = P(1, 2)\nr = q\nmoved(q)\nr.x"), 9);
	is!(&format!("{moving}numbers = [1, 2, 3]\ngrown(numbers)\ncount(numbers)"), 4);
}

// the same in the browser, through the fixture's moved
#[test]
fn a_compiled_fixtures_change_of_an_instance_is_the_programs() {
	is!(&format!("{COMPILED_SHAPES}p = P(1, 2)\nmoved(p)\np.x"), 9);
}
