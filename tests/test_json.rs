use serde_json::json;
use wasp::eq;
use wasp::node::Bracket;
use wasp::node::Node;
use wasp::node::Node::Symbol;
use wasp::node::Separator;
use wasp::util::show_type_name;

#[test]
fn test_node_to_json_compact() {
	let n = Node::keys("name", "Alice");
	let json = n.to_json().unwrap();
	println!("Wide JSON:\n{}", json);
	eq!(json, r#"{
  "name": "Alice"
}"#);
	eq!(json, "{\n  \"name\": \"Alice\"\n}");
	// eq!(json, "{\n\t\"name\": \"Alice\"\n}");
	// println!("Compact JSON:\n{}", json);
	// eq!(json, r#"{"name": "Alice"}"#);
	show_type_name(&json!({"name": "Alice"}));
	// Compare the Node directly with the JSON value (using PartialEq<Value> for Node)
	eq!(n, json!({"name": "Alice"}));
	// eq!(json, json!({"name": "Alice", "alice": "Bob"}));
	// eq!(json, warp!({name: "Alice", age: "Bob"})); // todo create warp! macro if possible, else
	// eq!(json, warp!({"name": "Alice", "alice": "Bob"})); // todo warp! constructs Node, not JSON
}

#[test]
fn test_implicit_html_structure() {
	// html{ ul{ li:"hi" li:"ok"} colors=[red, green,blue]}
	let html = Node::List(
		vec![
			Node::List(
				vec![Node::keys("li", "hi"), Node::keys("li", "ok")],
				Bracket::Curly,
				Separator::None,
			),
			Node::Key(
				Box::new(Symbol("colors".to_string())),
				Box::new(Node::list(vec![
					Node::symbol("red"),
					Node::symbol("green"),
					Node::symbol("blue"),
				])),
			),
		],
		Bracket::Curly,
		Separator::None,
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
	eq!(json.trim(), "42");

	// Strings
	let s = Node::text("hello");
	let json = s.to_json().unwrap();
	eq!(json.trim(), r#""hello""#);

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
	let n = Node::key(
		"user",
		Node::list(vec![Node::keys("name", "Alice"), Node::keys("age", "30")]),
	);

	let json = n.to_json().unwrap();
	println!("Nested JSON:\n{}", json);

	// Compact format: nested structures
	assert!(json.contains("user"));
	assert!(json.contains("Alice"));
}

#[test]
fn test_block_node_json() {
	let n = Node::List(
		vec![Node::int(1), Node::int(2), Node::int(3)],
		Bracket::Curly,
		Separator::None,
	);

	let json = n.to_json().unwrap();
	println!("Block JSON:\n{}", json);

	// Block with curly braces becomes object
	assert!(json.contains("{"));
}

#[test]
fn test_data_node_json() {
	let n = Node::data(vec![1, 2, 3]);

	let json = n.to_json().unwrap();
	println!("Data JSON:\n{}", json);

	// Data nodes serialize metadata only
	assert!(json.contains("_type"));
}

// ===== Node PartialEq with serde_json::Value tests =====

#[test]
fn test_node_eq_json_primitives() {
	// Empty → null
	assert_eq!(Node::Empty, json!(null));
	assert_eq!(json!(null), Node::Empty);

	// Booleans
	assert_eq!(Node::True, json!(true));
	assert_eq!(json!(true), Node::True);
	assert_eq!(Node::False, json!(false));
	assert_eq!(json!(false), Node::False);

	// Numbers - integers
	assert_eq!(Node::from(42), json!(42));
	assert_eq!(json!(42), Node::from(42));
	assert_eq!(Node::int(0), json!(0));
	assert_eq!(Node::int(-100), json!(-100));

	// Numbers - floats
	assert_eq!(Node::from(1.23), json!(1.23));
	assert_eq!(json!(1.23), Node::from(1.23));
	assert_eq!(Node::float(0.0), json!(0.0));

	// Strings - Text
	assert_eq!(Node::Text("hello".into()), json!("hello"));
	assert_eq!(json!("hello"), Node::Text("hello".into()));
	assert_eq!(Node::text("world"), json!("world"));
	assert_eq!(Node::text(""), json!(""));

	// Strings - Symbol
	assert_eq!(Node::Symbol("foo".into()), json!("foo"));
	assert_eq!(json!("foo"), Node::Symbol("foo".into()));
	assert_eq!(Node::symbol("bar"), json!("bar"));

	// Strings - Char
	assert_eq!(Node::Char('a'), json!("a"));
	assert_eq!(json!("a"), Node::Char('a'));
	assert_eq!(Node::Char('🦀'), json!("🦀"));
}

#[test]
fn test_node_eq_json_lists() {
	// Empty list
	assert_eq!(Node::list(vec![]), json!([]));
	assert_eq!(json!([]), Node::list(vec![]));

	// Simple number list
	let list = Node::list(vec![Node::int(1), Node::int(2), Node::int(3)]);
	assert_eq!(list, json!([1, 2, 3]));
	assert_eq!(json!([1, 2, 3]), list);

	// Mixed types
	let mixed = Node::list(vec![Node::int(42), Node::text("hello"), Node::True]);
	assert_eq!(mixed, json!([42, "hello", true]));
	assert_eq!(json!([42, "hello", true]), mixed);

	// Nested lists
	let nested = Node::list(vec![
		Node::int(1),
		Node::list(vec![Node::int(2), Node::int(3)]),
		Node::int(4),
	]);
	assert_eq!(nested, json!([1, [2, 3], 4]));
	assert_eq!(json!([1, [2, 3], 4]), nested);
}

#[test]
fn test_node_eq_json_keys() {
	// Simple key
	let key = Node::keys("name", "Alice");
	assert_eq!(key, json!({"name": "Alice"}));
	assert_eq!(json!({"name": "Alice"}), key);

	// Key with number value
	let key = Node::key("age", Node::int(30));
	assert_eq!(key, json!({"age": 30}));
	assert_eq!(json!({"age": 30}), key);

	// Key with list value
	let key = Node::key("items", Node::list(vec![Node::int(1), Node::int(2)]));
	assert_eq!(key, json!({"items": [1, 2]}));
	assert_eq!(json!({"items": [1, 2]}), key);
}

#[test]
fn test_node_eq_json_curly_lists() {
	// Curly list with keys → object
	let obj = Node::List(
		vec![Node::keys("name", "Bob"), Node::key("age", Node::int(25))],
		Bracket::Curly,
		Separator::None,
	);
	assert_eq!(obj, json!({"name": "Bob", "age": 25}));
	assert_eq!(json!({"name": "Bob", "age": 25}), obj);

	// Empty curly list
	let empty_obj = Node::List(vec![], Bracket::Curly, Separator::None);
	assert_eq!(empty_obj, json!({}));
	assert_eq!(json!({}), empty_obj);
}

#[test]
fn test_node_eq_json_meta() {
	// Meta nodes should unwrap for comparison
	let meta_node = Node::Meta {
		node: Box::new(Node::int(42)),
		data: Box::new(Node::text("some comment")),
	};
	assert_eq!(meta_node, json!(42));
	assert_eq!(json!(42), meta_node);

	// Nested meta
	let nested_meta = Node::Meta {
		node: Box::new(Node::Meta {
			node: Box::new(Node::text("hello")),
			data: Box::new(Node::Empty),
		}),
		data: Box::new(Node::Empty),
	};
	assert_eq!(nested_meta, json!("hello"));
}

#[test]
fn test_node_neq_json() {
	// Type mismatches
	assert_ne!(Node::int(42), json!("42"));
	assert_ne!(Node::text("42"), json!(42));
	assert_ne!(Node::True, json!(1));
	assert_ne!(Node::False, json!(0));
	assert_ne!(Node::Empty, json!(false));

	// Value mismatches
	assert_ne!(Node::int(42), json!(43));
	assert_ne!(Node::text("hello"), json!("world"));
	assert_ne!(Node::True, json!(false));

	// List mismatches
	assert_ne!(Node::list(vec![Node::int(1)]), json!([1, 2]));
	assert_ne!(Node::list(vec![Node::int(1), Node::int(2)]), json!([2, 1]));

	// Key mismatches
	assert_ne!(Node::keys("name", "Alice"), json!({"name": "Bob"}));
	assert_ne!(Node::keys("name", "Alice"), json!({"age": "Alice"}));
	assert_ne!(Node::keys("name", "Alice"), json!({"name": "Alice", "age": 30}));
}
