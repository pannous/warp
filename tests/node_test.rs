use wasp::*;
use wasp::parser::Node;
use wasp::parser::Node::*;

#[test]
fn test_node(){
    // let n:Node = Node::new();
    let n:Node = Node::keys("key", "value");
    // let n:Node = KeyValue("key".s(), Box::new(Text("value".s())));
    println!("{:?}", n );

    // let n2:Node = List(vec![Symbol("a".s()), Symbol("b".s()), Symbol("c".s()),n]);
    let n2:Node = List(vec![symbol("a"), Symbol("b".s()), Symbol('c'.s()),n]);
        println!("{:?}", n2);
}

fn symbol(p0: &str) -> Node { Symbol(p0.s()) }