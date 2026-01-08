use warp::eq;
use warp::Node;
use warp::wasp_parser::WaspParser;

#[test]
fn test_wasp_to_json() {
	let wasp = r#"html{
        ul{ li:"hi" li:"ok" }
        colors=[red, green, blue]
    }"#;

	let node = WaspParser::parse(wasp);
	let json = node.to_json().unwrap();

	println!("WASP:\n{}\n", wasp);
	println!("JSON:\n{}", json);

	assert!(json.contains("html"));
	assert!(json.contains("colors"));
}

#[test]
fn test_function_syntax() {
	let wasp = "def myfun(a, b){ return a + b }";
	let node = WaspParser::parse(wasp);
	let json = node.to_json().unwrap();

	println!("WASP: {}", wasp);
	println!("JSON: {}", json);
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

	let node = WaspParser::parse(wasp);
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

	let node = WaspParser::parse(wasp);
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
	let node = WaspParser::parse(wasp);

	// Convert to JSON
	let json = node.to_json().unwrap();
	println!("Original WASP: {}", wasp);
	println!("JSON output:\n{}", json);

	// Verify structure - user{...} becomes Tag
	if let Node::Key(title, ..) = node.drop_meta() {
		if let Node::Symbol(s) | Node::Text(s) = title.as_ref() {
			eq!(s, "user");
		} else {
			panic!("Expected Symbol or Text key");
		}
	} else {
		panic!("Expected Tag node");
	}
}

#[test]
fn test_list_operations() {
	let wasp = "numbers=[1, 2, 3, 4, 5]";
	let node = WaspParser::parse(wasp);

	let value = node.get_value();
	if let Node::List(items, _, _) = value {
		eq!(items.len(), 5);
		eq!(items[0], 1);
		eq!(items[4], 5);
	}
}

#[test]
fn test_empty_structures() {
	// peq!("leer{}", Node::Empty);
	let wasp = "leer{}";
	let node = WaspParser::parse(wasp);
	let json = node.to_json().unwrap();

	println!("Empty block: {}", json);
	assert!(json.contains("leer"));
}
