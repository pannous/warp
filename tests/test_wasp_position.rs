use wasp::eq;
use wasp::node::Node;
use wasp::wasp_parser::WaspParser;

#[test]
fn test_position_tracking() {
	let wasp = r#"name: "Alice"
age: 30
city: "NYC""#;

	let node = WaspParser::parse(wasp);
	println!("Parsed: {:?}", node);

	if let Node::List(items, _, _) = node {
		// First item should be at line 1
		if let Some(first) = items.get(0) {
			if let Some(meta) = first.get_lineinfo() {
				println!(
					"First item position: line {:?}, column {:?}",
					meta.line_nr, meta.column
				);
				eq!(meta.line_nr, 1);
			}
		}

		// Second item should be at line 2
		if let Some(second) = items.get(1) {
			if let Some(meta) = second.get_lineinfo() {
				println!("Second item position: line {:?}, column {:?}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 2);
			}
		}

		// Third item should be at line 3
		if let Some(third) = items.get(2) {
			if let Some(meta) = third.get_lineinfo() {
				println!("Third item position: line {:?}, column {:?}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 3);
			}
		}
	}
}

#[test]
fn test_position_with_comments() {
	let wasp = r#"// User data
name: "Bob"
// Age field
age: 25"#;

	let node = WaspParser::parse(wasp);
	println!("Parsed with comments: {:?}", node);

	if let Node::List(items, _, _) = node {
		if let Some(first) = items.get(0) {
			if let Some(meta) = first.get_lineinfo() {
				println!("First: line={:?}, col={:?}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 2);
			}
		}

		if let Some(second) = items.get(1) {
			if let Some(meta) = second.get_lineinfo() {
				println!("Second: line={:?}, col={:?}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 4);
			}
		}
	}
}

#[test]
fn test_nested_position_tracking() {
	let wasp = r#"server {
    port: 8080
    host: "localhost"
}"#;

	let node = WaspParser::parse(wasp);
	println!("Nested structure: {:?}", node);
	if let Some(meta) = node.get_lineinfo() {
		println!("Server tag position: line={:?}, col={:?}", meta.line_nr, meta.column);
		eq!(meta.line_nr, 1);
	}
}

#[test]
fn test_multiline_structure() {
	let wasp = r#"html{
    header{ title:"Site" }
    body{ content:"Hello" }
}"#;

	let node = WaspParser::parse(wasp);
	println!("HTML structure: {:?}", node);

	// Top-level html should be at line 1
	if let Some(meta) = node.get_lineinfo() {
		println!("HTML position: {:?}:{:?}", meta.line_nr, meta.column);
		eq!(meta.line_nr, 1);
	}
}

#[test]
fn test_column_tracking() {
	let wasp = "a:1 b:2 c:3";

	let node = WaspParser::parse(wasp);
	println!("Parsed: {:?}", node);

	if let Node::List(items, _, _) = node {
		if let Some(first) = items.get(0) {
			if let Some(meta) = first.get_lineinfo() {
				println!("First at: {}:{}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 1);
				eq!(meta.column, 1);
			}
		}

		if let Some(second) = items.get(1) {
			if let Some(meta) = second.get_lineinfo() {
				println!("Second at: {}:{}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 1);
				eq!(meta.column, 5);
			}
		}

		if let Some(third) = items.get(2) {
			if let Some(meta) = third.get_lineinfo() {
				println!("Third at: {}:{}", meta.line_nr, meta.column);
				eq!(meta.line_nr, 1);
				eq!(meta.column, 9);
			}
		}
	}
}

#[test]
fn test_position_in_json_output() {
	let wasp = "// Important\nvalue: 42";

	let node = WaspParser::parse(wasp);
	let json = node.to_json().unwrap();

	println!("JSON with metadata:\n{}", json);

	// JSON should preserve position in metadata
	assert!(json.contains("value"));
	assert!(json.contains("42"));
}
