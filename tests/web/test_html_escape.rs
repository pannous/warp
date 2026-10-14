// Card cleanup-html-escape: one HTML escaper, warp::web::markup::escaped, for markup's failed renderings and for site.rs
// and web_server.rs (they had two: markup.rs's inline one left `>` as it was)
use warp::web::markup::escaped;

const FILES_THAT_ESCAPE: [(&str, &str); 3] = [
	("src/web/markup.rs", include_str!("../../src/web/markup.rs")),
	("src/web/site.rs", include_str!("../../src/web/site.rs")),
	("src/web/web_server.rs", include_str!("../../src/web/web_server.rs")),
];

#[test]
fn test_escaped_replaces_the_three_markup_characters() {
	assert_eq!(escaped("a < b > c & d"), "a &lt; b &gt; c &amp; d");
	assert_eq!(escaped("&lt;"), "&amp;lt;");
	assert_eq!(escaped("plain"), "plain");
}

#[test]
fn test_one_html_escaper() {
	let spelled_out: Vec<&str> =
		FILES_THAT_ESCAPE.iter().filter(|(_, source)| source.contains("\"&lt;\"")).map(|(path, _)| *path).collect();
	assert_eq!(spelled_out, ["src/web/markup.rs"], "only markup::escaped spells out the entities");
	assert_eq!(FILES_THAT_ESCAPE[0].1.matches("\"&lt;\"").count(), 1, "markup.rs escapes in one place");
}
