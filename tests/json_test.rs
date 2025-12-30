use wasp::node::Node;

#[test]
fn test_node_to_json() {
    let n = Node::keys("name", "Alice");
    let json = n.to_json().unwrap();
    println!("JSON:\n{}", json);

    // Verify it's valid JSON
    assert!(json.contains("KeyValue"));
    assert!(json.contains("name"));
    assert!(json.contains("Alice"));
}

#[test]
fn test_node_from_json() {
    let n = Node::keys("age", "30");
    let json = n.to_json().unwrap();

    let n2 = Node::from_json(&json).unwrap();
    assert_eq!(n.get_key(), n2.get_key());
    assert_eq!(n.get_value(), n2.get_value());
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

    let n2 = Node::from_json(&json).unwrap();

    // Verify structure
    if let Node::List(items) = &n2 {
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], 1);
        assert_eq!(items[1], 2);
    } else {
        panic!("Not a list");
    }
}

#[test]
fn test_nested_node_json() {
    let n = Node::key("user", Node::list(vec![
        Node::keys("name", "Alice"),
        Node::keys("age", "30"),
    ]));

    let json = n.to_json().unwrap();
    println!("Nested JSON:\n{}", json);

    let n2 = Node::from_json(&json).unwrap();
    assert_eq!(n2.get_key().unwrap(), "user");
}

#[test]
fn test_block_node_json() {
    use wasp::node::{Kind, Bracket};

    let n = Node::Block(
        vec![Node::int(1), Node::int(2), Node::int(3)],
        Kind::Object,
        Bracket::Curly,
    );

    let json = n.to_json().unwrap();
    println!("Block JSON:\n{}", json);

    let n2 = Node::from_json(&json).unwrap();
    if let Node::Block(items, _, _) = &n2 {
        assert_eq!(items.len(), 3);
    }
}

#[test]
fn test_pair_node_json() {
    let n = Node::pair(Node::int(1), Node::text("one"));

    let json = n.to_json().unwrap();
    println!("Pair JSON:\n{}", json);

    let n2 = Node::from_json(&json).unwrap();
    if let Node::Pair(a, b) = &n2 {
        assert_eq!(**a, 1);
        assert_eq!(**b, "one");
    }
}

#[test]
fn test_data_node_json() {
    let n = Node::data(vec![1, 2, 3]);

    let json = n.to_json().unwrap();
    println!("Data JSON:\n{}", json);

    // Data nodes serialize metadata only
    assert!(json.contains("type_name"));
    assert!(json.contains("data_type"));

    let n2 = Node::from_json(&json).unwrap();
    if let Node::Data(dada) = &n2 {
        assert!(dada.type_name.contains("Vec"));
    }
}
