use wasp::node::Node;

#[derive(Clone, Debug, PartialEq)]
struct CustomData {
    id: i32,
    name: String,
}

#[test]
fn test_node_data() {
    let custom = CustomData {
        id: 42,
        name: "test".to_string(),
    };

    let n = Node::data(custom.clone());
    println!("{:?}", n);

    // Extract data back out using downcast
    if let Node::Data(dada) = &n {
        if let Some(extracted) = dada.downcast_ref::<CustomData>() {
            assert_eq!(extracted.id, 42);
            assert_eq!(extracted.name, "test");
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
            assert_eq!(vec, &vec![1, 2, 3]);
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
