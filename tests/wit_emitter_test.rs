use wasp::wit_emitter::{WitEmitter, node_to_wit_value};
use wasp::wasp_parser::WaspParser;
use wasp::node::{Node, Meta};

#[test]
fn test_wit_interface_generation() {
    let mut emitter = WitEmitter::new();
    emitter.emit_interface("test", "nodes");

    let output = emitter.get_output();

    // Check package declaration
    assert!(output.contains("package test:nodes"));

    // Check interface declaration
    assert!(output.contains("interface nodes"));

    // Check all type definitions
    assert!(output.contains("variant number"));
    assert!(output.contains("variant node"));
    assert!(output.contains("enum bracket"));
    assert!(output.contains("enum kind"));
    assert!(output.contains("enum data-type"));
    assert!(output.contains("record meta"));
    assert!(output.contains("record data"));

    // Check function signatures
    assert!(output.contains("parse: func(input: string) -> Node"));
    assert!(output.contains("to-json: func(node: node) -> result<string, string>"));
    assert!(output.contains("to-wasp: func(node: node) -> string"));

    // Check world export
    assert!(output.contains("world nodes"));
    assert!(output.contains("export nodes"));
}

#[test]
fn test_simple_node_to_wit() {
    // Empty
    let node = Node::Empty;
    assert_eq!(node_to_wit_value(&node), "empty");

    // Integer
    let node = Node::int(42);
    assert_eq!(node_to_wit_value(&node), "number(int(42))");

    // Float
    let node = Node::float(3.14);
    assert_eq!(node_to_wit_value(&node), "number(float(3.14))");

    // Text
    let node = Node::text("hello");
    assert_eq!(node_to_wit_value(&node), "text(\"hello\")");

    // Symbol
    let node = Node::symbol("foo");
    assert_eq!(node_to_wit_value(&node), "symbol(\"foo\")");
}

#[test]
fn test_key_value_to_wit() {
    let node = Node::keys("name", "Alice");
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("key-value"));
    assert!(wit.contains("\"name\""));
    assert!(wit.contains("text(\"Alice\")"));
}

#[test]
fn test_pair_to_wit() {
    let node = Node::pair(Node::int(1), Node::text("one"));
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("pair"));
    assert!(wit.contains("number(int(1))"));
    assert!(wit.contains("text(\"one\")"));
}

#[test]
fn test_tag_to_wit() {
    let node = Node::tag("div", Node::text("content"));
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("tag"));
    assert!(wit.contains("\"div\""));
    assert!(wit.contains("empty"));
    assert!(wit.contains("text(\"content\")"));
}

#[test]
fn test_list_to_wit() {
    let node = Node::list(vec![
        Node::int(1),
        Node::int(2),
        Node::int(3),
    ]);
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("list(["));
    assert!(wit.contains("number(int(1))"));
    assert!(wit.contains("number(int(2))"));
    assert!(wit.contains("number(int(3))"));
}

#[test]
fn test_meta_to_wit() {
    let meta = Meta::with_position(10, 5);
    let node = Node::int(42).with_meta(meta);
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("with-meta"));
    assert!(wit.contains("line: some(10)"));
    assert!(wit.contains("column: some(5)"));
    assert!(wit.contains("comment: none"));
}

#[test]
fn test_meta_with_comment_to_wit() {
    let mut meta = Meta::with_position(5, 1);
    meta.comment = Some("important value".to_string());

    let node = Node::int(100).with_meta(meta);
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("with-meta"));
    assert!(wit.contains("line: some(5)"));
    assert!(wit.contains("column: some(1)"));
    assert!(wit.contains("comment: some(\"important value\")"));
}

#[test]
fn test_string_escaping() {
    let node = Node::text("hello\nworld\t\"quoted\"");
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("\\n"));
    assert!(wit.contains("\\t"));
    assert!(wit.contains("\\\""));
}

#[test]
fn test_wasp_to_wit_roundtrip() {
    let wasp = r#"config {
        port: 8080
        host: "localhost"
    }"#;

    let node = WaspParser::parse(wasp);
    let wit = node_to_wit_value(&node);

    // Check structure is preserved
    assert!(wit.contains("tag"));
    assert!(wit.contains("\"config\""));
    assert!(wit.contains("\"port\""));
    assert!(wit.contains("8080"));
    assert!(wit.contains("\"host\""));
    assert!(wit.contains("\"localhost\""));

    // Check metadata is included
    assert!(wit.contains("with-meta"));
    assert!(wit.contains("line:"));
    assert!(wit.contains("column:"));
}

#[test]
fn test_nested_structure_to_wit() {
    let node = Node::tag(
        "html",
        Node::list(vec![
            Node::tag("head", Node::keys("title", "My Page")),
            Node::tag(
                "body",
                Node::list(vec![
                    Node::keys("h1", "Welcome"),
                    Node::keys("p", "Hello World"),
                ]),
            ),
        ]),
    );

    let wit = node_to_wit_value(&node);

    assert!(wit.contains("tag((\"html\""));
    assert!(wit.contains("tag((\"head\""));
    assert!(wit.contains("tag((\"body\""));
    assert!(wit.contains("\"title\""));
    assert!(wit.contains("\"My Page\""));
    assert!(wit.contains("\"Welcome\""));
    assert!(wit.contains("\"Hello World\""));
}

#[test]
fn test_mixed_types_list() {
    let node = Node::list(vec![
        Node::int(42),
        Node::float(3.14),
        Node::text("string"),
        Node::symbol("sym"),
        Node::Empty,
    ]);

    let wit = node_to_wit_value(&node);

    assert!(wit.contains("number(int(42))"));
    assert!(wit.contains("number(float(3.14))"));
    assert!(wit.contains("text(\"string\")"));
    assert!(wit.contains("symbol(\"sym\")"));
    assert!(wit.contains("empty"));
}

#[test]
fn test_data_node_to_wit() {
    let node = Node::data(vec![1, 2, 3]);
    let wit = node_to_wit_value(&node);

    assert!(wit.contains("data({"));
    assert!(wit.contains("type-name:"));
    assert!(wit.contains("data-type: vec"));
}

#[test]
fn test_wit_file_generation() {
    let mut emitter = WitEmitter::new();
    emitter.emit_interface("wasp", "test");

    // Should be able to write to file
    let result = emitter.emit_to_file("/tmp/test-wasp.wit");
    assert!(result.is_ok());

    // Verify file contents
    let contents = std::fs::read_to_string("/tmp/test-wasp.wit").unwrap();
    assert!(contents.contains("package wasp:test"));
    assert!(contents.contains("variant node"));
}
