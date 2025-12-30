use wasp::wasp_parser::WaspParser;
use wasp::node::Node;

#[test]
fn test_wasp_to_json() {
    let wasp = r#"html{
        ul{ li:"hi" li:"ok" }
        colors=[red, green, blue]
    }"#;

    let node = WaspParser::parse(wasp).unwrap();
    let json = node.to_json().unwrap();

    println!("WASP:\n{}\n", wasp);
    println!("JSON:\n{}", json);

    assert!(json.contains("html"));
    assert!(json.contains("colors"));
}

#[test]
fn test_function_syntax() {
    let wasp = "def myfun(a, b){ return a + b }";
    let node = WaspParser::parse(wasp).unwrap();
    let json = node.to_json().unwrap();

    println!("WASP: {}", wasp);
    println!("JSON: {}", json);

    // Function is represented as Pair
    if let Node::Pair(sig, body) = &node {
        println!("Signature: {:?}", sig);
        println!("Body: {:?}", body);
    }
}

#[test]
fn test_nested_structures() {
    let wasp = r#"
        config {
            server {
                host: "localhost"
                port: 8080
            }
            database {
                url: "postgresql://..."
                pool_size: 10
            }
        }
    "#;

    let node = WaspParser::parse(wasp).unwrap();
    let json = node.to_json().unwrap();

    println!("WASP config:\n{}\n", wasp);
    println!("JSON:\n{}", json);

    assert!(json.contains("config"));
    assert!(json.contains("server"));
    assert!(json.contains("database"));
}

#[test]
fn test_mixed_syntax() {
    // Test both : and = for key-value
    let wasp = r#"{
        name: "Alice"
        age = 30
        tags = [rust, developer, engineer]
        address {
            city: "San Francisco"
            zip = 94102
        }
    }"#;

    let node = WaspParser::parse(wasp).unwrap();
    let json = node.to_json().unwrap();

    println!("WASP:\n{}\n", wasp);
    println!("JSON:\n{}", json);

    assert!(json.contains("Alice"));
    assert!(json.contains("30"));
    assert!(json.contains("rust"));
}

#[test]
fn test_wasp_roundtrip() {
    let wasp = r#"user{ name:"Bob" age:25 active:true }"#;
    let node = WaspParser::parse(wasp).unwrap();

    // Convert to JSON
    let json = node.to_json().unwrap();
    println!("Original WASP: {}", wasp);
    println!("JSON output:\n{}", json);

    // Verify structure
    assert_eq!(node.get_key().unwrap(), "user");
}

#[test]
fn test_list_operations() {
    let wasp = "numbers=[1, 2, 3, 4, 5]";
    let node = WaspParser::parse(wasp).unwrap();

    if let Some(value) = node.get_value() {
        if let Node::List(items) = value {
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], 1);
            assert_eq!(items[4], 5);
        }
    }
}

#[test]
fn test_empty_structures() {
    let wasp = "empty{}";
    let node = WaspParser::parse(wasp).unwrap();
    let json = node.to_json().unwrap();

    println!("Empty block: {}", json);
    assert!(json.contains("empty"));
}
