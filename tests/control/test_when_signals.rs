// Card signals-shape (user decision 2026-10-08): `when` by shape. `when click {…}` is `on click {…}`, `when x > 3 {…}` is
// `whenever x > 3 {…}` (edge-triggered, P156), a block of `->` arms stays Kotlin's switch
use crate::is;

#[test]
fn when_an_event_listens_like_on() {
	is!("n = 0; when ping { n += 1 }; emit ping; emit ping; n", 2);
}

#[test]
fn when_a_condition_listens_like_whenever() {
	is!("x = 0; hits = 0; when x > 1 { hits += 1 }; x = 1; x = 2; x = 3; x = 0; x = 5; hits", 2);
}

#[test]
fn when_arms_stay_a_switch() {
	is!("when 3 { 1 -> \"a\"; 3 -> \"c\" }", 'c');
	is!("x = 2; when (x) { 1 -> 10; else -> 20 }", 20);
}
