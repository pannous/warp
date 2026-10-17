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
	assert!(lowered.contains("key_held"), "{lowered}");
	is!(r#"held_key == """#, true);
	assert_eq!(named(0), r#""""#);
	assert_eq!(named('a' as i64), r#""a""#);
	assert_eq!(named(0xF702), r#""ArrowLeft""#);
	assert_eq!(named(13), r#""Enter""#);
}

// the key words compare with `held_key` as with `key` in a key handler (card key-if): `held_key == LEFT` is
// `held_key == "ArrowLeft"`, and a word that names nothing warns, as there
#[test]
fn held_key_compares_with_the_key_words() {
	// whatever key another test's `emit key` holds now, each word reads as its name
	is!(r#"held_key != ESC or held_key == "Escape""#, true);
	let lowered = warp::pipeline::lower("held_key == LEFT or ESC == held_key").expect("lowers").serialize();
	assert!(lowered.contains(r#""ArrowLeft""#) && lowered.contains(r#""Escape""#), "{lowered}");
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval("held_key == LEFTT");
	let warned = warp::errors::diagnostic::take_warnings()
		.into_iter()
		.filter(|warning| warning.message.contains("LEFTT names nothing"))
		.count();
	assert_eq!(warned, 1);
}

// card allow-diagonals: two keys held at once are both held, so a drawing loop steers diagonally with
// `if held_key == LEFT {…}` and `if held_key == UP {…}`: each comparison asks key_held whether its key is among those
// held. Checked on given keys: holding them here would make other tests' "none held" fail
#[cfg(feature = "native")]
#[test]
fn held_key_compares_with_every_key_held() {
	let lowered = warp::pipeline::lower("held_key == LEFT and UP == held_key").expect("lowers").serialize();
	assert_eq!(lowered.matches("key_held").count(), 2, "{lowered}");
	let (left, up, right) = (0xF702, 0xF700, 0xF703);
	use warp_runtime::system_values::held_among;
	assert!(
		held_among(left, up, &[left, up]) && held_among(up, up, &[left, up]) && !held_among(right, up, &[left, up])
	);
	assert!(!held_among(0, up, &[left, up]) && held_among(0, 0, &[]));
	is!(r#"arrow = "ArrowUp"; held_key != arrow"#, true);
}
