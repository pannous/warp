use wasp::*;
use wasp::node::Node;
use wasp::node::Node::{Empty, False, True};
// use wasp::node::Node::*;

#[test]
fn test_node(){
    // let n:Node = Node::new();
    // eval("key=value");
    let n:Node = Node::keys("key", "value");
    eq!(n.get_key(), "key");
    eq!(n.get_value(), &Node::text("value"));
    // let n:Node = Key("key".s(), Box::new(Text("value".s())));
    println!("{:?}", n );
}

#[test]
fn test_node_list() {
    let n: Node = Node::ints(vec![1, 2, 3]);
    println!("{:?}", n);
    assert_eq!(n[0], 1);
}

#[test]
fn test_node_index_str() {
    use wasp::node::{Bracket, Grouper};
    // Test indexing with &str on Block containing Key nodes
    let block = Node::Block(
        vec![
            Node::key("name", Node::text("Alice")),
            Node::key("age", Node::int(30)),
        ],
        Grouper::Object,
        Bracket::Curly,
    );
    assert_eq!(block["name"], Node::text("Alice"));
    assert_eq!(block["age"], 30);
    assert_eq!(block["nonexistent"], Node::Empty);
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

#[test]
fn test_roots() {
    assert!(Empty == 0);
    // is!((char *) "'hello'", "hello");
    is!("hello", "hello"); // todo reference==string really?
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
    skip!(
        is!("()", Empty);
        is!("{}", Empty); // NOP
    );
}
