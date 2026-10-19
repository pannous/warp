// card add-http: `://` after a word marks a URL written without quotes, whatever its scheme, as `http://` does
use crate::is;
use warp::parse;

#[test]
fn any_scheme_before_colon_slash_slash_starts_a_url() {
	is!("x = ssh://git@pannous.com/warp\nx", "ssh://git@pannous.com/warp");
	is!("x = s3://bucket/key.txt\nx", "s3://bucket/key.txt");
	is!("x = https://pannous.com/files/test\nx", "https://pannous.com/files/test");
	is!("x = ipfs://cid/file.txt\nx", "ipfs://cid/file.txt");
}

#[test]
fn a_colon_without_slashes_stays_a_pair() {
	assert_eq!(parse("a:1"), parse("a: 1"));
}
