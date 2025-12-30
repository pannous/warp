use wast::parser::parse;
use wasp::*;
use wasp::node::Node;
// use wasp::node::Node::*;

#[test]
fn test_node(){
    // let n:Node = Node::new();
    // eval("key=value");
    let n:Node = Node::keys("key", "value");
    eq!(n.get_key().unwrap(), "key");
    eq!(n.get_value().unwrap(), &Node::text("value"));
    // let n:Node = KeyValue("key".s(), Box::new(Text("value".s())));
    println!("{:?}", n );
}

#[test]
fn test_node_list() {
    let n: Node = Node::ints(vec![1, 2, 3]);
    println!("{:?}", n);
    assert_eq!(n[0], 1);
}


#[test]
fn test_node_equality(){
    let n0:Node = Node::int(0);
    let n1:Node = Node::int(1);
    let n2:Node = Node::int(2);
    let n3:Node = Node::float(2.0);
    assert_eq!(n1, 1);
    assert_eq!(n2, 2);
    assert_eq!(n3, 2);
    assert_eq!(n3, 2.0);
    assert_eq!(n1, true);
    assert_eq!(n0, false);
    assert_ne!(n1, n2);
    assert_ne!(n1, 2);
}


#[test]
fn test_node_data_eq() {
    let n = Node::data(vec![1, 2, 3]);
    let n2 = n.clone();
    eq!(n,n2)
}

// #[test]
// fn test_node_box() {
//     let n: Node = Node::Data("data".into());
//     println!("{:?}", n);
//     assert_eq!(n, Node::Data("data".into()));
// }

