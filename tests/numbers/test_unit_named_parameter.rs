// card static-units-parameter: a parameter named like a unit the program also writes as a unit (`f(s) := s * 2; f(3 s)`)
// is a loud name clash, not the misleading "undefined variable: s (a unit…)"
use warp::wasm_emitter::eval;
use warp::Node;

fn clash(code: &str) -> String {
	match eval(code) {
		Node::Error(reason) => reason.serialize(),
		other => panic!("{code} should be an error, got {other:?}"),
	}
}

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_parameter_named_like_a_unit_written_as_that_unit_is_a_name_clash() {
	for code in ["f(s) := s * 2; f(3 s)", "f(m) := m + 1; f(2 m)", "f(s) := s * 2; 3 s + f(1)"] {
		let message = clash(code);
		assert!(message.contains("is a parameter of f and a unit"), "{code}: {message}");
		assert!(message.contains("rename the parameter"), "{code}: {message}");
		assert!(!message.contains("undefined variable"), "{code}: {message}");
	}
}

#[test]
fn a_parameter_named_like_another_unit_or_used_only_inside_is_no_clash() {
	assert_eq!(shown("f(x) := x * 2; f(3 s)"), "6s");
	assert_eq!(shown("f(s) := s * 2; f(3 km)"), "6km");
	assert_eq!(shown("f(s) := s * 2; f(3)"), "6");
}
