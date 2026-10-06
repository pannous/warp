// System values (P136, notes/system_signals.md): `battery` (percent), `charging`, `online` and `dark mode` read the
// machine's state; `whenever battery < 20% {…}`, `whenever dark mode {…}`, `on change online {…}` listen to them,
// checked at the program's check points and once a second while a `warp run` stays
use crate::is;
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
		Node::Number(number) => assert!((0.0..=100.0).contains(&f64::from(number.clone())), "{number:?}"),
		Node::Error(message) => assert!(format!("{message}").contains("battery"), "{message}"),
		other => panic!("battery: {other:?}"),
	}
}
