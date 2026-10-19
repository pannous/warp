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
