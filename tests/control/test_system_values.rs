// System values (P136, notes/system_signals.md): `battery` (percent), `charging`, `online` and `dark mode` read the
// machine's state; `whenever battery < 20% {…}`, `whenever dark mode {…}`, `on change online {…}` listen to them,
// checked at the program's check points and once a second while a `warp run` stays
use crate::is;
#[cfg(feature = "native")]
use warp::Node;

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program that needs a module").serialize()
}

#[test]
fn a_system_value_is_a_host_reading() {
	assert!(lowered("print online").contains("system_value"));
	assert!(lowered("print dark mode").contains("dark mode"));
	assert!(lowered("print battery").contains("(system_value \"battery\")"));
}

#[test]
fn a_program_keeps_its_own_names() {
	is!("online = 3; online + 1", 4);
	is!("f(online) := online * 2; f(3)", 6);
	is!("p = {battery: 5}; p.battery", 5);
}

#[test]
fn a_listener_on_a_system_value_polls_it() {
	let code = lowered("whenever battery < 20% { print \"plug me in\" }");
	assert!(code.contains("(system_value \"battery\")") && code.contains("20") && code.contains("on·shared"), "{code}");
	assert!(code.contains("signal_every"), "a program listening to the machine stays: {code}");
	assert!(lowered("whenever dark mode { print \"dark\" }").contains("system_value"));
}

#[test]
#[cfg(all(feature = "native", any(target_os = "macos", target_os = "linux")))]
fn yes_no_values_read_the_machine() {
	is!("online == true or online == false", true);
	is!("n = 1; whenever dark mode { n = 2 }; n", 1);
}

#[test]
#[cfg(all(feature = "native", target_os = "macos"))]
fn dark_mode_is_the_appearance_of_macos() {
	let style = std::process::Command::new("defaults").args(["read", "-g", "AppleInterfaceStyle"]).output().expect("defaults runs");
	let dark = String::from_utf8_lossy(&style.stdout).trim() == "Dark";
	is!("dark mode", dark);
}

#[test]
#[cfg(all(feature = "native", any(target_os = "macos", target_os = "linux")))]
fn the_battery_is_a_percent_or_a_loud_error() {
	match warp::wasm_emitter::eval("battery") {
		Node::Number(number) => assert!((0.0..=100.0).contains(&f64::from(number)), "{number:?}"),
		Node::Error(message) => assert!(format!("{message}").contains("battery"), "{message}"),
		other => panic!("battery: {other:?}"),
	}
}

// `whenever not online {…}` and `not dark mode` negate the system value (cards whenever-online, whenever-dark): the
// word `not` after `whenever` was read as an infix operator, `whenever not online` fired at once, `not dark mode` split
// the phrase
#[test]
fn a_negated_system_value() {
	assert!(lowered("whenever not dark mode { print \"light\" }").contains("not (system_value dark mode)"));
	assert!(lowered("whenever not online { print \"offline\" }").contains("on·shared"));
	is!("x = 1; whenever not x { x = 7 }; x = 0; x", 7);
	is!("n = 1; whenever not online { n = 2 }; n", 1);
}

#[test]
#[cfg(all(feature = "native", any(target_os = "macos", target_os = "linux")))]
fn not_dark_mode_is_the_opposite_reading() {
	is!("x = not dark mode; y = dark mode; x != y", true);
}

// a method call's arguments were taken for a field name: seen.push([online, battery]) pushed the symbols
#[test]
fn method_arguments_read_system_values() {
	assert!(lowered("seen = []; seen.push([online, battery]); seen").contains("(system_value \"battery\")"));
	assert!(lowered("seen = []; seen.push(online); seen").contains("system_value"));
	is!("p = {battery: 5}; p.battery", 5);
}

// the pointer over the page's canvas (card drawing-frames): mouse_x / mouse_y in canvas pixels, mouse_down yes/no;
// natively the pointer over the paint window, 0 before one (card native-system)
#[test]
fn the_mouse_is_a_system_value_of_the_page() {
	assert!(lowered("print mouse_x").contains("(system_value \"mouse_x\")"));
	let pressed = lowered("if mouse_down { print 1 }");
	assert!(pressed.contains("(system_value mouse_down)!=0"), "{pressed}");
	is!("mouse_y = 3; mouse_y + 1", 4);
	// its own process: no window there, and no other test's pointer
	#[cfg(feature = "native")]
	assert_eq!(String::from_utf8_lossy(&crate::common::warp_command().args(["--no-ask", "eval", "mouse_x"]).output().unwrap().stdout).trim(), "» 0");
}

// card native-system: natively the pointer over the paint window (src/paint_window.rs follow_input) in the painted
// frame's pixels, as the page's canvas
#[test]
#[cfg(feature = "native")]
fn the_mouse_is_the_pointer_over_the_paint_window() {
	warp_runtime::system_values::tell_window_input([12.0, 7.0, 1.0, 0.0]);
	is!("mouse_x * 100 + mouse_y", 1207);
	is!("if mouse_down { 1 } else { 0 }", 1);
}

// window_open: whether a paint window shows the program's pictures, so `while window_open { … }` ends when the window
// closes and never starts headless (tests, CI, WARP_NO_WINDOW); the page's canvas is always open
#[test]
fn window_open_is_no_without_a_window() {
	assert!(lowered("while window_open { sleep 16ms }").contains("(system_value window_open)!=0"));
	#[cfg(feature = "native")]
	assert_eq!(String::from_utf8_lossy(&crate::common::warp_command().args(["--no-ask", "eval", "while window_open { sleep 16ms }; 7"]).output().unwrap().stdout).trim(), "» 7");
}
