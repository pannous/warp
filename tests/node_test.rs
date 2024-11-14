use wasp::*;
use wasp::node::Node;
use wasp::node::Node::*;

#[test]
fn test_node(){
    // let n:Node = Node::new();
    let n:Node = Node::keys("key", "value");
    // let n:Node = KeyValue("key".s(), Box::new(Text("value".s())));
    println!("{:?}", n );
}


#[test]
fn test_node_list() {
    let n: Node = Node::ints(vec![1, 2, 3]);
    println!("{:?}", n);
    assert_eq!(n[0], Node::Number(wasp::Number::Int(1)));
}


#[test]
fn test_node_equality(){
    let n:Node = Node::number(1);
    let n2:Node = Node::number(2);
    assert_eq!(n, n2);
}


#[test]
fn test_node_box() {
    let n: Node = Node::Data("data".into());
    println!("{:?}", n);
    assert_eq!(n, Node::Data("data".into()));
}

