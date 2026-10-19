use crate::is;
use warp::warp_parser::WarpParser;

#[test]
fn test_comments_in_html_structure() {
	let warp = r#"
        html{
            // Header section
            header{ title:"My Site" }
            // Main content
            body{ content:"Hello World" }
        }
    "#;

	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("WARP with comments:\n{}\n", warp);
	println!("JSON output:\n{}", json);

	assert!(json.contains("html"));
	assert!(json.contains("header"));
}

// Comments
#[test]
fn test_comments() {
	is!("1+1 // comment", 2);
	is!("1 /* inline */ + 1", 2);
	is!("/* block \n comment */ 1+1", 2);
}

#[test] // samples/hello/comments.warp: "Nested block comments are supported"
fn test_nested_block_comments() {
	is!("/* a /* b */ c */ 3", 3);
	is!("/# a /# b #/ #/ 4", 4);
	is!("1 /* x */ + 2", 3);
}

#[test] // card comment-apostrophe: the scans before parsing skip `#` comments as the parser does
fn test_apostrophe_in_hash_comment_hides_no_code() {
	is!("# Rust's\nclass Point { x: int }\n# it's\n/# don't #/\nstr(Point{x: 1})", "Point{x:1}");
	// `type` ending a line declares no type named by the next line's first word (analyzer.warp settle)
	is!("f(t) := type\nchanged = yes\nn = 0\nwhile changed {\n\tchanged = no\n\tn += 1\n}\nn", 1);
}
