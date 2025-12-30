use wasp::wasp_parser::WaspParser;
use wasp::node::Node;

#[test]
fn test_line_comments() {
    let wasp = r#"
        // This is a comment
        name: "Alice"
        age: 30
    "#;

    let node = WaspParser::parse(wasp);
    println!("Parsed with line comment: {:?}", node);

    // Should parse successfully
    if let Node::Block(items, _, _) = node {
        assert_eq!(items.len(), 2);
    }
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

    if let Node::Block(items, _, _) = node {
        assert_eq!(items.len(), 2);
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
    println!("Node: {:?}", node);

    if let Node::Block(items, _, _) = node {
        if let Some(first) = items.get(0) {
            if let Some(meta) = first.get_meta() {
                println!("Comment metadata: {:?}", meta.comment);
                assert!(meta.comment.is_some());
            }
        }
    }
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

    assert_eq!(node.unwrap_meta(), &Node::int(42));

    if let Some(meta) = node.get_meta() {
        assert_eq!(meta.comment, Some("This is the answer".to_string()));
    } else {
        panic!("Expected metadata");
    }
}
