use crate::eq;
use warp::Node;
use warp::warp_parser::WarpParser;

#[test]
fn test_warp_to_json() {
	let warp = r#"html{
        ul{ li:"hi" li:"ok" }
        colors=[red, green, blue]
    }"#;

	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("WARP:\n{}\n", warp);
	println!("JSON:\n{}", json);

	assert!(json.contains("html"));
	assert!(json.contains("colors"));
}

#[test]
fn test_function_syntax() {
	let warp = "def myfun(a, b){ return a + b }";
	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("WARP: {}", warp);
	println!("JSON: {}", json);
}

#[test]
fn test_nested_structures() {
	let warp = r#"
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

	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("WARP config:\n{}\n", warp);
	println!("JSON:\n{}", json);

	assert!(json.contains("config"));
	assert!(json.contains("server"));
	assert!(json.contains("database"));
}

#[test]
fn test_mixed_syntax() {
	// Test both : and = for key-value
	let warp = r#"{
        name: "Alice"
        age = 30
        tags = [rust, developer, engineer]
        address {
            city: "San Francisco"
            zip = 94102
        }
    }"#;

	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("WARP:\n{}\n", warp);
	println!("JSON:\n{}", json);

	assert!(json.contains("Alice"));
	assert!(json.contains("30"));
	assert!(json.contains("rust"));
}

#[test]
fn test_warp_roundtrip() {
	let warp = r#"user{ name:"Bob" age:25 active:true }"#;
	let node = WarpParser::parse(warp);

	// Convert to JSON
	let json = node.to_json().unwrap();
	println!("Original WARP: {}", warp);
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
	let warp = "numbers=[1, 2, 3, 4, 5]";
	let node = WarpParser::parse(warp);

	let value = node.get_value();
	if let Node::List(items, _, _) = value {
		eq!(items.len(), 5);
		eq!(items[0], 1);
		eq!(items[4], 5);
	}
}

#[test]
fn test_empty_structures() {
	// parses_to!("leer{}", Node::Empty);
	let warp = "leer{}";
	let node = WarpParser::parse(warp);
	let json = node.to_json().unwrap();

	println!("Empty block: {}", json);
	assert!(json.contains("leer"));
}

#[test]
fn html_attributes_and_repeated_tags_in_data() {
	let parsed = warp::warp_parser::parse_data("{div{class:\"ab\"} div{type:email}}");
	assert_eq!(parsed.serialize(), "{div{class:\"ab\"} div{type:email}}");
	crate::is!("x={class:\"btn\" type:\"email\"}; x.class", "btn");
	crate::is!("7 div 2", 3);
}
