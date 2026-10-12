// card held-key: `held_key` names the key held now, as `on key` names each press ("a", "ArrowUp", "Enter"), "" while
// none is, so a drawing loop can steer: `if held_key == "ArrowLeft" { x -= 1 }`. Only "none held" is checked running
// (holding a key here would queue a press other tests' `on key` handlers hear); the naming runs on given codes
use crate::is;
use warp::node::Node;

fn named(code: i64) -> String {
	let program = warp::system_signals::key_named(Node::int(code)).serialize();
	warp::wasm_emitter::eval(&program).serialize()
}

#[test]
fn held_key_names_the_key_held_now() {
	let lowered = warp::pipeline::lower(r#"held_key == "a""#).expect("lowers").serialize();
	assert!(lowered.contains("system_value"), "{lowered}");
	is!(r#"held_key == """#, true);
	assert_eq!(named(0), r#""""#);
	assert_eq!(named('a' as i64), r#""a""#);
	assert_eq!(named(0xF702), r#""ArrowLeft""#);
	assert_eq!(named(13), r#""Enter""#);
}
