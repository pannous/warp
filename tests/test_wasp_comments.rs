// use warp::wasp_parser::WaspParser::parse;
use warp::Node;
use warp::wasp_parser::{parse, WaspParser};
use warp::{eq, is, put};

#[test]
fn test_line_comments() {
	let wasp = r#"
        // This is a comment
        name: "Alice"
        age: 30
    "#;

	let node = WaspParser::parse(wasp);
	println!("Parsed with line comment: {:?}", node);

	// Comments are attached as metadata to the following node
	// So we get 2 items: name (with comment metadata) + age
	eq!(node.len(), 2);
	// Verify comment is attached to the first item
	assert!(node[0]["comment"].to_string().contains("This is a comment"));
}

#[test]
fn test_block_comments() {
	let wasp = r#"
        /* This is a
           multi-line comment */
        name: "Bob"
        age: 25
    "#;

	let node = WaspParser::parse(wasp);
	println!("Parsed with block comment: {:?}", node);

	// Comments are attached as metadata to the following node
	// So we get 2 items: name (with comment metadata) + age
	if let Node::List(items, _, _) = node {
		eq!(items.len(), 2);
		// Verify comment is attached to the first item
		assert!(items[0]["comment"].to_string().contains("multi-line comment"));
	}
}

#[test]
fn test_inline_comments() {
	let wasp = r#"
        name: "Charlie" // name field
        age: 35 /* age in years */
    "#;

	let node = WaspParser::parse(wasp);
	println!("Parsed with inline comments: {:?}", node);
}

#[test]
fn test_comment_metadata() {
	let wasp = "// Important config\nport: 8080";
	let node = WaspParser::parse(wasp);
	put!(node.serialize());
	put!(node.serialize_recurse(true));
	println!("Node: {}", node);
	println!("Node: {:?}", node);
	println!("Comment metadata: {:?}", node["comment"]);
	assert!(node["comment"].to_string().contains("Important config"));
}

#[test]
fn test_comments_in_html_structure() {
	let wasp = r#"
        html{
            // Header section
            header{ title:"My Site" }
            // Main content
            body{ content:"Hello World" }
        }
    "#;

	let node = WaspParser::parse(wasp);
	let json = node.to_json().unwrap();

	println!("WASP with comments:\n{}\n", wasp);
	println!("JSON output:\n{}", json);

	assert!(json.contains("html"));
	assert!(json.contains("header"));
}

#[test]
fn test_comment_with_metadata_accessor() {
	let node = Node::int(42).with_comment("This is the answer".to_string());
	println!("{}", node);
	println!("{}", node.serialize());
	println!("{}", node["comment"]);
	eq!(node.drop_meta(), &Node::int(42));
	// Use &node["comment"] to explicitly use Not for &Node
	if !&node["comment"] {
		panic!("Expected metadata");
	} else {
		eq!(node["comment"], "This is the answer");
	}
}

// Comments
#[test]
#[ignore]
fn test_comments() {
	is!("1+1 // comment", 2);
	is!("1 /* inline */ + 1", 2);
	is!("/* block \n comment */ 1+1", 2);
}

#[test]
#[ignore]
fn test_comments2() {
	let c = "blah a b c # to silence python warnings;)\n y/* yeah! */=0 // really";
	let result: Node = parse(c);
	assert!(result.length() == 2);
	assert!(result[0].length() == 4);
	assert!(result[1].length() == 3);
}
