// card parser-tag: a glued pair `for:email` takes the one word as its value, so a text after it is the next item:
// `label{ for:email "Email address:" }` is the attribute and the label's text (samples/html.warp); a spaced pair
// `x: upper "c"` still takes the call
use warp::warp_parser::parse;
use warp::{Node, Op};

/// The items of the block `{…}` or `name{…}` as written
fn block_items(code: &str) -> Vec<String> {
	let parsed = parse(code);
	let body = match parsed.drop_meta() {
		Node::Key(_, Op::Colon, body) => body.drop_meta().clone(),
		other => other.clone(),
	};
	let Node::List(items, _, _) = body else { panic!("no block: {}", parsed.serialize()) };
	items.iter().map(|item| item.serialize().trim().to_string()).collect()
}

#[test]
fn a_glued_pair_ends_at_its_word() {
	assert_eq!(block_items("label{ for:email \"Email address:\" }"), ["for:email", "\"Email address:\""]);
	assert_eq!(block_items("{a:b \"cd\"}"), ["a:b", "\"cd\""]);
	assert_eq!(block_items("{a: upper \"c\"}").len(), 1);
}
