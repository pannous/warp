use wasp::eq;
use wasp::Node;

#[derive(Clone, Debug, PartialEq)]
struct CustomDataExample {
	id: i32,
	name: String,
}

#[test]
fn test_node_data() {
	let custom = CustomDataExample {
		id: 42,
		name: "test".to_string(),
	};

	let n = Node::data(custom.clone());
	println!("{:?}", n);

	// Extract data back out using downcast
	if let Node::Data(dada) = &n {
		if let Some(extracted) = dada.downcast_ref::<CustomDataExample>() {
			eq!(extracted.id, 42);
			eq!(extracted.name, "test");
			println!("Extracted: {:?}", extracted);
		} else {
			panic!("Failed to downcast");
		}
	} else {
		panic!("Not a Data node");
	}
}

#[test]
fn test_node_data_clone() {
	let n = Node::data(vec![1, 2, 3]);
	let n2 = n.clone();

	if let Node::Data(dada) = &n2 {
		if let Some(vec) = dada.downcast_ref::<Vec<i32>>() {
			eq!(vec, &vec![1, 2, 3]);
			println!("Cloned data: {:?}", vec);
		}
	}
}

#[test]
fn test_node_data_various_types() {
	// String
	let s = Node::data("hello world".to_string());
	println!("String node: {:?}", s);

	// Vec
	let v = Node::data(vec![1.0, 2.0, 3.0]);
	println!("Vec node: {:?}", v);

	// Tuple
	let t = Node::data((42, "answer"));
	println!("Tuple node: {:?}", t);

	// Can store any cloneable type
	let bytes = Node::data(vec![0u8, 1u8, 2u8]);
	println!("Bytes node: {:?}", bytes);
}

#[test]
fn test_node_data_equality() {
	// Same values should be equal
	let n1 = Node::data(vec![1, 2, 3]);
	let n2 = Node::data(vec![1, 2, 3]);
	eq!(n1, n2);
	println!("Vec equality: {:?} == {:?}", n1, n2);

	// Different values should not be equal
	let n3 = Node::data(vec![1, 2, 4]);
	assert_ne!(n1, n3);
	println!("Vec inequality: {:?} != {:?}", n1, n3);

	// String equality
	let s1 = Node::data("test".to_string());
	let s2 = Node::data("test".to_string());
	eq!(s1, s2);

	let s3 = Node::data("other".to_string());
	assert_ne!(s1, s3);

	// Custom type equality
	let c1 = Node::data(CustomDataExample {
		id: 42,
		name: "test".to_string(),
	});
	let c2 = Node::data(CustomDataExample {
		id: 42,
		name: "test".to_string(),
	});
	eq!(c1, c2);

	let c3 = Node::data(CustomDataExample {
		id: 99,
		name: "test".to_string(),
	});
	assert_ne!(c1, c3);

	// Different types should not be equal
	let int_node = Node::data(42);
	let str_node = Node::data("42".to_string());
	assert_ne!(int_node, str_node);
}

#[test]
fn test_node_data_type_mismatch() {
	let n1 = Node::data(vec![1, 2, 3]);
	let n2 = Node::int(123);

	// Data node should not equal non-Data node
	assert_ne!(n1, n2);
}

#[test]
fn test_node_data_metadata() {
	use wasp::DataType;

	// Vec metadata
	let v = Node::data(vec![1, 2, 3]);
	if let Node::Data(dada) = &v {
		eq!(dada.data_type, DataType::Vec);
		assert!(dada.type_name.contains("Vec"));
		println!("Vec type: {:?}, name: {}", dada.data_type, dada.type_name);
	}

	// Tuple metadata
	let t = Node::data((42, "test"));
	if let Node::Data(dada) = &t {
		eq!(dada.data_type, DataType::Tuple);
		println!("Tuple type: {:?}, name: {}", dada.data_type, dada.type_name);
	}

	// String metadata
	let s = Node::data("hello".to_string());
	if let Node::Data(dada) = &s {
		eq!(dada.data_type, DataType::String);
		println!(
			"String type: {:?}, name: {}",
			dada.data_type, dada.type_name
		);
	}

	// Primitive metadata
	let p = Node::data(42i32);
	if let Node::Data(dada) = &p {
		eq!(dada.data_type, DataType::Primitive);
		eq!(dada.type_name, "i32");
		println!(
			"Primitive type: {:?}, name: {}",
			dada.data_type, dada.type_name
		);
	}

	// Custom struct metadata
	let c = Node::data(CustomDataExample {
		id: 1,
		name: "test".to_string(),
	});
	if let Node::Data(dada) = &c {
		eq!(dada.data_type, DataType::Struct);
		assert!(dada.type_name.contains("CustomDataExample"));
		println!(
			"Struct type: {:?}, name: {}",
			dada.data_type, dada.type_name
		);
	}

	// Debug output shows metadata
	println!("Vec debug: {:?}", v);
	println!("Tuple debug: {:?}", t);
	println!("Struct debug: {:?}", c);
}
