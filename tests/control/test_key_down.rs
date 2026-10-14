// card keyboard-state: `key_down`, like mouse_down, is whether a key is held now (natively over the paint window, in
// the playground over the page), so a drawing loop can react without a handler: `if key_down { clear(white) }`.
// Only "no" is checked running: holding a key here would queue a press other tests' `on key` handlers hear
use crate::is;

#[test]
fn key_down_is_a_system_value_and_no_without_a_key_held() {
	let lowered = warp::pipeline::lower("if key_down { 1 } else { 0 }").expect("lowers").serialize();
	assert!(lowered.contains("system_value key_down"), "{lowered}");
	is!("if key_down { 1 } else { 0 }", 0);
}
