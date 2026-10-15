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

#[test]
fn held_key_compares_with_lowercase_key_words() {
	let lowered = warp::pipeline::lower("held_key == left or space == held_key").expect("lowers").serialize();
	assert!(lowered.contains(r#""ArrowLeft""#) && lowered.contains(r#"" ""#), "{lowered}");
}
