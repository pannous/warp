// An emoji in code (outside a text) is an atom like other non-ASCII characters (card emoji-code): one code point is a
// codepoint, a sequence of several (flags, skin tones, ZWJ sequences) is a symbol, read as itself
use crate::is;

#[test]
fn an_emoji_is_a_codepoint() {
	is!("\\:world == 🌍", true);
	is!("🌍 == '🌍'", true);
	is!("x = 🌍; x == '🌍'", true);
	is!("★ == '★'", true);
	is!("[🍎, 🍌]#2 == '🍌'", true);
}

#[test]
fn an_emoji_sequence_is_one_atom() {
	is!("x = 🇩🇪; x", warp::Node::Symbol("🇩🇪".to_string()));
	is!("👍🏽", warp::Node::Symbol("👍🏽".to_string()));
	is!("🇩🇪 = 3; 🇩🇪 + 1", 4);
	let warp::Node::List(emojis, _, _) = warp::wasp_parser::parse("[👨‍👩‍👧, 🇩🇪, 🌍]").drop_meta().clone() else { panic!("expected a list") };
	assert_eq!(emojis.len(), 3);
	assert_eq!(emojis[0].drop_meta(), &warp::Node::Symbol("👨‍👩‍👧".to_string()));
}
