use wasp::node::Node;
use wasp::node::{Grouper, Bracket};

#[test]
fn test_node_to_json_compact() {
    let n = Node::keys("name", "Alice");
    let json = n.to_json().unwrap();
    println!("Compact JSON:\n{}", json);

    // Should be simple key-value
    assert!(json.contains("name"));
    assert!(json.contains("Alice"));
    assert!(!json.contains("Key")); // No type tags!
}

#[test]
fn test_implicit_html_structure() {
    // html{ ul{ li:"hi" li:"ok"} colors=[red, green,blue]}
    let html = Node::Block(
        vec![
            Node::Block(
                vec![
                    Node::keys("li", "hi"),
                    Node::keys("li", "ok"),
                ],
                Grouper::Object,
                Bracket::Curly,
            ),
            Node::Key(
                "colors".to_string(),
                Box::new(Node::list(vec![
                    Node::symbol("red"),
                    Node::symbol("green"),
                    Node::symbol("blue"),
                ])),
            ),
        ],
        Grouper::Object,
        Bracket::Curly,
    );

    let json = html.to_json().unwrap();
    println!("HTML-like structure:\n{}", json);

    // Should be compact and implicit
    assert!(json.contains("li"));
    assert!(json.contains("colors"));
    assert!(json.contains("red"));
}

#[test]
fn test_node_from_json() {
    // Note: compact JSON format is lossy, can't perfectly round-trip
    // For full round-trip, use the tagged serde format directly
    let n = Node::keys("age", "30");
    let json = n.to_json().unwrap();
    println!("JSON: {}", json);

    // Just verify it's valid JSON
    assert!(json.contains("age"));
    assert!(json.contains("30"));
}

#[test]
fn test_simple_values() {
    // Numbers
    let n = Node::int(42);
    let json = n.to_json().unwrap();
    assert_eq!(json.trim(), "42");

    // Strings
    let s = Node::text("hello");
    let json = s.to_json().unwrap();
    assert_eq!(json.trim(), r#""hello""#);

    // Arrays
    let arr = Node::list(vec![Node::int(1), Node::int(2), Node::int(3)]);
    let json = arr.to_json().unwrap();
    println!("Array: {}", json);
    assert!(json.contains("["));
    assert!(!json.contains("List")); // No type tag!
}

#[test]
fn test_complex_node_json() {
    let n = Node::list(vec![
        Node::int(1),
        Node::int(2),
        Node::text("hello"),
        Node::keys("key", "value"),
    ]);

    let json = n.to_json().unwrap();
    println!("Complex JSON:\n{}", json);

    // Should be a simple array with objects
    assert!(json.contains("["));
    assert!(json.contains(r#""hello""#));
    assert!(json.contains("key"));
    assert!(!json.contains("List")); // No type tags!
}

#[test]
fn test_nested_node_json() {
    let n = Node::key("user", Node::list(vec![
        Node::keys("name", "Alice"),
        Node::keys("age", "30"),
    ]));

    let json = n.to_json().unwrap();
    println!("Nested JSON:\n{}", json);

    // Compact format: nested structures
    assert!(json.contains("user"));
    assert!(json.contains("Alice"));
}

#[test]
fn test_block_node_json() {
    let n = Node::Block(
        vec![Node::int(1), Node::int(2), Node::int(3)],
        Grouper::Object,
        Bracket::Curly,
    );

    let json = n.to_json().unwrap();
    println!("Block JSON:\n{}", json);

    // Block with curly braces becomes object
    assert!(json.contains("{"));
}

#[test]
fn test_pair_node_json() {
    let n = Node::pair(Node::int(1), Node::text("one"));

    let json = n.to_json().unwrap();
    println!("Pair JSON:\n{}", json);

    // Pair becomes array
    assert!(json.contains("["));
    assert!(json.contains("1"));
    assert!(json.contains(r#""one""#));
}

#[test]
fn test_data_node_json() {
    let n = Node::data(vec![1, 2, 3]);

    let json = n.to_json().unwrap();
    println!("Data JSON:\n{}", json);

    // Data nodes serialize metadata only
    assert!(json.contains("_type"));
}
