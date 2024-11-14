use wasp::*;
use wasp::node::Node;

#[allow(dead_code)]

#[test]
fn test_indexed(){
    println!("test_indexed own extension indexed for Vec<i32>");
    let v = vec![1, 2, 3, 4, 5];
    for (i, item) in v.indexed() {
        println!("{}: {}", i, item);
            assert_eq!(item, i+1);
    }
}

#[test]
fn test_filter(){
    println!("test_filter own extension filter for Vec<i32>");
    let v = vec![1, 2, 3, 4, 5];
    for i in v.filter(|&x| x > 2) {
        print!("{} ", i);
    }
    let xs = Node::list(v);
    for node in xs.filter(|&x| x > 2) {
        print!("{} ", node);
    }
}
