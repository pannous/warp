use wasp::node::Node::{Empty, False, True};
use wasp::node::{Bracket, Node};
use wasp::*;

// use wasp::node::Node::*;

#[test]
fn test_node() {
	// let n:Node = Node::new();
	// eval("key=value");
	let n: Node = Node::keys("key", "value");
	eq!(n.get_key(), "key");
	eq!(n.get_value(), &Node::text("value"));
	// let n:Node = Key("key".s(), Box::new(Text("value".s())));
	println!("{:?}", n);
}

#[test]
fn test_node_list() {
	let n: Node = Node::ints(vec![1, 2, 3]);
	println!("{:?}", n);
	eq!(n[0], 1);
}

#[test]
fn test_node_index_str() {
	// Test indexing let str : with on Block containing Key nodes
	let mut block = Node::List(
		vec![
			Node::key("name", Node::text("Alice")),
			Node::key("age", Node::int(30)),
		],
		Bracket::Curly,
	);
	eq!(block["name"], Node::text("Alice"));
	eq!(block["age"], 30);
	eq!(block["nonexistent"], Node::Empty);

	// Test mutable indexing with automatic conversion
	block["name"] = "Bob".into();
	eq!(block["name"], "Bob");

	block["age"] = 25.into();
	eq!(block["age"], 25);
}

#[test]
fn test_node_not_operator() {
	// Boolean nodes
	eq!(!True, False);
	eq!(!False, True);

	// Empty/null
	eq!(!Empty, True);

	// Numbers
	eq!(!Node::int(0), True);
	eq!(!Node::int(1), False);
	eq!(!Node::int(42), False);
	eq!(!Node::float(0.0), True);
	eq!(!Node::float(3.14), False);

	// Strings
	eq!(!Node::text(""), True);
	eq!(!Node::text("hello"), False);
	eq!(!Node::symbol(""), True);
	eq!(!Node::symbol("x"), False);

	// Collections
	eq!(!Node::List(vec![], Bracket::Square), True);
	eq!(!Node::ints(vec![1, 2, 3]), False);
}

#[test]
fn test_node_equality() {
	let n0: Node = Node::int(0);
	let n1: Node = Node::int(1);
	let n2: Node = Node::int(2);
	let n3: Node = Node::float(2.0);
	eq!(n1, 1);
	eq!(n2, 2);
	eq!(n3, 2);
	eq!(n3, 2.0);
	eq!(n1, true);
	eq!(n0, false);
	assert_ne!(n1, n2);
	assert_ne!(n1, 2);
}

#[test]
fn test_node_data_eq() {
	let n = Node::data(vec![1, 2, 3]);
	let n2 = n.clone();
	eq!(n, n2)
}

// #[test]
// fn test_node_box() {
//     let n: Node = Node::Data("data".into());
//     println!("{:?}", n);
//     eq!(n, Node::Data("data".into()));
// }

#[test]
#[ignore]
fn test_roots() {
	assert!(Empty == 0);
	/* is!((char *) "'hello'", "hello"); */
	is!("True", True);
	is!("False", False);
	is!("true", True);
	is!("false", False);
	is!("yes", True);
	is!("no", False);
	//	is!("right", True);
	//	is!("wrong", False);
	is!("null", Empty);
	is!("", Empty);
	assert!(Empty == 0);
	is!("0", Empty);
	is!("1", 1);
	is!("123", 123);
	is!("()", Empty);
	is!("{}", Empty); // NOP
	is!("hello", "hello"); // todo reference==string really?
}
