// Card import-compiled (notes/import_compiled.md): a module warp compiled is imported like any WebAssembly module, and
// its functions taking or giving values (texts, lists, any value: a Node of the module) are called too, each value
// copied between the program's instance and the module's
#[cfg(feature = "native")]
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
