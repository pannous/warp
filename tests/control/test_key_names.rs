// card key-if: inside `on key {…}` a bare `key` is the pressed key (event.key) and key words name keys, so
// `on key { if key == ESC { exit } }` reads as written (ESC, RETURN, UP, F1 … aliases of the KeyboardEvent names)
use crate::is;

#[test]
fn a_bare_key_in_a_key_handler_is_the_pressed_key() {
	is!("seen = \"\"\non key { seen = key }\nemit key{key:\"q\"}\nseen", "q");
}

#[test]
fn key_words_name_keys() {
	is!("n = 0\non key { if key == ESC { n += 1 } }\nemit key{key:\"Escape\"}\nemit key{key:\"q\"}\nn", 1);
	is!("n = 0\non key { if key == UP or key == ArrowDown { n += 1 } }\nemit key{key:\"ArrowUp\"}\nemit key{key:\"ArrowDown\"}\nn", 2);
	is!("n = 0\non key { if key == RETURN { n += 1 } }\nemit key{key:\"Enter\"}\nn", 1);
	is!("n = 0\non key { if key == F1 { n += 1 } }\nemit key{key:\"F1\"}\nn", 1);
}

#[test]
fn a_programs_own_key_variable_stays_its_own() {
	is!("key = \"mine\"\nseen = \"\"\non key { seen = key }\nemit key{key:\"q\"}\nseen", "mine");
	is!("seen = \"\"\non key { seen = event.key }\nemit key{key:\"q\"}\nseen", "q");
}

// a key handler comparing with a word that names nothing (`ESCAPEE`, a letter `q`) never matches: a warning says so
fn key_warnings(program: &str) -> Vec<String> {
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval(program);
	warp::errors::diagnostic::take_warnings()
		.into_iter()
		.map(|warning| warning.message)
		.filter(|message| message.contains("names nothing"))
		.collect()
}

#[test]
fn a_key_handler_comparing_with_an_unbound_word_warns() {
	let warned = key_warnings("n = 0\non key { if key == ESCAPEE { n += 1 } }\nn");
	assert!(warned.iter().any(|message| message.contains("ESCAPEE")), "{warned:?}");
	assert_eq!(
		key_warnings("n = 0\nmine = \"q\"\non key { local = \"w\"\n if key == ESC or key == mine or key == local { n += 1 } }\nn"),
		Vec::<String>::new()
	);
}
