// card serialize-index: `g[i]` is `g#(i+1)` in the tree, without a group; its text keeps the parentheses, so it re-parses
// to the same tree (`g#i+1` would read as `(g#i)+1`)
use warp::parse;

fn serialized(code: &str) -> String {
	parse(code).serialize().trim().to_string()
}

#[test]
fn a_bracket_index_serializes_with_its_parentheses() {
	assert_eq!(serialized("g[i]"), "g#(i+1)");
	assert_eq!(serialized("g[i][j]"), "g#(i+1)#(j+1)");
	assert_eq!(serialized("g[2]"), "g#3");
	assert_eq!(serialized("#x"), "#x");
}

#[test]
fn a_bracket_index_roundtrips() {
	for code in ["g[i]", "g[i][j]", "g[i+1]", "g#(i+1)"] {
		let once = serialized(code);
		assert_eq!(serialized(&once), once, "{code} → {once}");
	}
}
