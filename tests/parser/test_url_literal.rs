// card add-http: `://` after a word marks a URL written without quotes, whatever its scheme, as `http://` does
use crate::is;
use warp::{parse, Node};

#[test]
fn any_scheme_before_colon_slash_slash_starts_a_url() {
	assert_eq!(parse("ssh://git@pannous.com/warp"), Node::Text("ssh://git@pannous.com/warp".into()));
	assert_eq!(parse("s3://bucket/key.txt"), Node::Text("s3://bucket/key.txt".into()));
	assert_eq!(parse("https://pannous.com/files/test"), Node::Text("https://pannous.com/files/test".into()));
	is!("x = ipfs://cid/file.txt\nx", "ipfs://cid/file.txt");
}

#[test]
fn a_colon_without_slashes_stays_a_pair() {
	assert_eq!(parse("a:1"), parse("a: 1"));
}
