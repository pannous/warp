use warp::Node;

#[test]
fn test_values_key() {
	let key = Node::key("name", Node::text("Alice"));
	let result = key.values();
	assert_eq!(result, "Alice");
}

#[test]
fn test_values_nested_key() {
	let inner_key = Node::key("inner", Node::int(42));
	let outer_key = Node::key("outer", inner_key);
	let result = outer_key.values();
	// Should return the inner Key node itself (not recursively unwrap)
	assert_eq!(result, &Node::key("inner", Node::int(42)));
}

#[test]
fn test_values_list() {
	let list = Node::list(vec![Node::int(1), Node::int(2), Node::int(3)]);
	let result = list.values();
	// List returns itself
	assert_eq!(result, &list);
}

#[test]
fn test_values_meta() {
	let key = Node::key("x", Node::int(10)).with_comment("test".to_string());
	let result = key.values();
	// Meta unwraps and calls values on inner node
	assert_eq!(result, 10);
}

#[test]
fn test_values_empty() {
	let empty = Node::Empty;
	let result = empty.values();
	assert_eq!(result, &Node::Empty);
}

#[test]
fn test_values_number() {
	let num = Node::int(42);
	let result = num.values();
	// Non-Key types return Empty
	assert_eq!(result, &Node::Empty);
}
