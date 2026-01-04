use wasp::eq;
use wasp::node::Node;
use wasp::wasp_parser::parse_xml;

#[test]
fn test_simple_xml_tag() {
	let xml = "<div>Hello</div>";
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should be Key("div", Text("Hello"))
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "div");
		eq!(**value, Node::Text("Hello".to_string())); // **unbox de&reference &Box<Node>
	} else {
		panic!("Expected Key node");
	}
}

#[test]
fn test_xml_with_attributes() {
	let xml = r#"<div class="container" id="main">Content</div>"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should be Key("div", List([Key(".class", "container"), Key(".id", "main"), Text("Content")]))
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "div");
		if let Node::List(items, _, _) = value.as_ref() {
			eq!(items.len(), 3);
			// Check first attribute
			if let Node::Key(attr_name, attr_val) = &items[0] {
				eq!(attr_name, ".class");
				eq!(**attr_val, Node::Text("container".to_string()));
			}
		} else {
			panic!("Expected List node for body");
		}
	} else {
		panic!("Expected Key node");
	}
}

#[test]
fn test_self_closing_tag() {
	let xml = "<br />";
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should be Key("br", Empty)
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "br");
		eq!(**value, Node::Empty);
	} else {
		panic!("Expected Key node");
	}
}

#[test]
fn test_nested_xml() {
	let xml = "<div><p>Paragraph</p><span>Text</span></div>";
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should be Key("div", List([Key("p", ...), Key("span", ...)]))
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "div");
		if let Node::List(items, _, _) = value.as_ref() {
			eq!(items.len(), 2);
			// Check first child
			if let Node::Key(child_name, _) = items[0].drop_meta() {
				eq!(child_name, "p");
			}
			// Check second child
			if let Node::Key(child_name, _) = items[1].drop_meta() {
				eq!(child_name, "span");
			}
		} else {
			panic!("Expected List node for body with children");
		}
	} else {
		panic!("Expected Key node");
	}
}

#[test]
fn test_xml_roundtrip() {
	// Test that we can parse XML, convert to XML, and re-parse
	let xml = r#"<html><head><title>My Page</title></head><body><h1>Hello</h1><p class="intro">Welcome</p></body></html>"#;
	let node1 = parse_xml(xml);
	println!("Parsed 1: {:?}", node1);

	// Convert back to XML
	let xml_output = node1.to_xml();
	println!("to_xml(): {}", xml_output);

	// Parse the XML output
	let node2 = parse_xml(&xml_output);
	println!("Parsed 2: {:?}", node2);

	// Compare structure (should be identical)
	eq!(node1, node2);

	// Verify it's valid XML
	assert!(xml_output.contains("<html>"));
	assert!(xml_output.contains("</html>"));
	assert!(xml_output.contains("class=\"intro\""));
}

#[test]
fn test_xml_to_json() {
	let xml = r#"<div class="box" id="main"><p>Text</p></div>"#;
	let node = parse_xml(xml);
	let json = node.to_json().unwrap();
	println!("XML to JSON:\n{}", json);

	// Should have dotted keys for attributes
	assert!(json.contains(".class"));
	assert!(json.contains("box"));
	assert!(json.contains("div"));
}

#[test]
fn test_boolean_attribute() {
	let xml = r#"<input type="checkbox" checked />"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should have .checked: true
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "input");
		if let Node::List(items, _, _) = value.as_ref() {
			// Find the checked attribute
			let has_checked = items.iter().any(|item| {
				if let Node::Key(attr_name, attr_val) = item {
					attr_name == ".checked" && **attr_val == Node::True
				} else {
					false
				}
			});
			assert!(has_checked, "Expected .checked attribute with True value");
		}
	}
}

#[test]
fn test_complex_xml_document() {
	let xml = r#"
<html>
	<head>
		<title>Test Page</title>
		<meta charset="utf-8" />
	</head>
	<body>
		<div class="container">
			<h1>Welcome</h1>
			<p class="intro">This is a test.</p>
			<ul>
				<li>Item 1</li>
				<li>Item 2</li>
			</ul>
		</div>
	</body>
</html>
"#;

	let node = parse_xml(xml);
	println!("Complex XML parsed successfully");

	// Verify basic structure
	if let Node::Key(name, _) = node.drop_meta() {
		eq!(name, "html");
	} else {
		panic!("Expected html root");
	}

	// Test that re-parsing same XML gives same structure
	let node2 = parse_xml(xml);
	eq!(node, node2);

	// Test XML serialization roundtrip
	let xml_output = node.to_xml();
	println!("XML output: {}", xml_output);
	let node3 = parse_xml(&xml_output);
	eq!(node, node3);

	// Also test Wasp notation serialization
	let serialized = node.serialize();
	println!("Serialized (Wasp notation): {}", serialized);
	assert!(serialized.contains("html"));
	assert!(serialized.contains(".class")); // Attributes use dotted notation
}

#[test]
fn test_to_xml_simple() {
	let xml = "<div>Hello World</div>";
	let node = parse_xml(xml);
	let output = node.to_xml();
	println!("Input:  {}", xml);
	println!("Output: {}", output);

	assert!(output.contains("<div>"));
	assert!(output.contains("Hello World"));
	assert!(output.contains("</div>"));

	// Roundtrip test
	let reparsed = parse_xml(&output);
	eq!(node, reparsed);
}

#[test]
fn test_to_xml_with_attributes() {
	let xml = r#"<div class="container" id="main">Content</div>"#;
	let node = parse_xml(xml);
	let output = node.to_xml();
	println!("Input:  {}", xml);
	println!("Output: {}", output);

	assert!(output.contains("<div"));
	assert!(output.contains("class=\"container\""));
	assert!(output.contains("id=\"main\""));
	assert!(output.contains("Content"));
	assert!(output.contains("</div>"));

	// Roundtrip test
	let reparsed = parse_xml(&output);
	eq!(node, reparsed);
}

#[test]
fn test_to_xml_self_closing() {
	let xml = r#"<img src="photo.jpg" />"#;
	let node = parse_xml(xml);
	let output = node.to_xml();
	println!("Input:  {}", xml);
	println!("Output: {}", output);

	assert!(output.contains("<img"));
	assert!(output.contains("src=\"photo.jpg\""));
	assert!(output.contains("/>"));

	// Roundtrip test
	let reparsed = parse_xml(&output);
	eq!(node, reparsed);
}

#[test]
fn test_to_xml_nested() {
	let xml = "<div><p>First</p><p>Second</p></div>";
	let node = parse_xml(xml);
	let output = node.to_xml();
	println!("Input:  {}", xml);
	println!("Output: {}", output);

	assert!(output.contains("<div>"));
	assert!(output.contains("<p>First</p>"));
	assert!(output.contains("<p>Second</p>"));
	assert!(output.contains("</div>"));

	// Roundtrip test
	let reparsed = parse_xml(&output);
	eq!(node, reparsed);
}

#[test]
fn test_to_xml_boolean_attribute() {
	let xml = r#"<input type="checkbox" checked />"#;
	let node = parse_xml(xml);
	let output = node.to_xml();
	println!("Input:  {}", xml);
	println!("Output: {}", output);

	assert!(output.contains("<input"));
	assert!(output.contains("type=\"checkbox\""));
	assert!(output.contains("checked"));
	assert!(output.contains("/>"));

	// Roundtrip test
	let reparsed = parse_xml(&output);
	eq!(node, reparsed);
}
