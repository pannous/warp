// card if-key (user): the key words compare in lowercase too, `if key == space`, `held_key == left`, `key == f1`;
// a lowercase word stays a key name only beside key or held_key, so `return` and a program's own `left` keep meaning
use crate::is;

#[test]
fn lowercase_key_words_name_keys() {
	is!("n = 0\non key { if key == space { n += 1 } }\nemit key{key:\" \"}\nemit key{key:\"q\"}\nn", 1);
	is!("n = 0\non key { if key == esc or up == key { n += 1 } }\nemit key{key:\"Escape\"}\nemit key{key:\"ArrowUp\"}\nn", 2);
	is!("n = 0\non key { if key != enter { n += 1 } }\nemit key{key:\"Enter\"}\nemit key{key:\"f1\"}\nn", 1);
	is!("n = 0\non key { if key == f1 { n += 1 } }\nemit key{key:\"F1\"}\nn", 1);
}

#[test]
fn a_lowercase_key_word_elsewhere_keeps_its_meaning() {
	is!(
		"left = \"mine\"\nseen = \"\"\non key { if key == left { seen = left } }\nemit key{key:\"mine\"}\nseen",
		"mine"
	);
	is!("seen = \"\"\non key { up = 3; seen = up }\nemit key{key:\"q\"}\nseen", 3);
}

// reviewer veto on if-key: a lowercase word names a key only compared with key itself, and never a name bound anywhere
#[test]
fn a_bound_or_unrelated_lowercase_word_is_no_key() {
	is!(
		"seen = \"\"\non key { left = 3\n if 3 == left { seen = \"same\" } else { seen = \"differs\" } }\nemit key{key:\"q\"}\nseen",
		"same"
	);
	is!(
		"seen = \"\"\non key { for left in [\"q\"] { if key == left { seen = \"loop\" } } }\nemit key{key:\"q\"}\nseen",
		"loop"
	);
	is!(
		"same(up) := up == \"q\"\nseen = \"\"\non key { if same(key) { seen = \"parameter\" } }\nemit key{key:\"q\"}\nseen",
		"parameter"
	);
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval("mode = \"x\"\non key { if mode == up { mode = \"y\" } }\nmode");
	let warned = warp::errors::diagnostic::take_warnings()
		.into_iter()
		.any(|warning| warning.message.contains("up names nothing"));
	assert!(warned, "mode == up compares no key: up names nothing");
}

#[test]
fn held_key_compares_with_lowercase_key_words() {
	let lowered = warp::pipeline::lower("held_key == left or space == held_key").expect("lowers").serialize();
	assert!(lowered.contains(r#""ArrowLeft""#) && lowered.contains(r#"" ""#), "{lowered}");
}

// reviewer: a lowercase key word the program binds elsewhere names no key, and the warning says why and what to write
#[test]
fn a_key_word_bound_elsewhere_says_to_write_it_in_capitals() {
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval("f(left) := left\nn = 0\non key { if key == left { n += 1 } }\nn");
	let warned = warp::errors::diagnostic::take_warnings().into_iter().any(|warning| {
		warning.message.contains("left is also a name the program binds") && warning.message.contains("write LEFT")
	});
	assert!(warned);
}
