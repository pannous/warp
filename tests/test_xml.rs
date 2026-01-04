use wasp::eq;
use wasp::Node;
use wasp::wasp_parser::parse_xml;
use Node::*;

#[test]
fn test_simple_xml_tag() {
	let xml = "<div>Hello</div>";
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);
	eq!(node.name(), "div");
	// eq!(node.key(), "div");
	eq!(node.get_key(), "div");
	// eq!(node.first(), "div"); ambiguous: is it key or value? Hello? even "H"?
	// eq!(node.get_key().name(), "div"); // get_key returns &str, not Node
	eq!(node.value(), "Hello");
	eq!(node["div"], "Hello");

	// Should be Key("div", Text("Hello"))
	if let Key(name, value) = node.drop_meta() {
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
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_parse_all_xml_files() {
	use std::fs;
	use std::path::Path;

	println!("\n=== Parsing all XML files in filesystem ===\n");

	let search_paths = vec![
		"/usr/share",
		"/System/Library",
		"/Library",
		"/Applications",
		"/Users/me",
	];

	let mut total_files = 0;
	let mut successful = 0;
	let mut failed = 0;
	let mut errors = Vec::new();

	for search_path in &search_paths {
		if !Path::new(search_path).exists() {
			continue;
		}

		println!("Searching in {}...", search_path);

		// Find XML files (limit depth to avoid taking too long)
		if let Ok(entries) = find_xml_files(search_path, 3) {
			for file_path in entries {
				total_files += 1;

				// Read and parse
				if let Ok(content) = fs::read_to_string(&file_path) {
					// Skip very large files (> 1MB)
					if content.len() > 1_000_000 {
						continue;
					}

					let node = parse_xml(&content);

					// Check if parsing succeeded (no Error nodes in result)
					if contains_error(&node) {
						failed += 1;
						if errors.len() < 10 {
							// Only keep first 10 errors
							errors.push((file_path.clone(), node.serialize()));
						}
					} else {
						successful += 1;

						// Test roundtrip on successful parses
						let xml_out = node.to_xml();
						let reparsed = parse_xml(&xml_out);
						if node != reparsed {
							println!("  ⚠️  Roundtrip mismatch: {}", file_path);
						}
					}
				}

				// Print progress every 100 files
				if total_files % 100 == 0 {
					println!("  Processed {} files...", total_files);
				}
			}
		}
	}

	println!("\n=== Results ===");
	println!("Total files:  {}", total_files);
	println!("Successful:   {} ({:.1}%)", successful, (successful as f64 / total_files as f64) * 100.0);
	println!("Failed:       {} ({:.1}%)", failed, (failed as f64 / total_files as f64) * 100.0);

	if !errors.is_empty() {
		println!("\n=== First {} Errors ===", errors.len());
		for (path, error) in errors {
			println!("File: {}", path);
			println!("Error: {}", error);
			println!();
		}
	}

	// Test passes if we successfully parsed at least some files
	assert!(successful > 0, "Should successfully parse at least some XML files");
}

fn find_xml_files(path: &str, max_depth: usize) -> std::io::Result<Vec<String>> {
	use std::path::Path;

	let mut xml_files = Vec::new();
	find_xml_files_recursive(Path::new(path), max_depth, 0, &mut xml_files)?;
	Ok(xml_files)
}

fn find_xml_files_recursive(
	path: &std::path::Path,
	max_depth: usize,
	current_depth: usize,
	results: &mut Vec<String>,
) -> std::io::Result<()> {
	if current_depth >= max_depth {
		return Ok(());
	}

	if let Ok(entries) = std::fs::read_dir(path) {
		for entry in entries.flatten() {
			let path = entry.path();

			// Skip hidden files and system directories
			if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
				if name.starts_with('.') {
					continue;
				}
			}

			if path.is_file() {
				if let Some(ext) = path.extension() {
					if ext == "xml" {
						if let Some(path_str) = path.to_str() {
							results.push(path_str.to_string());

							// Limit to first 1000 files to avoid taking forever
							if results.len() >= 1000 {
								return Ok(());
							}
						}
					}
				}
			} else if path.is_dir() {
				// Recurse into subdirectory
				let _ = find_xml_files_recursive(&path, max_depth, current_depth + 1, results);
			}
		}
	}

	Ok(())
}

fn contains_error(node: &wasp::Node) -> bool {
	use wasp::Node::*;

	match node {
		Error(_) => true,
		List(items, _, _) => items.iter().any(contains_error),
		Key(_, v) => contains_error(v),
		Meta { node, .. } => contains_error(node),
		_ => false,
	}
}

#[test]
fn test_xml_declaration() {
	let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<root>Content</root>"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// XML declaration should be skipped, only root tag parsed
	if let Node::Key(name, _) = node.drop_meta() {
		eq!(name, "root");
	} else {
		panic!("Expected root Key node");
	}
}

#[test]
fn test_doctype() {
	let xml = r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
<html><body>Test</body></html>"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// DOCTYPE should be skipped
	if let Node::Key(name, _) = node.drop_meta() {
		eq!(name, "html");
	} else {
		panic!("Expected html Key node");
	}
}

#[test]
fn test_xml_comments() {
	let xml = r#"<!-- This is a comment -->
<div>Content</div>
<!-- Another comment -->"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Comments should be skipped
	if let Node::Key(name, _) = node.drop_meta() {
		eq!(name, "div");
	} else {
		panic!("Expected div Key node");
	}
}

#[test]
fn test_cdata() {
	let xml = r#"<script><![CDATA[
function test() {
    if (x < 5 && y > 3) {
        return true;
    }
}
]]></script>"#;
	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// CDATA content should be preserved as text
	if let Node::Key(name, value) = node.drop_meta() {
		eq!(name, "script");
		// Content should include the JavaScript
		let text = value.serialize();
		assert!(text.contains("function test"));
		assert!(text.contains("x < 5"));
	} else {
		panic!("Expected script Key node");
	}
}

#[test]
fn test_real_world_xml() {
	// Example from macOS system files
	let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Name</key>
	<string>Test</string>
</dict>
</plist>"#;

	let node = parse_xml(xml);
	println!("Parsed: {:?}", node);

	// Should successfully parse despite XML declaration and DOCTYPE
	if let Node::Key(name, _) = node.drop_meta() {
		eq!(name, "plist");
	} else {
		panic!("Expected plist Key node");
	}

	// Test roundtrip
	let xml_out = node.to_xml();
	let reparsed = parse_xml(&xml_out);
	eq!(node, reparsed);
}
