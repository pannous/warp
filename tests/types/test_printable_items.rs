//! card list-printable-items: an instance of a class with text() shows as that text wherever it is shown, as an item of
//! a list (printed, interpolated, the program's value) and when its type is known only at run time (`print x` in f)
use crate::is;

const NUMBERED: &str = "class N { k: int; text() := \"n\" + string(k) }\n";

fn shown(program: &str) -> String {
	warp::wasm_emitter::eval(&format!("{NUMBERED}{program}")).serialize()
}

#[test]
fn list_items_show_their_text() {
	assert_eq!(shown("[N(1), N(2)]"), "[n1 n2]");
	assert_eq!(shown("xs = [N(1), N(2)]\nxs"), "[n1 n2]");
	assert_eq!(shown("[[N(1)], 2, \"s\"]"), "[[n1] 2 's']");
	assert_eq!(shown("class P{x:int}\n[P(1), N(2)]"), "[P{x:1} n2]");
	is!(&format!("{NUMBERED}xs = [N(1)]\n\"\\(xs)\""), "[n1]");
	is!(&format!("{NUMBERED}str([N(1), N(2)])"), "[n1 n2]");
	is!(&format!("{NUMBERED}count([N(1), N(2)])"), 2);
}

#[test]
fn an_instance_of_run_time_type_shows_its_text() {
	is!(&format!("{NUMBERED}f(x) := str(x)\nf(N(1))"), "n1");
	assert_eq!(shown("N(3)"), "n3");
	is!(&format!("{NUMBERED}x = N(1)\nstr(x) + \"!\""), "n1!");
}

#[test]
fn methods_of_the_text_type() {
	is!("fs = methods(text)\nfs#1.name", "repeat");
	is!("(\"Cursor\" in methods(text)) > 0", false);
}

/// card traps-task: a task runs in a fresh instance of the module, which never runs main: the start function installs
/// the witness tables there too (a null witness trapped `go` blocks and live loops)
#[test]
fn an_instance_shows_its_text_in_a_task() {
	is!(&format!("{NUMBERED}t = go {{ str([N(3)]) }}\nawait t"), "[n3]");
}
